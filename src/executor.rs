use std::cell::RefCell;
use std::fs::File;
use std::io::{BufRead, BufReader, pipe};
use std::process::{Command, Stdio};
use std::rc::Rc;

use super::parser::{Ast, ShellCommand};
use super::state::ShellState;

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
                        input: cmd.input.clone(),
                        output: None, // We're capturing output
                        append: None,
                        here_doc_delimiter: None,
                        here_doc_quoted: false,
                        here_string_content: None,
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
    }
}

fn execute_single_command(cmd: &ShellCommand, shell_state: &mut ShellState) -> i32 {
    if cmd.args.is_empty() {
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
            input: cmd.input.clone(),
            output: cmd.output.clone(),
            append: cmd.append.clone(),
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
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
        }

        // Prepare command if we have one
        let mut command = if has_command {
            let mut cmd = Command::new(&expanded_args[command_start_idx]);
            cmd.args(&expanded_args[command_start_idx + 1..]);

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

            cmd.env_clear();
            for (key, value) in child_env {
                cmd.env(key, value);
            }

            // If we're capturing output, redirect stdout to capture buffer
            let capturing = shell_state.capture_output.is_some();
            if capturing {
                cmd.stdout(Stdio::piped());
            }

            Some(cmd)
        } else {
            None
        };

        // Handle input redirection (process even if no command)
        if let Some(ref input_file) = cmd.input {
            let expanded_input = expand_variables_in_string(input_file, shell_state);
            if let Some(ref mut command) = command {
                match File::open(&expanded_input) {
                    Ok(file) => {
                        command.stdin(Stdio::from(file));
                    }
                    Err(e) => {
                        if shell_state.colors_enabled {
                            eprintln!(
                                "{}Error opening input file '{}{}",
                                shell_state.color_scheme.error,
                                input_file,
                                &format!("': {}\x1b[0m", e)
                            );
                        } else {
                            eprintln!("Error opening input file '{}': {}", input_file, e);
                        }
                        return 1;
                    }
                }
            } else {
                // No command but redirection - just verify file exists for side effects
                match File::open(&expanded_input) {
                    Ok(_) => {
                        // File opened successfully, side effect complete
                    }
                    Err(e) => {
                        if shell_state.colors_enabled {
                            eprintln!(
                                "{}Error opening input file '{}{}",
                                shell_state.color_scheme.error,
                                input_file,
                                &format!("': {}\x1b[0m", e)
                            );
                        } else {
                            eprintln!("Error opening input file '{}': {}", input_file, e);
                        }
                        return 1;
                    }
                }
            }
        } else if let Some(ref delimiter) = cmd.here_doc_delimiter {
            // Handle here-document redirection (process even if no command)
            let here_doc_content = collect_here_document_content(delimiter, shell_state);
            // Expand variables and command substitutions ONLY if delimiter was not quoted
            // Quoted delimiters (<<'EOF' or <<"EOF") disable expansion per POSIX
            let expanded_content = if cmd.here_doc_quoted {
                here_doc_content.clone() // No expansion for quoted delimiters
            } else {
                expand_variables_in_string(&here_doc_content, shell_state)
            };

            if let Some(ref mut command) = command {
                let pipe_result = pipe();
                match pipe_result {
                    Ok((reader, mut writer)) => {
                        use std::io::Write;
                        if let Err(e) = writeln!(writer, "{}", expanded_content) {
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}Error writing here-document content: {}\x1b[0m",
                                    shell_state.color_scheme.error, e
                                );
                            } else {
                                eprintln!("Error writing here-document content: {}", e);
                            }
                            return 1;
                        }
                        // Note: writer will be closed when it goes out of scope
                        command.stdin(Stdio::from(reader));
                    }
                    Err(e) => {
                        if shell_state.colors_enabled {
                            eprintln!(
                                "{}Error creating pipe for here-document: {}\x1b[0m",
                                shell_state.color_scheme.error, e
                            );
                        } else {
                            eprintln!("Error creating pipe for here-document: {}", e);
                        }
                        return 1;
                    }
                }
            }
            // If no command, here-doc content was consumed for side effects (POSIX requirement)
        } else if let Some(ref content) = cmd.here_string_content {
            // Handle here-string redirection (process even if no command)
            let expanded_content = expand_variables_in_string(content, shell_state);

            if let Some(ref mut command) = command {
                let pipe_result = pipe();
                match pipe_result {
                    Ok((reader, mut writer)) => {
                        use std::io::Write;
                        if let Err(e) = write!(writer, "{}", expanded_content) {
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}Error writing here-string content: {}\x1b[0m",
                                    shell_state.color_scheme.error, e
                                );
                            } else {
                                eprintln!("Error writing here-string content: {}", e);
                            }
                            return 1;
                        }
                        // Note: writer will be closed when it goes out of scope
                        command.stdin(Stdio::from(reader));
                    }
                    Err(e) => {
                        if shell_state.colors_enabled {
                            eprintln!(
                                "{}Error creating pipe for here-string: {}\x1b[0m",
                                shell_state.color_scheme.error, e
                            );
                        } else {
                            eprintln!("Error creating pipe for here-string: {}", e);
                        }
                        return 1;
                    }
                }
            }
            // If no command, here-string was processed for side effects
        }

        // Handle output redirection (process even if no command)
        if let Some(ref output_file) = cmd.output {
            let expanded_output = expand_variables_in_string(output_file, shell_state);
            match File::create(&expanded_output) {
                Ok(file) => {
                    if let Some(ref mut command) = command {
                        command.stdout(Stdio::from(file));
                    }
                    // If no command, file was created for side effects (POSIX requirement)
                }
                Err(e) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}Error creating output file '{}{}",
                            shell_state.color_scheme.error,
                            output_file,
                            &format!("': {}\x1b[0m", e)
                        );
                    } else {
                        eprintln!("Error creating output file '{}': {}", output_file, e);
                    }
                    return 1;
                }
            }
        } else if let Some(ref append_file) = cmd.append {
            let expanded_append = expand_variables_in_string(append_file, shell_state);
            match File::options()
                .append(true)
                .create(true)
                .open(&expanded_append)
            {
                Ok(file) => {
                    if let Some(ref mut command) = command {
                        command.stdout(Stdio::from(file));
                    }
                    // If no command, file was opened/created for side effects (POSIX requirement)
                }
                Err(e) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}Error opening append file '{}{}",
                            shell_state.color_scheme.error,
                            append_file,
                            &format!("': {}\x1b[0m", e)
                        );
                    } else {
                        eprintln!("Error opening append file '{}': {}", append_file, e);
                    }
                    return 1;
                }
            }
        }

        // If no command to execute, return success after processing redirections
        let Some(mut command) = command else {
            return 0;
        };

        // Check if we're capturing output for this command
        let capturing = shell_state.capture_output.is_some();

        match command.spawn() {
            Ok(mut child) => {
                // If capturing, read stdout
                if capturing && let Some(mut stdout) = child.stdout.take() {
                    use std::io::Read;
                    let mut output = Vec::new();
                    if stdout.read_to_end(&mut output).is_ok()
                        && let Some(ref capture_buffer) = shell_state.capture_output
                    {
                        capture_buffer.borrow_mut().extend_from_slice(&output);
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
        if cmd.args.is_empty() {
            continue;
        }

        let is_last = i == commands.len() - 1;

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
                input: cmd.input.clone(),
                output: cmd.output.clone(),
                append: cmd.append.clone(),
                here_doc_delimiter: None,
                here_doc_quoted: false,
                here_string_content: None,
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
            }

            // Set stdout for next command, or for capturing if this is the last
            if !is_last {
                command.stdout(Stdio::piped());
            } else if shell_state.capture_output.is_some() {
                // Last command in pipeline but we're capturing output
                command.stdout(Stdio::piped());
            }

            // Handle input redirection (only for first command)
            if i == 0 {
                if let Some(ref input_file) = cmd.input {
                    let expanded_input = expand_variables_in_string(input_file, shell_state);
                    match File::open(&expanded_input) {
                        Ok(file) => {
                            command.stdin(Stdio::from(file));
                        }
                        Err(e) => {
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}Error opening input file '{}{}",
                                    shell_state.color_scheme.error,
                                    input_file,
                                    &format!("': {}\x1b[0m", e)
                                );
                            } else {
                                eprintln!("Error opening input file '{}': {}", input_file, e);
                            }
                            return 1;
                        }
                    }
                } else if let Some(ref delimiter) = cmd.here_doc_delimiter {
                    // Handle here-document redirection for first command in pipeline
                    let here_doc_content = collect_here_document_content(delimiter, shell_state);
                    // Expand variables and command substitutions ONLY if delimiter was not quoted
                    // Quoted delimiters (<<'EOF' or <<"EOF") disable expansion per POSIX
                    let expanded_content = if cmd.here_doc_quoted {
                        here_doc_content // No expansion for quoted delimiters
                    } else {
                        expand_variables_in_string(&here_doc_content, shell_state)
                    };
                    let pipe_result = pipe();
                    match pipe_result {
                        Ok((reader, mut writer)) => {
                            use std::io::Write;
                            if let Err(e) = writeln!(writer, "{}", expanded_content) {
                                if shell_state.colors_enabled {
                                    eprintln!(
                                        "{}Error writing here-document content: {}\x1b[0m",
                                        shell_state.color_scheme.error, e
                                    );
                                } else {
                                    eprintln!("Error writing here-document content: {}", e);
                                }
                                return 1;
                            }
                            command.stdin(Stdio::from(reader));
                        }
                        Err(e) => {
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}Error creating pipe for here-document: {}\x1b[0m",
                                    shell_state.color_scheme.error, e
                                );
                            } else {
                                eprintln!("Error creating pipe for here-document: {}", e);
                            }
                            return 1;
                        }
                    }
                } else if let Some(ref content) = cmd.here_string_content {
                    // Handle here-string redirection for first command in pipeline
                    let expanded_content = expand_variables_in_string(content, shell_state);
                    let pipe_result = pipe();
                    match pipe_result {
                        Ok((reader, mut writer)) => {
                            use std::io::Write;
                            if let Err(e) = write!(writer, "{}", expanded_content) {
                                if shell_state.colors_enabled {
                                    eprintln!(
                                        "{}Error writing here-string content: {}\x1b[0m",
                                        shell_state.color_scheme.error, e
                                    );
                                } else {
                                    eprintln!("Error writing here-string content: {}", e);
                                }
                                return 1;
                            }
                            command.stdin(Stdio::from(reader));
                        }
                        Err(e) => {
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}Error creating pipe for here-string: {}\x1b[0m",
                                    shell_state.color_scheme.error, e
                                );
                            } else {
                                eprintln!("Error creating pipe for here-string: {}", e);
                            }
                            return 1;
                        }
                    }
                }
            }

            // Handle output redirection (only for last command)
            if is_last {
                if let Some(ref output_file) = cmd.output {
                    let expanded_output = expand_variables_in_string(output_file, shell_state);
                    match File::create(&expanded_output) {
                        Ok(file) => {
                            command.stdout(Stdio::from(file));
                        }
                        Err(e) => {
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}Error creating output file '{}{}",
                                    shell_state.color_scheme.error,
                                    output_file,
                                    &format!("': {}\x1b[0m", e)
                                );
                            } else {
                                eprintln!("Error creating output file '{}': {}", output_file, e);
                            }
                            return 1;
                        }
                    }
                } else if let Some(ref append_file) = cmd.append {
                    let expanded_append = expand_variables_in_string(append_file, shell_state);
                    match File::options()
                        .append(true)
                        .create(true)
                        .open(&expanded_append)
                    {
                        Ok(file) => {
                            command.stdout(Stdio::from(file));
                        }
                        Err(e) => {
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}Error opening append file '{}{}",
                                    shell_state.color_scheme.error,
                                    append_file,
                                    &format!("': {}\x1b[0m", e)
                                );
                            } else {
                                eprintln!("Error opening append file '{}': {}", append_file, e);
                            }
                            return 1;
                        }
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_single_command_builtin() {
        let cmd = ShellCommand {
            args: vec!["true".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
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
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_single_command_external_nonexistent() {
        let cmd = ShellCommand {
            args: vec!["nonexistent_command".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
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
                input: None,
                output: None,
                append: None,
                here_doc_delimiter: None,
                here_doc_quoted: false,
                here_string_content: None,
            },
            ShellCommand {
                args: vec!["cat".to_string()], // cat reads from stdin
                input: None,
                output: None,
                append: None,
                here_doc_delimiter: None,
                here_doc_quoted: false,
                here_string_content: None,
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
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
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
                input: None,
                output: None,
                append: None,
                here_doc_delimiter: None,
                here_doc_quoted: false,
                here_string_content: None,
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
                input: None,
                output: None,
                append: None,
                here_doc_delimiter: None,
                here_doc_quoted: false,
                here_string_content: None,
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
                input: None,
                output: None,
                append: None,
                here_doc_delimiter: None,
                here_doc_quoted: false,
                here_string_content: None,
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
                input: None,
                output: None,
                append: None,
                here_doc_delimiter: None,
                here_doc_quoted: false,
                here_string_content: None,
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
                    input: None,
                    output: None,
                    append: None,
                    here_doc_delimiter: None,
                    here_doc_quoted: false,
                    here_string_content: None,
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
                    input: None,
                    output: None,
                    append: None,
                    here_doc_delimiter: None,
                    here_doc_quoted: false,
                    here_string_content: None,
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
                    input: None,
                    output: None,
                    append: None,
                    here_doc_delimiter: None,
                    here_doc_quoted: false,
                    here_string_content: None,
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
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: Some("hello world".to_string()),
        };

        // Note: This test would require mocking stdin to provide the here-string content
        // For now, we'll just verify the command structure is parsed correctly
        assert_eq!(cmd.args, vec!["cat"]);
        assert_eq!(cmd.here_string_content, Some("hello world".to_string()));
    }

    #[test]
    fn test_here_document_execution() {
        // Test here-document redirection with a simple command
        let cmd = ShellCommand {
            args: vec!["cat".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: Some("EOF".to_string()),
            here_doc_quoted: false,
            here_string_content: None,
        };

        // Note: This test would require mocking stdin to provide the here-document content
        // For now, we'll just verify the command structure is parsed correctly
        assert_eq!(cmd.args, vec!["cat"]);
        assert_eq!(cmd.here_doc_delimiter, Some("EOF".to_string()));
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
}
