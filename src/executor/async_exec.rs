//! Asynchronous command execution module for background job control.
//!
//! This module handles the execution of commands in the background using the `&` operator,
//! including proper process group management, stdin redirection, and job table updates.

use std::fs::File;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use crate::parser::{Ast, ShellCommand};
use crate::state::{Job, ShellState};

use super::expansion::{expand_variables_in_args, expand_wildcards};
use super::redirection::apply_redirections;

/// Execute an AST node asynchronously (in the background).
///
/// This function spawns the command in the background with proper process group setup,
/// redirects stdin to /dev/null, creates a job entry in the job table, and prints
/// a job notification. Returns 0 immediately without waiting for the command to complete.
///
/// # Arguments
///
/// * `ast` - The AST node to execute asynchronously
/// * `shell_state` - Mutable reference to the shell state
///
/// # Returns
///
/// Always returns 0 (background jobs don't block)
pub fn execute_async(ast: Ast, shell_state: &mut ShellState) -> i32 {
    match ast {
        Ast::Pipeline(commands) => {
            if commands.is_empty() {
                return 0;
            }

            // For single commands, check if it's a builtin
            if commands.len() == 1 {
                let cmd = &commands[0];

                // Handle compound commands (subshells, etc.)
                if cmd.compound.is_some() {
                    return execute_external_async(&commands, shell_state);
                }

                if cmd.args.is_empty() {
                    return 0;
                }

                // Expand variables and wildcards
                let var_expanded_args = expand_variables_in_args(&cmd.args, shell_state);
                let expanded_args = match expand_wildcards(&var_expanded_args, shell_state) {
                    Ok(args) => args,
                    Err(_) => return 1,
                };

                if expanded_args.is_empty() {
                    return 0;
                }

                // Check if it's a builtin command
                if crate::builtins::is_builtin(&expanded_args[0]) {
                    // Execute builtin in background by forking
                    return execute_builtin_async(cmd, shell_state);
                }
            }

            // External command or pipeline - execute in background
            execute_external_async(&commands, shell_state)
        }
        // Handle other AST types asynchronously by forking
        Ast::Subshell { .. } => execute_compound_async(ast, "subshell", shell_state),
        Ast::CommandGroup { .. } => execute_compound_async(ast, "command group", shell_state),
        Ast::If { .. } => execute_compound_async(ast, "if statement", shell_state),
        Ast::For { .. } => execute_compound_async(ast, "for loop", shell_state),
        Ast::While { .. } => execute_compound_async(ast, "while loop", shell_state),
        Ast::Until { .. } => execute_compound_async(ast, "until loop", shell_state),
        Ast::Case { .. } => execute_compound_async(ast, "case statement", shell_state),
        _ => {
            // For other AST types (assignments, etc.), execute synchronously
            // These don't make sense to run in background
            super::execute(ast, shell_state)
        }
    }
}

/// Execute a compound command (subshell, command group, control structure) asynchronously.
///
/// This function forks a child process to execute the compound command in the background,
/// similar to how builtin commands are executed asynchronously.
///
/// # Arguments
///
/// * `ast` - The AST node to execute asynchronously
/// * `description` - A human-readable description of the command type (for job display)
/// * `shell_state` - Mutable reference to the shell state
///
/// # Returns
///
/// 0 on success, 1 on error
fn execute_compound_async(ast: Ast, description: &str, shell_state: &mut ShellState) -> i32 {
    // Format command string for job display
    let command_str = format!("{} &", description);

    // Fork to execute compound command in background
    unsafe {
        let pid = libc::fork();

        if pid < 0 {
            // Fork failed
            if shell_state.colors_enabled {
                eprintln!(
                    "{}Failed to fork for background {}\x1b[0m",
                    shell_state.color_scheme.error, description
                );
            } else {
                eprintln!("Failed to fork for background {}", description);
            }
            1
        } else if pid == 0 {
            // Child process

            // Create new process group
            let child_pid = libc::getpid();
            libc::setpgid(child_pid, child_pid);

            // Redirect stdin to /dev/null
            let dev_null = libc::open(b"/dev/null\0".as_ptr() as *const i8, libc::O_RDONLY);
            if dev_null >= 0 {
                libc::dup2(dev_null, 0);
                libc::close(dev_null);
            }

            // Execute the compound command
            let exit_code = super::execute(ast, shell_state);

            // Exit the child process
            libc::exit(exit_code);
        } else {
            // Parent process
            let pid_u32 = pid as u32;

            // Allocate job ID and create job entry
            let job_id = shell_state.job_table.borrow_mut().allocate_job_id();
            let job = Job::new(
                job_id,
                Some(pid_u32), // pgid is same as pid for single-process job
                command_str.clone(),
                vec![pid_u32],
                false, // not a builtin (it's a compound command)
            );

            // Print job notification: [job_id] pid
            println!("[{}] {}", job_id, pid_u32);

            // Add job to job table
            shell_state.job_table.borrow_mut().add_job(job);

            // Set $! to the PID of the background process
            shell_state.last_background_pid = Some(pid_u32);

            0
        }
    }
}

