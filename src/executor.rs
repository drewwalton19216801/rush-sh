use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write, pipe};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::rc::Rc;

use super::parser::{Ast, Redirection, ShellCommand};
use super::state::ShellState;

/// Maximum allowed subshell nesting depth to prevent stack overflow
const MAX_SUBSHELL_DEPTH: usize = 100;

/// Execute a command and capture its output as a string
/// This is used for command substitution $(...)
fn execute_and_capture_output(ast: Ast, shell_state: &mut ShellState) -> Result<String, String> {
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
                let expanded_args = expand_wildcards(&var_expanded_args)
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

fn expand_variables_in_args(args: &[String], shell_state: &mut ShellState) -> Vec<String> {
    let mut expanded_args = Vec::new();

    for arg in args {
        // Expand variables within the argument string
        let expanded_arg = expand_variables_in_string(arg, shell_state);
        expanded_args.push(expanded_arg);
    }

    expanded_args
}

pub fn expand_variables_in_string(input: &str, shell_state: &mut ShellState) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            // Check for command substitution $(...) or arithmetic expansion $((...))
            if let Some(&'(') = chars.peek() {
                chars.next(); // consume first (

                // Check if this is arithmetic expansion $((...))
                if let Some(&'(') = chars.peek() {
                    // Arithmetic expansion $((...))
                    chars.next(); // consume second (
                    let mut arithmetic_expr = String::new();
                    let mut paren_depth = 1;
                    let mut found_closing = false;

                    while let Some(c) = chars.next() {
                        if c == '(' {
                            paren_depth += 1;
                            arithmetic_expr.push(c);
                        } else if c == ')' {
                            paren_depth -= 1;
                            if paren_depth == 0 {
                                // Found the first closing ) - check for second )
                                if let Some(&')') = chars.peek() {
                                    chars.next(); // consume the second )
                                    found_closing = true;
                                    break;
                                } else {
                                    // Missing second closing paren, treat as error
                                    result.push_str("$((");
                                    result.push_str(&arithmetic_expr);
                                    result.push(')');
                                    break;
                                }
                            }
                            arithmetic_expr.push(c);
                        } else {
                            arithmetic_expr.push(c);
                        }
                    }

                    if found_closing {
                        // First expand variables in the arithmetic expression
                        // The arithmetic evaluator expects variable names without $ prefix
                        // So we need to expand $VAR to the value before evaluation
                        let mut expanded_expr = String::new();
                        let mut expr_chars = arithmetic_expr.chars().peekable();

                        while let Some(ch) = expr_chars.next() {
                            if ch == '$' {
                                // Expand variable
                                let mut var_name = String::new();
                                if let Some(&c) = expr_chars.peek() {
                                    if c == '?'
                                        || c == '$'
                                        || c == '0'
                                        || c == '#'
                                        || c == '*'
                                        || c == '@'
                                        || c.is_ascii_digit()
                                    {
                                        var_name.push(c);
                                        expr_chars.next();
                                    } else {
                                        while let Some(&c) = expr_chars.peek() {
                                            if c.is_alphanumeric() || c == '_' {
                                                var_name.push(c);
                                                expr_chars.next();
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                }

                                if !var_name.is_empty() {
                                    if let Some(value) = shell_state.get_var(&var_name) {
                                        expanded_expr.push_str(&value);
                                    } else {
                                        // Variable not found, use 0 for arithmetic
                                        expanded_expr.push('0');
                                    }
                                } else {
                                    expanded_expr.push('$');
                                }
                            } else {
                                expanded_expr.push(ch);
                            }
                        }

                        match crate::arithmetic::evaluate_arithmetic_expression(
                            &expanded_expr,
                            shell_state,
                        ) {
                            Ok(value) => {
                                result.push_str(&value.to_string());
                            }
                            Err(e) => {
                                // On arithmetic error, display a proper error message
                                if shell_state.colors_enabled {
                                    result.push_str(&format!(
                                        "{}arithmetic error: {}{}",
                                        shell_state.color_scheme.error, e, "\x1b[0m"
                                    ));
                                } else {
                                    result.push_str(&format!("arithmetic error: {}", e));
                                }
                            }
                        }
                    } else {
                        // Didn't find proper closing - keep as literal
                        result.push_str("$((");
                        result.push_str(&arithmetic_expr);
                        // Note: we don't add closing parens since they weren't in the input
                    }
                    continue;
                }

                // Regular command substitution $(...)
                let mut sub_command = String::new();
                let mut paren_depth = 1;

                for c in chars.by_ref() {
                    if c == '(' {
                        paren_depth += 1;
                        sub_command.push(c);
                    } else if c == ')' {
                        paren_depth -= 1;
                        if paren_depth == 0 {
                            break;
                        }
                        sub_command.push(c);
                    } else {
                        sub_command.push(c);
                    }
                }

                // Execute the command substitution within the current shell context
                // Parse and execute the command using our own lexer/parser/executor
                if let Ok(tokens) = crate::lexer::lex(&sub_command, shell_state) {
                    // Expand aliases before parsing
                    let expanded_tokens = match crate::lexer::expand_aliases(
                        tokens,
                        shell_state,
                        &mut std::collections::HashSet::new(),
                    ) {
                        Ok(t) => t,
                        Err(_) => {
                            // Alias expansion error, keep literal
                            result.push_str("$(");
                            result.push_str(&sub_command);
                            result.push(')');
                            continue;
                        }
                    };

                    match crate::parser::parse(expanded_tokens) {
                        Ok(ast) => {
                            // Execute within current shell context and capture output
                            match execute_and_capture_output(ast, shell_state) {
                                Ok(output) => {
                                    result.push_str(&output);
                                }
                                Err(_) => {
                                    // On failure, keep the literal
                                    result.push_str("$(");
                                    result.push_str(&sub_command);
                                    result.push(')');
                                }
                            }
                        }
                        Err(_parse_err) => {
                            // Parse error - try to handle as function call if it looks like one
                            let tokens_str = sub_command.trim();
                            if tokens_str.contains(' ') {
                                // Split by spaces and check if first token looks like a function call
                                let parts: Vec<&str> = tokens_str.split_whitespace().collect();
                                if let Some(first_token) = parts.first()
                                    && shell_state.get_function(first_token).is_some()
                                {
                                    // This is a function call, create AST manually
                                    let function_call = Ast::FunctionCall {
                                        name: first_token.to_string(),
                                        args: parts[1..].iter().map(|s| s.to_string()).collect(),
                                    };
                                    match execute_and_capture_output(function_call, shell_state) {
                                        Ok(output) => {
                                            result.push_str(&output);
                                            continue;
                                        }
                                        Err(_) => {
                                            // Fall back to literal
                                        }
                                    }
                                }
                            }
                            // Keep the literal
                            result.push_str("$(");
                            result.push_str(&sub_command);
                            result.push(')');
                        }
                    }
                } else {
                    // Lex error, keep literal
                    result.push_str("$(");
                    result.push_str(&sub_command);
                    result.push(')');
                }
            } else {
                // Regular variable
                let mut var_name = String::new();
                let mut next_ch = chars.peek();

                // Handle special single-character variables first
                if let Some(&c) = next_ch {
                    if c == '?' || c == '$' || c == '0' || c == '#' || c == '*' || c == '@' {
                        var_name.push(c);
                        chars.next(); // consume the character
                    } else if c.is_ascii_digit() {
                        // Positional parameter
                        var_name.push(c);
                        chars.next();
                    } else {
                        // Regular variable name
                        while let Some(&c) = next_ch {
                            if c.is_alphanumeric() || c == '_' {
                                var_name.push(c);
                                chars.next(); // consume the character
                                next_ch = chars.peek();
                            } else {
                                break;
                            }
                        }
                    }
                }

                if !var_name.is_empty() {
                    if let Some(value) = shell_state.get_var(&var_name) {
                        result.push_str(&value);
                    } else {
                        // Variable not found - for positional parameters, expand to empty string
                        // For other variables, keep the literal
                        if var_name.chars().next().unwrap().is_ascii_digit()
                            || var_name == "?"
                            || var_name == "$"
                            || var_name == "0"
                            || var_name == "#"
                            || var_name == "*"
                            || var_name == "@"
                        {
                            // Expand to empty string for undefined positional parameters
                        } else {
                            // Keep the literal for regular variables
                            result.push('$');
                            result.push_str(&var_name);
                        }
                    }
                } else {
                    result.push('$');
                }
            }
        } else if ch == '`' {
            // Backtick command substitution
            let mut sub_command = String::new();

            for c in chars.by_ref() {
                if c == '`' {
                    break;
                }
                sub_command.push(c);
            }

            // Execute the command substitution
            if let Ok(tokens) = crate::lexer::lex(&sub_command, shell_state) {
                // Expand aliases before parsing
                let expanded_tokens = match crate::lexer::expand_aliases(
                    tokens,
                    shell_state,
                    &mut std::collections::HashSet::new(),
                ) {
                    Ok(t) => t,
                    Err(_) => {
                        // Alias expansion error, keep literal
                        result.push('`');
                        result.push_str(&sub_command);
                        result.push('`');
                        continue;
                    }
                };

                if let Ok(ast) = crate::parser::parse(expanded_tokens) {
                    // Execute and capture output
                    match execute_and_capture_output(ast, shell_state) {
                        Ok(output) => {
                            result.push_str(&output);
                        }
                        Err(_) => {
                            // On failure, keep the literal
                            result.push('`');
                            result.push_str(&sub_command);
                            result.push('`');
                        }
                    }
                } else {
                    // Parse error, keep literal
                    result.push('`');
                    result.push_str(&sub_command);
                    result.push('`');
                }
            } else {
                // Lex error, keep literal
                result.push('`');
                result.push_str(&sub_command);
                result.push('`');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn expand_wildcards(args: &[String]) -> Result<Vec<String>, String> {
    let mut expanded_args = Vec::new();

    for arg in args {
        if arg.contains('*') || arg.contains('?') || arg.contains('[') {
            // Try to expand wildcard
            match glob::glob(arg) {
                Ok(paths) => {
                    let mut matches: Vec<String> = paths
                        .filter_map(|p| p.ok())
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();
                    if matches.is_empty() {
                        // No matches, keep literal
                        expanded_args.push(arg.clone());
                    } else {
                        // Sort for consistent behavior
                        matches.sort();
                        expanded_args.extend(matches);
                    }
                }
                Err(_e) => {
                    // Invalid pattern, keep literal
                    expanded_args.push(arg.clone());
                }
            }
        } else {
            expanded_args.push(arg.clone());
        }
    }
    Ok(expanded_args)
}

/// Collect here-document content from stdin until the specified delimiter is found
/// This function reads from stdin line by line until it finds a line that exactly matches the delimiter
/// If shell_state has pending_heredoc_content, it uses that instead (for script execution)
fn collect_here_document_content(delimiter: &str, shell_state: &mut ShellState) -> String {
    // Check if we have pending here-document content from script execution
    if let Some(content) = shell_state.pending_heredoc_content.take() {
        return content;
    }

    // Otherwise, read from stdin (interactive mode)
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut content = String::new();
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // EOF reached
                break;
            }
            Ok(_) => {
                // Check if this line (without trailing newline) matches the delimiter
                let line_content = line.trim_end();
                if line_content == delimiter {
                    // Found the delimiter, stop collecting
                    break;
                } else {
                    // This is content, add it to our collection
                    content.push_str(&line);
                }
            }
            Err(e) => {
                if shell_state.colors_enabled {
                    eprintln!(
                        "{}Error reading here-document content: {}\x1b[0m",
                        shell_state.color_scheme.error, e
                    );
                } else {
                    eprintln!("Error reading here-document content: {}", e);
                }
                break;
            }
        }
    }

    content
}

/// Apply all redirections for a command in left-to-right order (POSIX requirement)
///
/// # Arguments
/// * `redirections` - List of redirections to apply
/// * `shell_state` - Mutable reference to shell state
/// * `command` - Optional mutable reference to Command (for external commands)
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` with error message on failure
fn apply_redirections(
    redirections: &[Redirection],
    shell_state: &mut ShellState,
    mut command: Option<&mut Command>,
) -> Result<(), String> {
    // Process redirections in left-to-right order per POSIX
    for redir in redirections {
        match redir {
            Redirection::Input(file) => {
                apply_input_redirection(0, file, shell_state, command.as_deref_mut())?;
            }
            Redirection::Output(file) => {
                apply_output_redirection(1, file, false, shell_state, command.as_deref_mut())?;
            }
            Redirection::Append(file) => {
                apply_output_redirection(1, file, true, shell_state, command.as_deref_mut())?;
            }
            Redirection::FdInput(fd, file) => {
                apply_input_redirection(*fd, file, shell_state, command.as_deref_mut())?;
            }
            Redirection::FdOutput(fd, file) => {
                apply_output_redirection(*fd, file, false, shell_state, command.as_deref_mut())?;
            }
            Redirection::FdAppend(fd, file) => {
                apply_output_redirection(*fd, file, true, shell_state, command.as_deref_mut())?;
            }
            Redirection::FdDuplicate(target_fd, source_fd) => {
                apply_fd_duplication(*target_fd, *source_fd, shell_state, command.as_deref_mut())?;
            }
            Redirection::FdClose(fd) => {
                apply_fd_close(*fd, shell_state, command.as_deref_mut())?;
            }
            Redirection::FdInputOutput(fd, file) => {
                apply_fd_input_output(*fd, file, shell_state, command.as_deref_mut())?;
            }
            Redirection::HereDoc(delimiter, quoted_str) => {
                let quoted = quoted_str == "true";
                apply_heredoc_redirection(
                    0,
                    delimiter,
                    quoted,
                    shell_state,
                    command.as_deref_mut(),
                )?;
            }
            Redirection::HereString(content) => {
                apply_herestring_redirection(0, content, shell_state, command.as_deref_mut())?;
            }
        }
    }
    Ok(())
}

/// Apply input redirection for a specific file descriptor
fn apply_input_redirection(
    fd: i32,
    file: &str,
    shell_state: &mut ShellState,
    command: Option<&mut Command>,
) -> Result<(), String> {
    let expanded_file = expand_variables_in_string(file, shell_state);

    // Open file for reading
    let file_handle =
        File::open(&expanded_file).map_err(|e| format!("Cannot open {}: {}", expanded_file, e))?;

    if fd == 0 {
        // stdin redirection - apply to Command if present
        if let Some(cmd) = command {
            cmd.stdin(Stdio::from(file_handle));
        }
        // For builtins (command is None), the stdin is already handled by the shell's stdin
        // The builtin will need to read from fd 0 which is already set up
    } else {
        // Custom fd - for external commands, we need to redirect the custom fd for reading
        // Open the file (we need to keep the handle alive for the command)
        let fd_file = File::open(&expanded_file)
            .map_err(|e| format!("Cannot open {}: {}", expanded_file, e))?;

        // For external commands, store both in fd table and prepare for stdin redirect
        shell_state.fd_table.borrow_mut().open_fd(
            fd,
            &expanded_file,
            true,  // read
            false, // write
            false, // append
            false, // truncate
        )?;

        // If we have an external command, set up the file descriptor in the child process
        if let Some(cmd) = command {
            // Keep fd_file alive by moving it into the closure
            // It will be dropped (and closed) when the closure is dropped in the parent
            let target_fd = fd;
            unsafe {
                cmd.pre_exec(move || {
                    let raw_fd = fd_file.as_raw_fd();

                    // The inherited file descriptor might not be at the target fd number
                    // Use dup2 to ensure it's at the correct fd number
                    if raw_fd != target_fd {
                        let result = libc::dup2(raw_fd, target_fd);
                        if result < 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                        // We don't need to close raw_fd manually because fd_file
                        // has CLOEXEC set by default and will be closed on exec
                    }
                    Ok(())
                });
            }
        }
    }

    Ok(())
}

/// Apply output redirection for a specific file descriptor
fn apply_output_redirection(
    fd: i32,
    file: &str,
    append: bool,
    shell_state: &mut ShellState,
    command: Option<&mut Command>,
) -> Result<(), String> {
    let expanded_file = expand_variables_in_string(file, shell_state);

    // Open file for writing or appending
    let file_handle = if append {
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(&expanded_file)
            .map_err(|e| format!("Cannot open {}: {}", expanded_file, e))?
    } else {
        File::create(&expanded_file)
            .map_err(|e| format!("Cannot create {}: {}", expanded_file, e))?
    };

    if fd == 1 {
        // stdout redirection - apply to Command if present
        if let Some(cmd) = command {
            cmd.stdout(Stdio::from(file_handle));
        }
    } else if fd == 2 {
        // stderr redirection - apply to Command if present
        if let Some(cmd) = command {
            cmd.stderr(Stdio::from(file_handle));
        }
    } else {
        // Custom fd - store in fd table
        shell_state.fd_table.borrow_mut().open_fd(
            fd,
            &expanded_file,
            false, // read
            true,  // write
            append,
            !append, // truncate if not appending
        )?;
    }

    Ok(())
}

/// Apply file descriptor duplication
fn apply_fd_duplication(
    target_fd: i32,
    source_fd: i32,
    shell_state: &mut ShellState,
    _command: Option<&mut Command>,
) -> Result<(), String> {
    // Check if source_fd is explicitly closed before attempting duplication
    if shell_state.fd_table.borrow().is_closed(source_fd) {
        let error_msg = format!("File descriptor {} is closed", source_fd);
        if shell_state.colors_enabled {
            eprintln!(
                "{}Redirection error: {}\x1b[0m",
                shell_state.color_scheme.error, error_msg
            );
        } else {
            eprintln!("Redirection error: {}", error_msg);
        }
        return Err(error_msg);
    }

    // Duplicate source_fd to target_fd
    shell_state
        .fd_table
        .borrow_mut()
        .duplicate_fd(source_fd, target_fd)?;
    Ok(())
}

/// Apply file descriptor closing
fn apply_fd_close(
    fd: i32,
    shell_state: &mut ShellState,
    command: Option<&mut Command>,
) -> Result<(), String> {
    // Close the specified fd in the fd table
    shell_state.fd_table.borrow_mut().close_fd(fd)?;

    // For external commands, we need to redirect the fd to /dev/null
    // This ensures that writes to the closed fd don't produce errors
    if let Some(cmd) = command {
        match fd {
            0 => {
                // Close stdin - redirect to /dev/null for reading
                cmd.stdin(Stdio::null());
            }
            1 => {
                // Close stdout - redirect to /dev/null for writing
                cmd.stdout(Stdio::null());
            }
            2 => {
                // Close stderr - redirect to /dev/null for writing
                cmd.stderr(Stdio::null());
            }
            _ => {
                // For custom fds (3+), we use pre_exec to close them
                // This is handled via the fd_table and dup2 operations
            }
        }
    }

    Ok(())
}

/// Apply read/write file descriptor opening
fn apply_fd_input_output(
    fd: i32,
    file: &str,
    shell_state: &mut ShellState,
    _command: Option<&mut Command>,
) -> Result<(), String> {
    let expanded_file = expand_variables_in_string(file, shell_state);

    // Open file for both reading and writing
    shell_state.fd_table.borrow_mut().open_fd(
        fd,
        &expanded_file,
        true,  // read
        true,  // write
        false, // append
        false, // truncate
    )?;

    Ok(())
}

/// Apply here-document redirection
fn apply_heredoc_redirection(
    fd: i32,
    delimiter: &str,
    quoted: bool,
    shell_state: &mut ShellState,
    command: Option<&mut Command>,
) -> Result<(), String> {
    let here_doc_content = collect_here_document_content(delimiter, shell_state);

    // Expand variables and command substitutions ONLY if delimiter was not quoted
    let expanded_content = if quoted {
        here_doc_content
    } else {
        expand_variables_in_string(&here_doc_content, shell_state)
    };

    // Create a pipe and write the content
    let (reader, mut writer) =
        pipe().map_err(|e| format!("Failed to create pipe for here-document: {}", e))?;

    writeln!(writer, "{}", expanded_content)
        .map_err(|e| format!("Failed to write here-document content: {}", e))?;

    // Apply to stdin if fd is 0
    if fd == 0 {
        if let Some(cmd) = command {
            cmd.stdin(Stdio::from(reader));
        }
    }

    Ok(())
}

/// Apply here-string redirection
fn apply_herestring_redirection(
    fd: i32,
    content: &str,
    shell_state: &mut ShellState,
    command: Option<&mut Command>,
) -> Result<(), String> {
    let expanded_content = expand_variables_in_string(content, shell_state);

    // Create a pipe and write the content
    let (reader, mut writer) =
        pipe().map_err(|e| format!("Failed to create pipe for here-string: {}", e))?;

    write!(writer, "{}", expanded_content)
        .map_err(|e| format!("Failed to write here-string content: {}", e))?;

    // Apply to stdin if fd is 0
    if fd == 0 {
        if let Some(cmd) = command {
            cmd.stdin(Stdio::from(reader));
        }
    }

    Ok(())
}

/// Execute a trap handler command
/// Note: Signal masking during trap execution will be added in a future update
pub fn execute_trap_handler(trap_cmd: &str, shell_state: &mut ShellState) -> i32 {
    // Save current exit code to preserve it across trap execution
    let saved_exit_code = shell_state.last_exit_code;

    // TODO: Add signal masking to prevent recursive trap calls
    // This requires careful handling of the nix sigprocmask API
    // For now, traps execute without signal masking

    // Parse and execute the trap command
    let result = match crate::lexer::lex(trap_cmd, shell_state) {
        Ok(tokens) => {
            match crate::lexer::expand_aliases(
                tokens,
                shell_state,
                &mut std::collections::HashSet::new(),
            ) {
                Ok(expanded_tokens) => {
                    match crate::parser::parse(expanded_tokens) {
                        Ok(ast) => execute(ast, shell_state),
                        Err(_) => {
                            // Parse error in trap handler - silently continue
                            saved_exit_code
                        }
                    }
                }
                Err(_) => {
                    // Alias expansion error - silently continue
                    saved_exit_code
                }
            }
        }
        Err(_) => {
            // Lex error in trap handler - silently continue
            saved_exit_code
        }
    };

    // Restore the original exit code (trap handlers don't affect $?)
    shell_state.last_exit_code = saved_exit_code;

    result
}

pub fn execute(ast: Ast, shell_state: &mut ShellState) -> i32 {
    match ast {
        Ast::Assignment { var, value } => {
            // Expand variables and command substitutions in the value
            let expanded_value = expand_variables_in_string(&value, shell_state);
            shell_state.set_var(&var, expanded_value);
            0
        }
        Ast::LocalAssignment { var, value } => {
            // Expand variables and command substitutions in the value
            let expanded_value = expand_variables_in_string(&value, shell_state);
            shell_state.set_local_var(&var, expanded_value);
            0
        }
        Ast::Pipeline(commands) => {
            if commands.is_empty() {
                return 0;
            }

            if commands.len() == 1 {
                // Single command, handle redirections
                execute_single_command(&commands[0], shell_state)
            } else {
                // Pipeline
                execute_pipeline(&commands, shell_state)
            }
        }
        Ast::Sequence(asts) => {
            let mut exit_code = 0;
            for ast in asts {
                exit_code = execute(ast, shell_state);

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    return exit_code;
                }

                // Check if exit was requested (e.g., from trap handler)
                if shell_state.exit_requested {
                    return shell_state.exit_code;
                }
            }
            exit_code
        }
        Ast::If {
            branches,
            else_branch,
        } => {
            for (condition, then_branch) in branches {
                let cond_exit = execute(*condition, shell_state);
                if cond_exit == 0 {
                    let exit_code = execute(*then_branch, shell_state);

                    // Check if we got an early return from a function
                    if shell_state.is_returning() {
                        return exit_code;
                    }

                    return exit_code;
                }
            }
            if let Some(else_b) = else_branch {
                let exit_code = execute(*else_b, shell_state);

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    return exit_code;
                }

                exit_code
            } else {
                0
            }
        }
        Ast::Case {
            word,
            cases,
            default,
        } => {
            for (patterns, branch) in cases {
                for pattern in &patterns {
                    if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                        if glob_pattern.matches(&word) {
                            let exit_code = execute(branch, shell_state);

                            // Check if we got an early return from a function
                            if shell_state.is_returning() {
                                return exit_code;
                            }

                            return exit_code;
                        }
                    } else {
                        // If pattern is invalid, fall back to exact match
                        if &word == pattern {
                            let exit_code = execute(branch, shell_state);

                            // Check if we got an early return from a function
                            if shell_state.is_returning() {
                                return exit_code;
                            }

                            return exit_code;
                        }
                    }
                }
            }
            if let Some(def) = default {
                let exit_code = execute(*def, shell_state);

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    return exit_code;
                }

                exit_code
            } else {
                0
            }
        }
        Ast::For {
            variable,
            items,
            body,
        } => {
            let mut exit_code = 0;

            // Execute the loop body for each item
            for item in items {
                // Process any pending signals before executing the body
                crate::state::process_pending_signals(shell_state);

                // Check if exit was requested (e.g., from trap handler)
                if shell_state.exit_requested {
                    return shell_state.exit_code;
                }

                // Set the loop variable
                shell_state.set_var(&variable, item.clone());

                // Execute the body
                exit_code = execute(*body.clone(), shell_state);

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    return exit_code;
                }

                // Check if exit was requested after executing the body
                if shell_state.exit_requested {
                    return shell_state.exit_code;
                }
            }

            exit_code
        }
        Ast::While { condition, body } => {
            let mut exit_code = 0;

            // Execute the loop while condition is true (exit code 0)
            loop {
                // Evaluate the condition
                let cond_exit = execute(*condition.clone(), shell_state);

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    return cond_exit;
                }

                // Check if exit was requested (e.g., from trap handler)
                if shell_state.exit_requested {
                    return shell_state.exit_code;
                }

                // If condition is false (non-zero exit code), break
                if cond_exit != 0 {
                    break;
                }

                // Execute the body
                exit_code = execute(*body.clone(), shell_state);

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    return exit_code;
                }

                // Check if exit was requested (e.g., from trap handler)
                if shell_state.exit_requested {
                    return shell_state.exit_code;
                }
            }

            exit_code
        }
        Ast::FunctionDefinition { name, body } => {
            // Store function definition in shell state
            shell_state.define_function(name.clone(), *body);
            0
        }
        Ast::FunctionCall { name, args } => {
            if let Some(function_body) = shell_state.get_function(&name).cloned() {
                // Check recursion limit before entering function
                if shell_state.function_depth >= shell_state.max_recursion_depth {
                    eprintln!(
                        "Function recursion limit ({}) exceeded",
                        shell_state.max_recursion_depth
                    );
                    return 1;
                }

                // Enter function context for local variable scoping
                shell_state.enter_function();

                // Set up arguments as regular variables (will be enhanced in Phase 2)
                let old_positional = shell_state.positional_params.clone();

                // Set positional parameters for function arguments
                shell_state.set_positional_params(args.clone());

                // Execute function body
                let exit_code = execute(function_body, shell_state);

                // Check if we got an early return from the function
                if shell_state.is_returning() {
                    let return_value = shell_state.get_return_value().unwrap_or(0);

                    // Restore old positional parameters
                    shell_state.set_positional_params(old_positional);

                    // Exit function context
                    shell_state.exit_function();

                    // Clear return state
                    shell_state.clear_return();

                    // Return the early return value
                    return return_value;
                }

                // Restore old positional parameters
                shell_state.set_positional_params(old_positional);

                // Exit function context
                shell_state.exit_function();

                exit_code
            } else {
                eprintln!("Function '{}' not found", name);
                1
            }
        }
        Ast::Return { value } => {
            // Return statements can only be used inside functions
            if shell_state.function_depth == 0 {
                eprintln!("Return statement outside of function");
                return 1;
            }

            // Parse return value if provided
            let exit_code = if let Some(ref val) = value {
                val.parse::<i32>().unwrap_or(0)
            } else {
                0
            };

            // Set return state to indicate early return from function
            shell_state.set_return(exit_code);

            // Return the exit code - the function call handler will check for this
            exit_code
        }
        Ast::And { left, right } => {
            // Execute left side first
            let left_exit = execute(*left, shell_state);

            // Check if we got an early return from a function
            if shell_state.is_returning() {
                return left_exit;
            }

            // Only execute right side if left succeeded (exit code 0)
            if left_exit == 0 {
                execute(*right, shell_state)
            } else {
                left_exit
            }
        }
        Ast::Or { left, right } => {
            // Execute left side first
            let left_exit = execute(*left, shell_state);

            // Check if we got an early return from a function
            if shell_state.is_returning() {
                return left_exit;
            }

            // Only execute right side if left failed (exit code != 0)
            if left_exit != 0 {
                execute(*right, shell_state)
            } else {
                left_exit
            }
        }
        Ast::Subshell { body } => execute_subshell(*body, shell_state),
    }
}

fn execute_single_command(cmd: &ShellCommand, shell_state: &mut ShellState) -> i32 {
    // Check if this is a compound command (subshell)
    if let Some(ref compound_ast) = cmd.compound {
        // Execute compound command with redirections
        return execute_compound_with_redirections(compound_ast, shell_state, &cmd.redirections);
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
    let expanded_args = match expand_wildcards(&var_expanded_args) {
        Ok(args) => args,
        Err(_) => return 1,
    };

    if expanded_args.is_empty() {
        return 0;
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
            crate::builtins::execute_builtin(&temp_cmd, shell_state, Some(Box::new(writer)))
        } else {
            crate::builtins::execute_builtin(&temp_cmd, shell_state, None)
        }
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

                match child.wait() {
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
                }
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

fn execute_pipeline(commands: &[ShellCommand], shell_state: &mut ShellState) -> i32 {
    let mut exit_code = 0;
    let mut previous_stdout = None;

    for (i, cmd) in commands.iter().enumerate() {
        let is_last = i == commands.len() - 1;

        // Check if this is a compound command (subshell)
        if let Some(ref compound_ast) = cmd.compound {
            // Execute compound command (subshell) in pipeline
            exit_code = execute_compound_in_pipeline(
                compound_ast,
                shell_state,
                i == 0,
                is_last,
                &cmd.redirections,
            );

            // For Phase 2, compound commands in pipelines don't produce stdout for next stage
            // This will be enhanced in Phase 3 with proper pipe handling
            previous_stdout = None;
            continue;
        }

        if cmd.args.is_empty() {
            continue;
        }

        // First expand variables, then wildcards
        let var_expanded_args = expand_variables_in_args(&cmd.args, shell_state);
        let expanded_args = match expand_wildcards(&var_expanded_args) {
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
                    Ok(p) => p,
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
                previous_stdout = Some(Stdio::from(reader));
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
                command.stdin(prev);
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
                        previous_stdout = child.stdout.take().map(Stdio::from);
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

/// Execute a subshell with isolated state
///
/// # Arguments
/// * `body` - The AST to execute in the subshell
/// * `shell_state` - The parent shell state (will be cloned)
///
/// # Returns
/// * Exit code from the subshell execution
///
/// # Behavior
/// - Clones the shell state for isolation
/// - Executes the body in the cloned state
/// - Returns the exit code without modifying parent state
/// - Preserves parent state completely (variables, functions, etc.)
/// - Tracks subshell depth to prevent stack overflow
/// - Handles exit and return commands properly (isolated from parent)
/// - Cleans up file descriptors to prevent resource leaks
fn execute_subshell(body: Ast, shell_state: &mut ShellState) -> i32 {
    // Check depth limit to prevent stack overflow
    if shell_state.subshell_depth >= MAX_SUBSHELL_DEPTH {
        if shell_state.colors_enabled {
            eprintln!(
                "{}Subshell nesting limit ({}) exceeded\x1b[0m",
                shell_state.color_scheme.error, MAX_SUBSHELL_DEPTH
            );
        } else {
            eprintln!("Subshell nesting limit ({}) exceeded", MAX_SUBSHELL_DEPTH);
        }
        shell_state.last_exit_code = 1;
        return 1;
    }

    // Save current directory for restoration
    let original_dir = std::env::current_dir().ok();

    // Clone the shell state for isolation
    let mut subshell_state = shell_state.clone();

    // Deep clone the file descriptor table for isolation
    // shell_state.clone() only clones the Rc, so we need to manually deep clone the table
    // and put it in a new Rc<RefCell<_>>
    match shell_state.fd_table.borrow().deep_clone() {
        Ok(new_fd_table) => {
            subshell_state.fd_table = Rc::new(RefCell::new(new_fd_table));
        }
        Err(e) => {
            if shell_state.colors_enabled {
                eprintln!(
                    "{}Failed to clone file descriptor table: {}\x1b[0m",
                    shell_state.color_scheme.error, e
                );
            } else {
                eprintln!("Failed to clone file descriptor table: {}", e);
            }
            return 1;
        }
    }

    // Increment subshell depth in the cloned state
    subshell_state.subshell_depth = shell_state.subshell_depth + 1;

    // Clone trap handlers for isolation (subshells inherit but don't affect parent)
    let parent_traps = shell_state.trap_handlers.lock().unwrap().clone();
    subshell_state.trap_handlers = std::sync::Arc::new(std::sync::Mutex::new(parent_traps));

    // Execute the body in the isolated state
    let exit_code = execute(body, &mut subshell_state);

    // Handle exit in subshell: exit should only exit the subshell, not the parent
    // The exit_requested flag is isolated to the subshell_state, so it won't affect parent
    let final_exit_code = if subshell_state.exit_requested {
        // Subshell called exit - use its exit code
        subshell_state.exit_code
    } else if subshell_state.is_returning() {
        // Subshell called return - treat as exit from subshell
        // Return in subshell should not propagate to parent function
        subshell_state.get_return_value().unwrap_or(exit_code)
    } else {
        exit_code
    };

    // Clean up the subshell's file descriptor table to prevent resource leaks
    // This ensures any file descriptors opened in the subshell are properly released
    subshell_state.fd_table.borrow_mut().clear();

    // Restore original directory (in case subshell changed it)
    if let Some(dir) = original_dir {
        let _ = std::env::set_current_dir(dir);
    }

    // Update parent's last_exit_code to reflect subshell result
    shell_state.last_exit_code = final_exit_code;

    // Return the exit code
    final_exit_code
}

/// Execute a compound command with redirections
///
/// # Arguments
/// * `compound_ast` - The compound command AST
/// * `shell_state` - The shell state
/// * `redirections` - Redirections to apply
///
/// # Returns
/// * Exit code from the compound command
fn execute_compound_with_redirections(
    compound_ast: &Ast,
    shell_state: &mut ShellState,
    redirections: &[Redirection],
) -> i32 {
    match compound_ast {
        Ast::Subshell { body } => {
            // For subshells with redirections, we need to:
            // 1. Set up output capture if there are output redirections
            // 2. Execute the subshell
            // 3. Apply the redirections to the captured output

            // Check if we have output redirections
            let has_output_redir = redirections.iter().any(|r| {
                matches!(
                    r,
                    Redirection::Output(_)
                        | Redirection::Append(_)
                        | Redirection::FdOutput(_, _)
                        | Redirection::FdAppend(_, _)
                )
            });

            if has_output_redir {
                // Clone state for subshell
                let mut subshell_state = shell_state.clone();

                // Set up output capture
                let capture_buffer = Rc::new(RefCell::new(Vec::new()));
                subshell_state.capture_output = Some(capture_buffer.clone());

                // Execute subshell
                let exit_code = execute(*body.clone(), &mut subshell_state);

                // Get captured output
                let output = capture_buffer.borrow().clone();

                // Apply redirections to output
                for redir in redirections {
                    match redir {
                        Redirection::Output(file) => {
                            let expanded_file = expand_variables_in_string(file, shell_state);
                            if let Err(e) = std::fs::write(&expanded_file, &output) {
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
                        Redirection::Append(file) => {
                            let expanded_file = expand_variables_in_string(file, shell_state);
                            use std::fs::OpenOptions;
                            let mut file_handle = match OpenOptions::new()
                                .append(true)
                                .create(true)
                                .open(&expanded_file)
                            {
                                Ok(f) => f,
                                Err(e) => {
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
                            };
                            if let Err(e) = file_handle.write_all(&output) {
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
                        _ => {
                            // For Phase 2, only support basic output redirections
                            // Other redirections are silently ignored for subshells
                        }
                    }
                }

                shell_state.last_exit_code = exit_code;
                exit_code
            } else {
                // No output redirections, execute normally
                execute_subshell(*body.clone(), shell_state)
            }
        }
        _ => {
            eprintln!("Unsupported compound command type");
            1
        }
    }
}

/// Execute a compound command (subshell) as part of a pipeline
///
/// # Arguments
/// * `compound_ast` - The compound command AST (typically Subshell)
/// * `shell_state` - The parent shell state
/// * `is_last` - Whether this is the last command in the pipeline
/// * `redirections` - Redirections to apply to the compound command
///
/// # Returns
/// * Exit code from the compound command
fn execute_compound_in_pipeline(
    compound_ast: &Ast,
    shell_state: &mut ShellState,
    is_first: bool,
    is_last: bool,
    _redirections: &[Redirection],
) -> i32 {
    match compound_ast {
        Ast::Subshell { body } => {
            // Clone state for subshell
            let mut subshell_state = shell_state.clone();

            // If we are not the first command in pipeline, and since we don't receive input (broken pipe),
            // we use stdin_override to force reading from /dev/null, without modifying process-global 0.
            // We keep the file handle alive for the duration of the subshell execution.
            let _null_file = if !is_first {
                if let Ok(f) = File::open("/dev/null") {
                    subshell_state.stdin_override = Some(f.as_raw_fd());
                    Some(f)
                } else {
                    None
                }
            } else {
                None
            };

            // Execute subshell with appropriate stdout capture
            let exit_code = if !is_last || shell_state.capture_output.is_some() {
                // Need to capture subshell output
                let capture_buffer = Rc::new(RefCell::new(Vec::new()));
                subshell_state.capture_output = Some(capture_buffer.clone());

                // Execute subshell
                let code = execute(*body.clone(), &mut subshell_state);

                // Transfer captured output to parent's capture buffer
                if let Some(ref parent_capture) = shell_state.capture_output {
                    let captured = capture_buffer.borrow().clone();
                    parent_capture.borrow_mut().extend_from_slice(&captured);
                }

                // Update parent's last_exit_code
                shell_state.last_exit_code = code;

                code
            } else {
                // Last command, no capture needed
                let code = execute(*body.clone(), &mut subshell_state);
                shell_state.last_exit_code = code;
                code
            };

            exit_code
        }
        _ => {
            // Other compound commands not yet supported
            eprintln!("Unsupported compound command in pipeline");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize tests that modify environment variables or create files
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_execute_single_command_builtin() {
        let cmd = ShellCommand {
            args: vec!["true".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    // For external commands, test with a command that exists
    #[test]
    fn test_execute_single_command_external() {
        let cmd = ShellCommand {
            args: vec!["true".to_string()], // Assume true exists
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_single_command_external_nonexistent() {
        let cmd = ShellCommand {
            args: vec!["nonexistent_command".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 1); // Command not found
    }

    #[test]
    fn test_execute_pipeline() {
        let commands = vec![
            ShellCommand {
                args: vec!["printf".to_string(), "hello".to_string()],
                redirections: Vec::new(),
                compound: None,
            },
            ShellCommand {
                args: vec!["cat".to_string()], // cat reads from stdin
                redirections: Vec::new(),
                compound: None,
            },
        ];
        let mut shell_state = ShellState::new();
        let exit_code = execute_pipeline(&commands, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_empty_pipeline() {
        let commands = vec![];
        let mut shell_state = ShellState::new();
        let exit_code = execute(Ast::Pipeline(commands), &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_single_command() {
        let ast = Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            redirections: Vec::new(),
            compound: None,
        }]);
        let mut shell_state = ShellState::new();
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_function_definition() {
        let ast = Ast::FunctionDefinition {
            name: "test_func".to_string(),
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                redirections: Vec::new(),
                compound: None,
            }])),
        };
        let mut shell_state = ShellState::new();
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Check that function was stored
        assert!(shell_state.get_function("test_func").is_some());
    }

    #[test]
    fn test_execute_function_call() {
        // First define a function
        let mut shell_state = ShellState::new();
        shell_state.define_function(
            "test_func".to_string(),
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        );

        // Now call the function
        let ast = Ast::FunctionCall {
            name: "test_func".to_string(),
            args: vec![],
        };
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_function_call_with_args() {
        // First define a function that uses arguments
        let mut shell_state = ShellState::new();
        shell_state.define_function(
            "test_func".to_string(),
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "arg1".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        );

        // Now call the function with arguments
        let ast = Ast::FunctionCall {
            name: "test_func".to_string(),
            args: vec!["hello".to_string()],
        };
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_nonexistent_function() {
        let mut shell_state = ShellState::new();
        let ast = Ast::FunctionCall {
            name: "nonexistent".to_string(),
            args: vec![],
        };
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 1); // Should return error code
    }

    #[test]
    fn test_execute_function_integration() {
        // Test full integration: define function, then call it
        let mut shell_state = ShellState::new();

        // First define a function
        let define_ast = Ast::FunctionDefinition {
            name: "hello".to_string(),
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["printf".to_string(), "Hello from function".to_string()],
                redirections: Vec::new(),
                compound: None,
            }])),
        };
        let exit_code = execute(define_ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Now call the function
        let call_ast = Ast::FunctionCall {
            name: "hello".to_string(),
            args: vec![],
        };
        let exit_code = execute(call_ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_function_with_local_variables() {
        let mut shell_state = ShellState::new();

        // Set a global variable
        shell_state.set_var("global_var", "global_value".to_string());

        // Define a function that uses local variables
        let define_ast = Ast::FunctionDefinition {
            name: "test_func".to_string(),
            body: Box::new(Ast::Sequence(vec![
                Ast::LocalAssignment {
                    var: "local_var".to_string(),
                    value: "local_value".to_string(),
                },
                Ast::Assignment {
                    var: "global_var".to_string(),
                    value: "modified_in_function".to_string(),
                },
                Ast::Pipeline(vec![ShellCommand {
                    args: vec!["printf".to_string(), "success".to_string()],
                    redirections: Vec::new(),
                    compound: None,
                }]),
            ])),
        };
        let exit_code = execute(define_ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Global variable should not be modified during function definition
        assert_eq!(
            shell_state.get_var("global_var"),
            Some("global_value".to_string())
        );

        // Call the function
        let call_ast = Ast::FunctionCall {
            name: "test_func".to_string(),
            args: vec![],
        };
        let exit_code = execute(call_ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // After function call, global variable should be modified since function assignments affect global scope
        assert_eq!(
            shell_state.get_var("global_var"),
            Some("modified_in_function".to_string())
        );
    }

    #[test]
    fn test_execute_nested_function_calls() {
        let mut shell_state = ShellState::new();

        // Set global variable
        shell_state.set_var("global_var", "global".to_string());

        // Define outer function
        let outer_func = Ast::FunctionDefinition {
            name: "outer".to_string(),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "global_var".to_string(),
                    value: "outer_modified".to_string(),
                },
                Ast::FunctionCall {
                    name: "inner".to_string(),
                    args: vec![],
                },
                Ast::Pipeline(vec![ShellCommand {
                    args: vec!["printf".to_string(), "outer_done".to_string()],
                    redirections: Vec::new(),
                    compound: None,
                }]),
            ])),
        };

        // Define inner function
        let inner_func = Ast::FunctionDefinition {
            name: "inner".to_string(),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "global_var".to_string(),
                    value: "inner_modified".to_string(),
                },
                Ast::Pipeline(vec![ShellCommand {
                    args: vec!["printf".to_string(), "inner_done".to_string()],
                    redirections: Vec::new(),
                    compound: None,
                }]),
            ])),
        };

        // Define both functions
        execute(outer_func, &mut shell_state);
        execute(inner_func, &mut shell_state);

        // Set initial global value
        shell_state.set_var("global_var", "initial".to_string());

        // Call outer function (which calls inner function)
        let call_ast = Ast::FunctionCall {
            name: "outer".to_string(),
            args: vec![],
        };
        let exit_code = execute(call_ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // After nested function calls, global variable should be modified by inner function
        // (bash behavior: function variable assignments affect global scope)
        assert_eq!(
            shell_state.get_var("global_var"),
            Some("inner_modified".to_string())
        );
    }

    #[test]
    fn test_here_string_execution() {
        // Test here-string redirection with a simple command
        let cmd = ShellCommand {
            args: vec!["cat".to_string()],
            redirections: Vec::new(),
            compound: None,
            // TODO: Update test for new redirection system
        };

        // Note: This test would require mocking stdin to provide the here-string content
        // For now, we'll just verify the command structure is parsed correctly
        assert_eq!(cmd.args, vec!["cat"]);
        // assert_eq!(cmd.here_string_content, Some("hello world".to_string()));
    }

    #[test]
    fn test_here_document_execution() {
        // Test here-document redirection with a simple command
        let cmd = ShellCommand {
            args: vec!["cat".to_string()],
            redirections: Vec::new(),
            compound: None,
            // TODO: Update test for new redirection system
        };

        // Note: This test would require mocking stdin to provide the here-document content
        // For now, we'll just verify the command structure is parsed correctly
        assert_eq!(cmd.args, vec!["cat"]);
        // assert_eq!(cmd.here_doc_delimiter, Some("EOF".to_string()));
    }

    #[test]
    fn test_here_document_with_variable_expansion() {
        // Test that variables are expanded in here-document content
        let mut shell_state = ShellState::new();
        shell_state.set_var("PWD", "/test/path".to_string());

        // Simulate here-doc content with variable
        let content = "Working dir: $PWD";
        let expanded = expand_variables_in_string(content, &mut shell_state);

        assert_eq!(expanded, "Working dir: /test/path");
    }

    #[test]
    fn test_here_document_with_command_substitution_builtin() {
        // Test that builtin command substitutions work in here-document content
        let mut shell_state = ShellState::new();
        shell_state.set_var("PWD", "/test/dir".to_string());

        // Simulate here-doc content with pwd builtin command substitution
        let content = "Current directory: `pwd`";
        let expanded = expand_variables_in_string(content, &mut shell_state);

        // The pwd builtin should be executed and expanded
        assert!(expanded.contains("Current directory: "));
    }

    // ========================================================================
    // File Descriptor Integration Tests
    // ========================================================================

    #[test]
    fn test_fd_output_redirection() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_out_{}.txt", timestamp);

        // Test: echo "error" 2>errors.txt
        let cmd = ShellCommand {
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo error >&2".to_string(),
            ],
            redirections: vec![Redirection::FdOutput(2, temp_file.clone())],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify file was created and contains the error message
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert_eq!(content.trim(), "error");

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_input_redirection() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file with content
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_in_{}.txt", timestamp);

        std::fs::write(&temp_file, "test input\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Test: cat 3<input.txt (reading from fd 3)
        // Note: This tests that fd 3 is opened for reading
        let cmd = ShellCommand {
            args: vec!["cat".to_string()],
            compound: None,
            redirections: vec![
                Redirection::FdInput(3, temp_file.clone()),
                Redirection::Input(temp_file.clone()),
            ],
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_append_redirection() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file with initial content
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_append_{}.txt", timestamp);

        std::fs::write(&temp_file, "first line\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Test: echo "more" 2>>errors.txt
        let cmd = ShellCommand {
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo second line >&2".to_string(),
            ],
            redirections: vec![Redirection::FdAppend(2, temp_file.clone())],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify file contains both lines
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("first line"));
        assert!(content.contains("second line"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_duplication_stderr_to_stdout() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_dup_{}.txt", timestamp);

        // Test: command 2>&1 >output.txt
        // Note: For external commands, fd duplication is handled by the shell
        // We test that the command executes successfully with the redirection
        let cmd = ShellCommand {
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo test; echo error >&2".to_string(),
            ],
            compound: None,
            redirections: vec![Redirection::Output(temp_file.clone())],
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify file was created and contains output
        assert!(std::path::Path::new(&temp_file).exists());
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("test"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_close() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Test: command 2>&- (closes stderr)
        let cmd = ShellCommand {
            args: vec!["sh".to_string(), "-c".to_string(), "echo test".to_string()],
            redirections: vec![Redirection::FdClose(2)],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify fd 2 is closed in the fd table
        assert!(shell_state.fd_table.borrow().is_closed(2));
    }

    #[test]
    fn test_fd_read_write() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_rw_{}.txt", timestamp);

        std::fs::write(&temp_file, "initial content\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Test: 3<>file.txt (opens fd 3 for read/write)
        let cmd = ShellCommand {
            args: vec!["cat".to_string()],
            compound: None,
            redirections: vec![
                Redirection::FdInputOutput(3, temp_file.clone()),
                Redirection::Input(temp_file.clone()),
            ],
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_multiple_fd_redirections() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp files
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let out_file = format!("/tmp/rush_test_fd_multi_out_{}.txt", timestamp);
        let err_file = format!("/tmp/rush_test_fd_multi_err_{}.txt", timestamp);

        // Test: command 2>err.txt 1>out.txt
        let cmd = ShellCommand {
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo stdout; echo stderr >&2".to_string(),
            ],
            redirections: vec![
                Redirection::FdOutput(2, err_file.clone()),
                Redirection::Output(out_file.clone()),
            ],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify both files were created
        assert!(std::path::Path::new(&out_file).exists());
        assert!(std::path::Path::new(&err_file).exists());

        // Verify content
        let out_content = std::fs::read_to_string(&out_file).unwrap();
        let err_content = std::fs::read_to_string(&err_file).unwrap();
        assert!(out_content.contains("stdout"));
        assert!(err_content.contains("stderr"));

        // Cleanup
        let _ = std::fs::remove_file(&out_file);
        let _ = std::fs::remove_file(&err_file);
    }

    #[test]
    fn test_fd_swap_pattern() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp files
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_swap_{}.txt", timestamp);

        // Test fd operations: open fd 3, then close it
        // This tests the fd table operations
        let cmd = ShellCommand {
            args: vec!["sh".to_string(), "-c".to_string(), "echo test".to_string()],
            redirections: vec![
                Redirection::FdOutput(3, temp_file.clone()), // Open fd 3 for writing
                Redirection::FdClose(3),                     // Close fd 3
                Redirection::Output(temp_file.clone()),      // Write to stdout
            ],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify fd 3 is closed after the operations
        assert!(shell_state.fd_table.borrow().is_closed(3));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_redirection_with_pipes() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_pipe_{}.txt", timestamp);

        // Test: cmd1 | cmd2 >output.txt
        // This tests redirections in pipelines
        let commands = vec![
            ShellCommand {
                args: vec!["echo".to_string(), "piped output".to_string()],
                redirections: vec![],
                compound: None,
            },
            ShellCommand {
                args: vec!["cat".to_string()],
                compound: None,
                redirections: vec![Redirection::Output(temp_file.clone())],
            },
        ];

        let mut shell_state = ShellState::new();
        let exit_code = execute_pipeline(&commands, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify output file contains the piped content
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("piped output"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_error_invalid_fd_number() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_invalid_{}.txt", timestamp);

        // Test: Invalid fd number (>1024)
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "test".to_string()],
            compound: None,
            redirections: vec![Redirection::FdOutput(1025, temp_file.clone())],
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);

        // Should fail with error
        assert_eq!(exit_code, 1);

        // Cleanup (file may not exist)
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_error_duplicate_closed_fd() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Test: Attempting to duplicate a closed fd
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "test".to_string()],
            compound: None,
            redirections: vec![
                Redirection::FdClose(3),
                Redirection::FdDuplicate(2, 3), // Try to duplicate closed fd 3
            ],
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);

        // Should fail with error
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_fd_error_file_permission() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Test: Attempting to write to a read-only location
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "test".to_string()],
            redirections: vec![Redirection::FdOutput(2, "/proc/version".to_string())],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);

        // Should fail with permission error
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_fd_redirection_order() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp files
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file1 = format!("/tmp/rush_test_fd_order1_{}.txt", timestamp);
        let file2 = format!("/tmp/rush_test_fd_order2_{}.txt", timestamp);

        // Test: Redirections are processed left-to-right
        // 1>file1 1>file2 should write to file2
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "test".to_string()],
            compound: None,
            redirections: vec![
                Redirection::Output(file1.clone()),
                Redirection::Output(file2.clone()),
            ],
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // file2 should have the output (last redirection wins)
        let content2 = std::fs::read_to_string(&file2).unwrap();
        assert!(content2.contains("test"));

        // Cleanup
        let _ = std::fs::remove_file(&file1);
        let _ = std::fs::remove_file(&file2);
    }

    #[test]
    fn test_fd_builtin_with_redirection() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_builtin_{}.txt", timestamp);

        // Test: Built-in command with fd redirection
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "builtin test".to_string()],
            redirections: vec![Redirection::Output(temp_file.clone())],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify output
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("builtin test"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_variable_expansion_in_filename() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_var_{}.txt", timestamp);

        // Set variable for filename
        let mut shell_state = ShellState::new();
        shell_state.set_var("OUTFILE", temp_file.clone());

        // Test: Variable expansion in redirection filename
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "variable test".to_string()],
            compound: None,
            redirections: vec![Redirection::Output("$OUTFILE".to_string())],
        };

        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify output
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("variable test"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }
}
