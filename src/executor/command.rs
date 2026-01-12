//! Command execution module for the Rush shell.
//!
//! This module handles the execution of individual commands and pipelines.

use std::cell::RefCell;
use std::fs::File;
use std::io::pipe;
use std::os::fd::{FromRawFd, IntoRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::rc::Rc;

use crate::parser::{Ast, ShellCommand};
use crate::state::ShellState;

use super::expansion::{expand_variables_in_args, expand_wildcards};
use super::redirection::apply_redirections;
use super::{execute, execute_compound_in_pipeline, execute_compound_with_redirections};

/// Execute the given AST and return its standard output (as produced to stdout) with trailing newlines removed.
///
/// The function runs the AST in the provided shell state and captures whatever would be written to stdout
/// (including results from pipelines, builtins, functions, subshells, and external commands). If the executed
/// AST exits with a non-zero status or fails to spawn/execute, an `Err(String)` describing the failure is returned.
///
/// # Examples
///
/// ```
/// // Note: execute_and_capture_output is a crate-internal function
/// // This example is for documentation only
/// ```
pub(crate) fn execute_and_capture_output(ast: Ast, shell_state: &mut ShellState) -> Result<String, String> {
    // Create a pipe to capture stdout
    let (reader, writer) = pipe().map_err(|e| format!("Failed to create pipe: {}", e))?;

    // We need to capture the output, so we'll redirect stdout to our pipe
    // For builtins, we can pass the writer directly
    // For external commands, we need to handle them specially

    match &ast {
        Ast::Pipeline(commands) => {
            // Handle both single commands and multi-command pipelines
            if commands.is_empty() {
                return Ok(String::new());
            }

            if commands.len() == 1 {
                // Single command - use the existing optimized path
                let cmd = &commands[0];
                if cmd.args.is_empty() {
                    return Ok(String::new());
                }

                // Expand variables and wildcards
                let var_expanded_args = expand_variables_in_args(&cmd.args, shell_state);
                let expanded_args = expand_wildcards(&var_expanded_args, shell_state)
                    .map_err(|e| format!("Wildcard expansion failed: {}", e))?;

                if expanded_args.is_empty() {
                    return Ok(String::new());
                }

                // Check if it's a function call
                if shell_state.get_function(&expanded_args[0]).is_some() {
                    // Save previous capture state (for nested command substitutions)
                    let previous_capture = shell_state.capture_output.clone();

                    // Enable output capture mode
                    let capture_buffer = Rc::new(RefCell::new(Vec::new()));
                    shell_state.capture_output = Some(capture_buffer.clone());

                    // Create a FunctionCall AST and execute it
                    let function_call_ast = Ast::FunctionCall {
                        name: expanded_args[0].clone(),
                        args: expanded_args[1..].to_vec(),
                    };

                    let exit_code = execute(function_call_ast, shell_state);

                    // Retrieve captured output
                    let captured = capture_buffer.borrow().clone();
                    let output = String::from_utf8_lossy(&captured).trim_end().to_string();

                    // Restore previous capture state
                    shell_state.capture_output = previous_capture;

                    if exit_code == 0 {
                        Ok(output)
                    } else {
                        Err(format!("Function failed with exit code {}", exit_code))
                    }
                } else if crate::builtins::is_builtin(&expanded_args[0]) {
                    let temp_cmd = ShellCommand {
                        args: expanded_args,
                        redirections: cmd.redirections.clone(),
                        compound: None,
                    };

                    // Execute builtin with our writer
                    let exit_code = crate::builtins::execute_builtin(
                        &temp_cmd,
                        shell_state,
                        Some(Box::new(writer)),
                    );

                    // Read the captured output
                    drop(temp_cmd); // Ensure writer is dropped
                    let mut output = String::new();
                    use std::io::Read;
                    let mut reader = reader;
                    reader
                        .read_to_string(&mut output)
                        .map_err(|e| format!("Failed to read output: {}", e))?;

                    if exit_code == 0 {
                        Ok(output.trim_end().to_string())
                    } else {
                        Err(format!("Command failed with exit code {}", exit_code))
                    }
                } else {
                    // External command - execute with output capture
                    drop(writer); // Close writer end before spawning

                    let mut command = Command::new(&expanded_args[0]);
                    command.args(&expanded_args[1..]);
                    command.stdout(Stdio::piped());
                    command.stderr(Stdio::null()); // Suppress stderr for command substitution

                    // Set environment
                    let child_env = shell_state.get_env_for_child();
                    command.env_clear();
                    for (key, value) in child_env {
                        command.env(key, value);
                    }

                    let output = command
                        .output()
                        .map_err(|e| format!("Failed to execute command: {}", e))?;

                    if output.status.success() {
                        Ok(String::from_utf8_lossy(&output.stdout)
                            .trim_end()
                            .to_string())
                    } else {
                        Err(format!(
                            "Command failed with exit code {}",
                            output.status.code().unwrap_or(1)
                        ))
                    }
                }
            } else {
                // Multi-command pipeline - execute the entire pipeline and capture output
                drop(writer); // Close writer end before executing pipeline

                // Save previous capture state (for nested command substitutions)
                let previous_capture = shell_state.capture_output.clone();

                // Enable output capture mode
                let capture_buffer = Rc::new(RefCell::new(Vec::new()));
                shell_state.capture_output = Some(capture_buffer.clone());

                // Execute the pipeline
                let exit_code = execute_pipeline(commands, shell_state);

                // Retrieve captured output
                let captured = capture_buffer.borrow().clone();
                let output = String::from_utf8_lossy(&captured).trim_end().to_string();

                // Restore previous capture state
                shell_state.capture_output = previous_capture;

                if exit_code == 0 {
                    Ok(output)
                } else {
                    Err(format!("Pipeline failed with exit code {}", exit_code))
                }
            }
        }
        _ => {
            // For other AST nodes (sequences, etc.), we need special handling
            drop(writer);

            // Save previous capture state
            let previous_capture = shell_state.capture_output.clone();

            // Enable output capture mode
            let capture_buffer = Rc::new(RefCell::new(Vec::new()));
            shell_state.capture_output = Some(capture_buffer.clone());

            // Execute the AST
            let exit_code = execute(ast, shell_state);

            // Retrieve captured output
            let captured = capture_buffer.borrow().clone();
            let output = String::from_utf8_lossy(&captured).trim_end().to_string();

            // Restore previous capture state
            shell_state.capture_output = previous_capture;

            if exit_code == 0 {
                Ok(output)
            } else {
                Err(format!("Command failed with exit code {}", exit_code))
            }
        }
    }
}

/// ```
/// // Note: execute_single_command is a private function
/// // This example is for documentation only
/// ```
pub(crate) fn execute_single_command(cmd: &ShellCommand, shell_state: &mut ShellState) -> i32 {
    // Check if this is a compound command (subshell)
    if let Some(ref compound_ast) = cmd.compound {
        // Check noexec option (-n) for compound commands
        // Exception: The 'set' builtin must always execute to allow disabling noexec
        if shell_state.options.noexec {
            return 0; // Return success without executing
        }
        // Execute compound command with redirections
        return execute_compound_with_redirections(compound_ast, shell_state, &cmd.redirections);
    }

    // Check noexec option (-n): Read commands but don't execute them
    // Exception: The 'set' builtin must always execute to allow disabling noexec
    // IMPORTANT: Check this BEFORE processing redirections to prevent side effects
    let is_set_builtin = !cmd.args.is_empty() && cmd.args[0] == "set";
    
    if shell_state.options.noexec && !is_set_builtin {
        return 0; // Return success without executing (no side effects)
    }

    if cmd.args.is_empty() {
        // No command, but may have redirections - process them for side effects
        if !cmd.redirections.is_empty() {
            if let Err(e) = apply_redirections(&cmd.redirections, shell_state, None) {
                if shell_state.colors_enabled {
                    eprintln!(
                        "{}Redirection error: {}\x1b[0m",
                        shell_state.color_scheme.error, e
                    );
                } else {
                    eprintln!("Redirection error: {}", e);
                }
                return 1;
            }
        }
        return 0;
    }

    // First expand variables, then wildcards
    let var_expanded_args = expand_variables_in_args(&cmd.args, shell_state);
    let expanded_args = match expand_wildcards(&var_expanded_args, shell_state) {
        Ok(args) => args,
        Err(_) => return 1,
    };

    if expanded_args.is_empty() {
        return 0;
    }

    // Print command if xtrace is enabled (-x)
    if shell_state.options.xtrace {
        // Get PS4 prompt (default: "+ ")
        let ps4 = shell_state.get_var("PS4").unwrap_or_else(|| "+ ".to_string());
        
        // Print the command with expanded arguments to stderr
        let command_str = expanded_args.join(" ");
        if shell_state.colors_enabled {
            eprintln!(
                "{}{}{}\x1b[0m",
                shell_state.color_scheme.builtin,
                ps4,
                command_str
            );
        } else {
            eprintln!("{}{}", ps4, command_str);
        }
    }

    // Check if this is a function call
    if shell_state.get_function(&expanded_args[0]).is_some() {
        // This is a function call - create a FunctionCall AST node and execute it
        let function_call = Ast::FunctionCall {
            name: expanded_args[0].clone(),
            args: expanded_args[1..].to_vec(),
        };
        return execute(function_call, shell_state);
    }

    if crate::builtins::is_builtin(&expanded_args[0]) {
        // Create a temporary ShellCommand with expanded args
        let temp_cmd = ShellCommand {
            args: expanded_args,
            redirections: cmd.redirections.clone(),
            compound: None,
        };

        // If we're capturing output, create a writer for it
        let exit_code = if let Some(ref capture_buffer) = shell_state.capture_output.clone() {
            // Create a writer that writes to our capture buffer
            struct CaptureWriter {
                buffer: Rc<RefCell<Vec<u8>>>,
            }
            impl std::io::Write for CaptureWriter {
                fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                    self.buffer.borrow_mut().extend_from_slice(buf);
                    Ok(buf.len())
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    Ok(())
                }
            }
            let writer = CaptureWriter {
                buffer: capture_buffer.clone(),
            };
            crate::builtins::execute_builtin(&temp_cmd, shell_state, Some(Box::new(writer)))
        } else {
            crate::builtins::execute_builtin(&temp_cmd, shell_state, None)
        };

        // Check errexit option (-e): Exit immediately if command fails
        // POSIX: Don't exit in these contexts:
        // 1. Inside if/while/until condition (tracked by in_condition flag)
        // 2. Part of && or || chain (tracked by in_logical_chain flag)
        // 3. Pipeline (except last command) - handled by pipeline executor
        // 4. Negated command (tracked by in_negation flag)
        if shell_state.options.errexit
            && exit_code != 0
            && !shell_state.in_condition
            && !shell_state.in_logical_chain
            && !shell_state.in_negation {
            // Set exit_requested flag to trigger shell exit
            shell_state.exit_requested = true;
            shell_state.exit_code = exit_code;
        }

        exit_code
    } else {
        // Separate environment variable assignments from the actual command
        // Environment vars must come before the command and have the form VAR=value
        let mut env_assignments = Vec::new();
        let mut command_start_idx = 0;

        for (idx, arg) in expanded_args.iter().enumerate() {
            // Check if this looks like an environment variable assignment
            if let Some(eq_pos) = arg.find('=')
                && eq_pos > 0
            {
                let var_part = &arg[..eq_pos];
                // Check if var_part is a valid variable name
                if var_part
                    .chars()
                    .next()
                    .map(|c| c.is_alphabetic() || c == '_')
                    .unwrap_or(false)
                    && var_part.chars().all(|c| c.is_alphanumeric() || c == '_')
                {
                    env_assignments.push(arg.clone());
                    command_start_idx = idx + 1;
                    continue;
                }
            }
            // If we reach here, this is not an env assignment, so we've found the command
            break;
        }

        // Check if we have a command to execute (vs just env assignments)
        let has_command = command_start_idx < expanded_args.len();

        // If all args were env assignments, set them in the shell
        // but continue to process redirections per POSIX
        if !has_command {
            for assignment in &env_assignments {
                if let Some(eq_pos) = assignment.find('=') {
                    let var_name = &assignment[..eq_pos];
                    let var_value = &assignment[eq_pos + 1..];
                    shell_state.set_var(var_name, var_value.to_string());
                    
                    // Auto-export if allexport option (-a) is enabled
                    if shell_state.options.allexport {
                        shell_state.export_var(var_name);
                    }
                }
            }

            // Process redirections even without a command
            if !cmd.redirections.is_empty() {
                if let Err(e) = apply_redirections(&cmd.redirections, shell_state, None) {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}Redirection error: {}\x1b[0m",
                            shell_state.color_scheme.error, e
                        );
                    } else {
                        eprintln!("Redirection error: {}", e);
                    }
                    return 1;
                }
            }
            return 0;
        }

        // Prepare command
        let mut command = Command::new(&expanded_args[command_start_idx]);
        command.args(&expanded_args[command_start_idx + 1..]);

        // Check for stdin override (for pipeline subshells)
        if let Some(fd) = shell_state.stdin_override {
            unsafe {
                let dup_fd = libc::dup(fd);
                if dup_fd >= 0 {
                    command.stdin(Stdio::from_raw_fd(dup_fd));
                }
            }
        }

        // Set environment for child process
        let mut child_env = shell_state.get_env_for_child();

        // Add the per-command environment variable assignments
        for assignment in env_assignments {
            if let Some(eq_pos) = assignment.find('=') {
                let var_name = assignment[..eq_pos].to_string();
                let var_value = assignment[eq_pos + 1..].to_string();
                child_env.insert(var_name, var_value);
            }
        }

        command.env_clear();
        for (key, value) in child_env {
            command.env(key, value);
        }

        // If we're capturing output, redirect stdout to capture buffer
        let capturing = shell_state.capture_output.is_some();
        if capturing {
            command.stdout(Stdio::piped());
        }

        // Apply all redirections
        if let Err(e) = apply_redirections(&cmd.redirections, shell_state, Some(&mut command)) {
            if shell_state.colors_enabled {
                eprintln!(
                    "{}Redirection error: {}\x1b[0m",
                    shell_state.color_scheme.error, e
                );
            } else {
                eprintln!("Redirection error: {}", e);
            }
            return 1;
        }

        // Apply custom file descriptors (3-9) from fd table to external command
        // We need to keep the FD table borrowed until after the child is spawned
        // to prevent File handles from being dropped and FDs from being closed
        let custom_fds: Vec<(i32, RawFd)> = {
            let fd_table = shell_state.fd_table.borrow();
            let mut fds = Vec::new();

            for fd_num in 3..=9 {
                if fd_table.is_open(fd_num) {
                    if let Some(raw_fd) = fd_table.get_raw_fd(fd_num) {
                        fds.push((fd_num, raw_fd));
                    }
                }
            }

            fds
        };

        // If we have custom fds to apply, use pre_exec to set them in the child
        if !custom_fds.is_empty() {
            unsafe {
                command.pre_exec(move || {
                    for (target_fd, source_fd) in &custom_fds {
                        let result = libc::dup2(*source_fd, *target_fd);
                        if result < 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                    }
                    Ok(())
                });
            }
        }

        // Spawn and execute the command
        // Note: The FD table borrow above has been released, but the custom_fds
        // closure capture keeps the file handles alive
        match command.spawn() {
            Ok(mut child) => {
                // If capturing, read stdout
                if capturing {
                    if let Some(mut stdout) = child.stdout.take() {
                        use std::io::Read;
                        let mut output = Vec::new();
                        if stdout.read_to_end(&mut output).is_ok() {
                            if let Some(ref capture_buffer) = shell_state.capture_output {
                                capture_buffer.borrow_mut().extend_from_slice(&output);
                            }
                        }
                    }
                }

                let exit_code = match child.wait() {
                    Ok(status) => status.code().unwrap_or(0),
                    Err(e) => {
                        if shell_state.colors_enabled {
                            eprintln!(
                                "{}Error waiting for command: {}\x1b[0m",
                                shell_state.color_scheme.error, e
                            );
                        } else {
                            eprintln!("Error waiting for command: {}", e);
                        }
                        1
                    }
                };

                // Check errexit option (-e): Exit immediately if command fails
                // POSIX: Don't exit in these contexts:
                // 1. Inside if/while/until condition (tracked by in_condition flag)
                // 2. Part of && or || chain (tracked by in_logical_chain flag)
                // 3. Pipeline (except last command) - handled by pipeline executor
                // 4. Negated command (tracked by in_negation flag)
                if shell_state.options.errexit
                    && exit_code != 0
                    && !shell_state.in_condition
                    && !shell_state.in_logical_chain
                    && !shell_state.in_negation {
                    // Set exit_requested flag to trigger shell exit
                    shell_state.exit_requested = true;
                    shell_state.exit_code = exit_code;
                }

                exit_code
            }
            Err(e) => {
                if shell_state.colors_enabled {
                    eprintln!(
                        "{}Command spawn error: {}\x1b[0m",
                        shell_state.color_scheme.error, e
                    );
                } else {
                    eprintln!("Command spawn error: {}", e);
                }
                1
            }
        }
    }
}