/// Execute a builtin command asynchronously by forking a child process.
///
/// Since builtins run in the shell's process, we need to fork to run them
/// in the background. The child process executes the builtin and exits,
/// while the parent creates a job entry and returns immediately.
///
/// # Arguments
///
/// * `cmd` - The shell command to execute (must be a builtin)
/// * `shell_state` - Mutable reference to the shell state
///
/// # Returns
///
/// 0 on success, 1 on error
pub fn execute_builtin_async(cmd: &ShellCommand, shell_state: &mut ShellState) -> i32 {
    // Expand arguments
    let var_expanded_args = expand_variables_in_args(&cmd.args, shell_state);
    let expanded_args = match expand_wildcards(&var_expanded_args, shell_state) {
        Ok(args) => args,
        Err(_) => return 1,
    };

    if expanded_args.is_empty() {
        return 0;
    }

    // Format command string for job display
    let command_str = format_command_string(&expanded_args);

    // Fork to execute builtin in background
    unsafe {
        let pid = libc::fork();

        if pid < 0 {
            // Fork failed
            if shell_state.colors_enabled {
                eprintln!(
                    "{}Failed to fork for background builtin\x1b[0m",
                    shell_state.color_scheme.error
                );
            } else {
                eprintln!("Failed to fork for background builtin");
            }
            1
        } else if pid == 0 {
            // Child process

            // Create new process group
            let child_pid = libc::getpid();
            libc::setpgid(child_pid, child_pid);

            // Redirect stdin to /dev/null
            let dev_null = libc::open(b"/dev/null\0".as_ptr() as *const i8, libc::O_RDONLY);
            if dev_null >= 0 {
                libc::dup2(dev_null, 0);
                libc::close(dev_null);
            }

            // Execute the builtin
            let temp_cmd = ShellCommand {
                args: expanded_args,
                redirections: cmd.redirections.clone(),
                compound: None,
            };

            let exit_code = crate::builtins::execute_builtin(&temp_cmd, shell_state, None);

            // Exit the child process
            libc::exit(exit_code);
        } else {
            // Parent process
            let pid_u32 = pid as u32;

            // Allocate job ID and create job entry
            let job_id = shell_state.job_table.borrow_mut().allocate_job_id();
            let job = Job::new(
                job_id,
                Some(pid_u32), // pgid is same as pid for single-process job
                command_str.clone(),
                vec![pid_u32],
                true, // is_builtin
            );

            // Print job notification: [job_id] pid
            println!("[{}] {}", job_id, pid_u32);

            // Add job to job table
            shell_state.job_table.borrow_mut().add_job(job);

            // Set $! to the PID of the background process
            shell_state.last_background_pid = Some(pid_u32);

            0
        }
    }
}

/// Execute a compound command as part of a background pipeline.
///
/// This function forks a child process to execute the compound command,
/// properly handling pipeline I/O and process group management.
///
/// # Arguments
///
/// * `compound_ast` - The compound command AST to execute
/// * `redirections` - Redirections to apply to the compound command
/// * `stdin` - Optional stdin file from previous pipeline stage
/// * `is_first` - Whether this is the first command in the pipeline
/// * `is_last` - Whether this is the last command in the pipeline
/// * `pgid` - Optional process group ID to join
/// * `shell_state` - Mutable reference to the shell state
///
/// # Returns
///
/// Result containing (PID, optional stdout file for next stage) or error message
fn execute_compound_in_background_pipeline(
    compound_ast: &Ast,
    redirections: &[crate::parser::Redirection],
    stdin: Option<File>,
    is_first: bool,
    is_last: bool,
    pgid: Option<u32>,
    shell_state: &mut ShellState,
) -> Result<(u32, Option<File>), String> {
    use std::os::unix::io::AsRawFd;

    // Create pipe for stdout if not last command
    let (read_fd, write_fd) = if !is_last {
        let (reader, writer) =
            std::io::pipe().map_err(|e| format!("Failed to create pipe: {}", e))?;
        (Some(reader), Some(writer))
    } else {
        (None, None)
    };

    unsafe {
        let pid = libc::fork();

        if pid < 0 {
            return Err("Failed to fork for compound command".to_string());
        }
        if pid == 0 {
            // Child process

            // Set process group
            let child_pid = libc::getpid();
            if let Some(group_id) = pgid {
                libc::setpgid(child_pid, group_id as i32);
            } else {
                libc::setpgid(child_pid, child_pid);
            }

            // Setup stdin
            if let Some(stdin_file) = stdin {
                libc::dup2(stdin_file.as_raw_fd(), 0);
            } else if is_first {
                // First command - redirect stdin to /dev/null
                let dev_null = libc::open(b"/dev/null\0".as_ptr() as *const i8, libc::O_RDONLY);
                if dev_null >= 0 {
                    libc::dup2(dev_null, 0);
                    libc::close(dev_null);
                }
            }

            // Setup stdout
            if let Some(writer) = write_fd {
                libc::dup2(writer.as_raw_fd(), 1);
            }

            // Apply redirections
            if let Err(e) = super::redirection::apply_redirections(redirections, shell_state, None)
            {
                eprintln!("Redirection error: {}", e);
                libc::exit(1);
            }

            // Execute the compound command
            let exit_code = super::execute(compound_ast.clone(), shell_state);

            // Exit the child process
            libc::exit(exit_code);
        }

        let pid_u32 = pid as u32;
        drop(write_fd);

        let next_stdout = if let Some(reader) = read_fd {
            Some(File::from_raw_fd(reader.into_raw_fd()))
        } else {
            None
        };

        Ok((pid_u32, next_stdout))
    }
}

/// Execute external commands or pipelines asynchronously.
///
/// Spawns the command(s) in the background with proper process group setup,
/// redirects stdin to /dev/null, creates job entries, and prints notifications.
///
/// # Arguments
///
/// * `commands` - The pipeline of commands to execute
/// * `shell_state` - Mutable reference to the shell state
///
/// # Returns
///
/// 0 on success, 1 on error
fn execute_external_async(commands: &[ShellCommand], shell_state: &mut ShellState) -> i32 {
    // For pipelines, we need to spawn all commands and track their PIDs
    let mut pids = Vec::new();
    let mut previous_stdout: Option<File> = None;
    let mut pgid: Option<u32> = None;

    // Build command string for job display
    let command_str = format_pipeline_string(commands, shell_state);

    for (i, cmd) in commands.iter().enumerate() {
        let is_last = i == commands.len() - 1;

        // Handle compound commands (subshells, etc.)
        if let Some(ref compound) = cmd.compound {
            // Warn about compound commands in background pipelines
            if shell_state.colors_enabled {
                eprintln!(
                    "{}Warning: Compound command in background pipeline - executing via fork\x1b[0m",
                    shell_state.color_scheme.error
                );
            } else {
                eprintln!("Warning: Compound command in background pipeline - executing via fork");
            }

            // Execute compound command in background by forking
            match execute_compound_in_background_pipeline(
                compound.as_ref(),
                &cmd.redirections,
                previous_stdout.take(),
                i == 0,
                is_last,
                pgid,
                shell_state,
            ) {
                Ok((pid, stdout)) => {
                    pids.push(pid);

                    // Set pgid to first process's PID
                    if pgid.is_none() {
                        pgid = Some(pid);
                    }

                    // Save stdout for next command if not last
                    if !is_last {
                        previous_stdout = stdout;
                    }
                }
                Err(e) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}Error executing compound command in pipeline: {}\x1b[0m",
                            shell_state.color_scheme.error, e
                        );
                    } else {
                        eprintln!("Error executing compound command in pipeline: {}", e);
                    }
                    return 1;
                }
            }
            continue;
        }

        if cmd.args.is_empty() {
            continue;
        }

        // Expand variables and wildcards
        let var_expanded_args = expand_variables_in_args(&cmd.args, shell_state);
        let expanded_args = match expand_wildcards(&var_expanded_args, shell_state) {
            Ok(args) => args,
            Err(_) => return 1,
        };

        if expanded_args.is_empty() {
            continue;
        }

        // Prepare command
        let mut command = Command::new(&expanded_args[0]);
        command.args(&expanded_args[1..]);

        // Set environment for child process
        let child_env = shell_state.get_env_for_child();
        command.env_clear();
        for (key, value) in child_env {
            command.env(key, value);
        }

        // Set stdin
        if let Some(prev) = previous_stdout.take() {
            // Use output from previous command in pipeline
            command.stdin(Stdio::from(prev));
        } else if i == 0 {
            // First command in pipeline - redirect stdin to /dev/null
            command.stdin(Stdio::null());
        } else {
            // Should not happen, but handle gracefully
            command.stdin(Stdio::null());
        }

        // Set stdout for next command in pipeline
        if !is_last {
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

        // Set up process group using pre_exec
        let current_pgid = pgid;
        unsafe {
            command.pre_exec(move || {
                let pid = libc::getpid();

                // Set process group
                if let Some(group_id) = current_pgid {
                    // Join existing process group (for pipeline)
                    libc::setpgid(pid, group_id as i32);
                } else {
                    // Create new process group (first process in pipeline)
                    libc::setpgid(pid, pid);
                }

                Ok(())
            });
        }

        // Spawn the command
        match command.spawn() {
            Ok(mut child) => {
                let pid = child.id();
                pids.push(pid);

                // Set pgid to first process's PID
                if pgid.is_none() {
                    pgid = Some(pid);
                }

                // If not last command, save stdout for next command
                if !is_last {
                    previous_stdout = child
                        .stdout
                        .take()
                        .map(|s| unsafe { File::from_raw_fd(s.into_raw_fd()) });
                }

                // Don't wait for the child - it runs in background
            }
            Err(e) => {
                if shell_state.colors_enabled {
                    eprintln!(
                        "{}Error spawning command '{}': {}\x1b[0m",
                        shell_state.color_scheme.error, expanded_args[0], e
                    );
                } else {
                    eprintln!("Error spawning command '{}': {}", expanded_args[0], e);
                }
                return 1;
            }
        }
    }

    if pids.is_empty() {
        return 0;
    }

    // Allocate job ID and create job entry
    let job_id = shell_state.job_table.borrow_mut().allocate_job_id();
    let job = Job::new(
        job_id,
        pgid,
        command_str,
        pids.clone(),
        false, // not a builtin
    );

    // Print job notification: [job_id] pid
    // For pipelines, print the first PID (process group leader)
    println!("[{}] {}", job_id, pids[0]);

    // Add job to job table
    shell_state.job_table.borrow_mut().add_job(job);

    // Set $! to the PID of the last process in the pipeline (or the only process)
    // This matches POSIX behavior where $! is the PID of the last process in a background pipeline
    if let Some(&last_pid) = pids.last() {
        shell_state.last_background_pid = Some(last_pid);
    }

    0
}