/// ```
/// // Note: execute_pipeline is a private function
/// // This example is for documentation only
/// ```
pub(crate) fn execute_pipeline(commands: &[ShellCommand], shell_state: &mut ShellState) -> i32 {
    // Check noexec option (-n): Read commands but don't execute them
    // Exception: The 'set' builtin must always execute to allow disabling noexec
    // For pipelines, check if any command is 'set', otherwise skip execution
    let has_set_builtin = commands.iter().any(|cmd| {
        !cmd.args.is_empty() && cmd.args[0] == "set"
    });
    
    if shell_state.options.noexec && !has_set_builtin {
        return 0; // Return success without executing (no side effects)
    }

    let mut exit_code = 0;
    let mut previous_stdout: Option<File> = None;

    for (i, cmd) in commands.iter().enumerate() {
        let is_last = i == commands.len() - 1;

        if let Some(ref compound_ast) = cmd.compound {
            // Execute compound command (subshell) in pipeline
            let (com_exit_code, com_stdout) = execute_compound_in_pipeline(
                compound_ast,
                shell_state,
                previous_stdout.take(),
                i == 0,
                is_last,
                &cmd.redirections,
            );
            exit_code = com_exit_code;
            previous_stdout = com_stdout;
            continue;
        }

        if cmd.args.is_empty() {
            continue;
        }

        // First expand variables, then wildcards
        let var_expanded_args = expand_variables_in_args(&cmd.args, shell_state);
        let expanded_args = match expand_wildcards(&var_expanded_args, shell_state) {
            Ok(args) => args,
            Err(_) => return 1,
        };

        if expanded_args.is_empty() {
            continue;
        }

        if crate::builtins::is_builtin(&expanded_args[0]) {
            // Built-ins in pipelines are tricky - for now, execute them separately
            // This is not perfect but better than nothing
            let temp_cmd = ShellCommand {
                args: expanded_args,
                redirections: cmd.redirections.clone(),
                compound: None,
            };
            if !is_last {
                // Create a safe pipe
                let (reader, writer) = match pipe() {
                    Ok((r, w)) => (unsafe { File::from_raw_fd(r.into_raw_fd()) }, w),
                    Err(e) => {
                        if shell_state.colors_enabled {
                            eprintln!(
                                "{}Error creating pipe for builtin: {}\x1b[0m",
                                shell_state.color_scheme.error, e
                            );
                        } else {
                            eprintln!("Error creating pipe for builtin: {}", e);
                        }
                        return 1;
                    }
                };
                // Execute builtin with writer for output capture
                exit_code = crate::builtins::execute_builtin(
                    &temp_cmd,
                    shell_state,
                    Some(Box::new(writer)),
                );
                // Use reader for next command's stdin
                previous_stdout = Some(reader);
            } else {
                // Last command: check if we're capturing output
                if let Some(ref capture_buffer) = shell_state.capture_output.clone() {
                    // Create a writer that writes to our capture buffer
                    struct CaptureWriter {
                        buffer: Rc<RefCell<Vec<u8>>>,
                    }
                    impl std::io::Write for CaptureWriter {
                        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                            self.buffer.borrow_mut().extend_from_slice(buf);
                            Ok(buf.len())
                        }
                        fn flush(&mut self) -> std::io::Result<()> {
                            Ok(())
                        }
                    }
                    let writer = CaptureWriter {
                        buffer: capture_buffer.clone(),
                    };
                    exit_code = crate::builtins::execute_builtin(
                        &temp_cmd,
                        shell_state,
                        Some(Box::new(writer)),
                    );
                } else {
                    // Not capturing, execute normally
                    exit_code = crate::builtins::execute_builtin(&temp_cmd, shell_state, None);
                }
                previous_stdout = None;
            }
        } else {
            let mut command = Command::new(&expanded_args[0]);
            command.args(&expanded_args[1..]);

            // Set environment for child process
            let child_env = shell_state.get_env_for_child();
            command.env_clear();
            for (key, value) in child_env {
                command.env(key, value);
            }

            // Set stdin from previous command's stdout
            if let Some(prev) = previous_stdout.take() {
                command.stdin(Stdio::from(prev));
            } else if i > 0 {
                // We are in a pipeline (not first command) but have no input pipe.
                // This means the previous command didn't produce a pipe.
                // We should treat this as empty input (EOF), not inherit stdin!
                command.stdin(Stdio::null());
            } else if let Some(fd) = shell_state.stdin_override {
                // We have a stdin override (e.g. from parent subshell)
                // We must duplicate it because Stdio takes ownership
                unsafe {
                    let dup_fd = libc::dup(fd);
                    if dup_fd >= 0 {
                        command.stdin(Stdio::from_raw_fd(dup_fd));
                    }
                }
            }

            // Set stdout for next command, or for capturing if this is the last
            if !is_last {
                command.stdout(Stdio::piped());
            } else if shell_state.capture_output.is_some() {
                // Last command in pipeline but we're capturing output
                command.stdout(Stdio::piped());
            }

            // Apply redirections for this command
            if let Err(e) = apply_redirections(&cmd.redirections, shell_state, Some(&mut command)) {
                if shell_state.colors_enabled {
                    eprintln!(
                        "{}Redirection error: {}\x1b[0m",
                        shell_state.color_scheme.error, e
                    );
                } else {
                    eprintln!("Redirection error: {}", e);
                }
                return 1;
            }

            match command.spawn() {
                Ok(mut child) => {
                    if !is_last {
                        previous_stdout = child
                            .stdout
                            .take()
                            .map(|s| unsafe { File::from_raw_fd(s.into_raw_fd()) });
                    } else if shell_state.capture_output.is_some() {
                        // Last command and we're capturing - read its output
                        if let Some(mut stdout) = child.stdout.take() {
                            use std::io::Read;
                            let mut output = Vec::new();
                            if stdout.read_to_end(&mut output).is_ok()
                                && let Some(ref capture_buffer) = shell_state.capture_output
                            {
                                capture_buffer.borrow_mut().extend_from_slice(&output);
                            }
                        }
                    }
                    match child.wait() {
                        Ok(status) => {
                            exit_code = status.code().unwrap_or(0);
                        }
                        Err(e) => {
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}Error waiting for command: {}\x1b[0m",
                                    shell_state.color_scheme.error, e
                                );
                            } else {
                                eprintln!("Error waiting for command: {}", e);
                            }
                            exit_code = 1;
                        }
                    }
                }
                Err(e) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}Error spawning command '{}{}",
                            shell_state.color_scheme.error,
                            expanded_args[0],
                            &format!("': {}\x1b[0m", e)
                        );
                    } else {
                        eprintln!("Error spawning command '{}': {}", expanded_args[0], e);
                    }
                    exit_code = 1;
                }
            }
        }
    }

    exit_code
}