/// Format a command's arguments into a display string.
///
/// # Arguments
///
/// * `args` - The command arguments
///
/// # Returns
///
/// A formatted command string
fn format_command_string(args: &[String]) -> String {
    args.join(" ")
}

/// Format a pipeline of commands into a display string.
///
/// # Arguments
///
/// * `commands` - The pipeline of commands
/// * `shell_state` - Reference to shell state for variable expansion
///
/// # Returns
///
/// A formatted pipeline string
fn format_pipeline_string(commands: &[ShellCommand], shell_state: &mut ShellState) -> String {
    let mut parts = Vec::new();

    for cmd in commands {
        if let Some(ref compound) = cmd.compound {
            // For compound commands, use a simplified representation
            match compound.as_ref() {
                Ast::Subshell { .. } => parts.push("(...)".to_string()),
                Ast::CommandGroup { .. } => parts.push("{...}".to_string()),
                _ => parts.push("compound".to_string()),
            }
        } else if !cmd.args.is_empty() {
            // Expand variables for display
            let var_expanded_args = expand_variables_in_args(&cmd.args, shell_state);
            parts.push(var_expanded_args.join(" "));
        }
    }

    parts.join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_command_string() {
        let args = vec!["echo".to_string(), "hello".to_string(), "world".to_string()];
        assert_eq!(format_command_string(&args), "echo hello world");
    }

    #[test]
    fn test_format_pipeline_string() {
        let mut shell_state = ShellState::new();

        let commands = vec![
            ShellCommand {
                args: vec!["ls".to_string(), "-la".to_string()],
                redirections: vec![],
                compound: None,
            },
            ShellCommand {
                args: vec!["grep".to_string(), "txt".to_string()],
                redirections: vec![],
                compound: None,
            },
        ];

        assert_eq!(
            format_pipeline_string(&commands, &mut shell_state),
            "ls -la | grep txt"
        );
    }

    #[test]
    fn test_format_pipeline_with_subshell() {
        let mut shell_state = ShellState::new();

        let commands = vec![
            ShellCommand {
                args: vec![],
                redirections: vec![],
                compound: Some(Box::new(Ast::Subshell {
                    body: Box::new(Ast::Pipeline(vec![])),
                })),
            },
            ShellCommand {
                args: vec!["grep".to_string(), "txt".to_string()],
                redirections: vec![],
                compound: None,
            },
        ];

        assert_eq!(
            format_pipeline_string(&commands, &mut shell_state),
            "(...) | grep txt"
        );
    }
}
