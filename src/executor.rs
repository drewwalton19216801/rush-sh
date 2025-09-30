use std::fs::File;
use std::io::pipe;
use std::process::{Command, Stdio};

use super::parser::{Ast, ShellCommand};
use super::state::ShellState;

fn expand_variables_in_args(args: &[String], shell_state: &ShellState) -> Vec<String> {
    let mut expanded_args = Vec::new();

    for arg in args {
        // Expand variables within the argument string
        let expanded_arg = expand_variables_in_string(arg, shell_state);
        expanded_args.push(expanded_arg);
    }

    expanded_args
}

fn expand_variables_in_string(input: &str, shell_state: &ShellState) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            // Check if this is a variable
            let mut var_name = String::new();
            let mut next_ch = chars.peek();

            // Handle special single-character variables first
            if let Some(&c) = next_ch {
                if c == '?' || c == '$' || c == '0' {
                    var_name.push(c);
                    chars.next(); // consume the character
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
                    // Variable not found, keep the literal
                    result.push('$');
                    result.push_str(&var_name);
                }
            } else {
                result.push('$');
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

pub fn execute(ast: Ast, shell_state: &mut ShellState) -> i32 {
    match ast {
        Ast::Assignment { var, value } => {
            // Expand substitutions in the value
            let tokens = crate::lexer::lex(&value, shell_state).unwrap_or_else(|_| vec![]);
            let expanded_value = if !tokens.is_empty() {
                // Collect all Word tokens and join them with spaces
                let words: Vec<String> = tokens
                    .iter()
                    .filter_map(|token| {
                        if let crate::lexer::Token::Word(word) = token {
                            Some(word.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                if !words.is_empty() {
                    words.join(" ")
                } else {
                    value
                }
            } else {
                value
            };
            shell_state.set_var(&var, expanded_value);
            0
        }
        Ast::LocalAssignment { var, value } => {
            // Expand substitutions in the value
            let tokens = crate::lexer::lex(&value, shell_state).unwrap_or_else(|_| vec![]);
            let expanded_value = if !tokens.is_empty() {
                // Collect all Word tokens and join them with spaces
                let words: Vec<String> = tokens
                    .iter()
                    .filter_map(|token| {
                        if let crate::lexer::Token::Word(word) = token {
                            Some(word.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                if !words.is_empty() {
                    words.join(" ")
                } else {
                    value
                }
            } else {
                value
            };
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
                    return execute(*then_branch, shell_state);
                }
            }
            if let Some(else_b) = else_branch {
                execute(*else_b, shell_state)
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
                            return execute(branch, shell_state);
                        }
                    } else {
                        // If pattern is invalid, fall back to exact match
                        if &word == pattern {
                            return execute(branch, shell_state);
                        }
                    }
                }
            }
            if let Some(def) = default {
                execute(*def, shell_state)
            } else {
                0
            }
        }
        Ast::FunctionDefinition { name, body } => {
            // Store function definition in shell state
            shell_state.define_function(name.clone(), *body);
            0
        }
        Ast::FunctionCall { name, args } => {
            if let Some(function_body) = shell_state.get_function(&name).cloned() {
                // Enter function context for local variable scoping
                shell_state.enter_function();

                // Set up arguments as regular variables (will be enhanced in Phase 2)
                let old_positional = shell_state.positional_params.clone();

                // Set positional parameters for function arguments
                shell_state.set_positional_params(args.clone());

                // Execute function body
                let exit_code = execute(function_body, shell_state);

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
        };
        crate::builtins::execute_builtin(&temp_cmd, shell_state, None)
    } else {
        let mut command = Command::new(&expanded_args[0]);
        command.args(&expanded_args[1..]);

        // Set environment for child process
        let child_env = shell_state.get_env_for_child();
        command.env_clear();
        for (key, value) in child_env {
            command.env(key, value);
        }

        // Handle input redirection
        if let Some(ref input_file) = cmd.input {
            match File::open(input_file) {
                Ok(file) => {
                    command.stdin(Stdio::from(file));
                }
                Err(e) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}{}{}{}",
                            shell_state.color_scheme.error,
                            "Error opening input file '",
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

        // Handle output redirection
        if let Some(ref output_file) = cmd.output {
            match File::create(output_file) {
                Ok(file) => {
                    command.stdout(Stdio::from(file));
                }
                Err(e) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}{}{}{}",
                            shell_state.color_scheme.error,
                            "Error creating output file '",
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
            match File::options().append(true).create(true).open(append_file) {
                Ok(file) => {
                    command.stdout(Stdio::from(file));
                }
                Err(e) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}{}{}{}",
                            shell_state.color_scheme.error,
                            "Error opening append file '",
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

        match command.spawn() {
            Ok(mut child) => match child.wait() {
                Ok(status) => status.code().unwrap_or(0),
                Err(e) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}{}{}{}",
                            shell_state.color_scheme.error,
                            "Error waiting for command: ",
                            e,
                            "\x1b[0m"
                        );
                    } else {
                        eprintln!("Error waiting for command: {}", e);
                    }
                    1
                }
            },
            Err(e) => {
                if shell_state.colors_enabled {
                    eprintln!(
                        "{}{}{}{}",
                        shell_state.color_scheme.error, "Command spawn error: ", e, "\x1b[0m"
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
            };
            if !is_last {
                // Create a safe pipe
                let (reader, writer) = match pipe() {
                    Ok(p) => p,
                    Err(e) => {
                        if shell_state.colors_enabled {
                            eprintln!(
                                "{}{}{}{}",
                                shell_state.color_scheme.error,
                                "Error creating pipe for builtin: ",
                                e,
                                "\x1b[0m"
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
                // Last command: no need to pipe output
                exit_code = crate::builtins::execute_builtin(&temp_cmd, shell_state, None);
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

            // Set stdout for next command, unless this is the last
            if !is_last {
                command.stdout(Stdio::piped());
            }

            // Handle input redirection (only for first command)
            if i == 0 {
                if let Some(ref input_file) = cmd.input {
                    match File::open(input_file) {
                        Ok(file) => {
                            command.stdin(Stdio::from(file));
                        }
                        Err(e) => {
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}{}{}{}",
                                    shell_state.color_scheme.error,
                                    "Error opening input file '",
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
            }

            // Handle output redirection (only for last command)
            if is_last {
                if let Some(ref output_file) = cmd.output {
                    match File::create(output_file) {
                        Ok(file) => {
                            command.stdout(Stdio::from(file));
                        }
                        Err(e) => {
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}{}{}{}",
                                    shell_state.color_scheme.error,
                                    "Error creating output file '",
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
                    match File::options().append(true).create(true).open(append_file) {
                        Ok(file) => {
                            command.stdout(Stdio::from(file));
                        }
                        Err(e) => {
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}{}{}{}",
                                    shell_state.color_scheme.error,
                                    "Error opening append file '",
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
                    }
                    match child.wait() {
                        Ok(status) => {
                            exit_code = status.code().unwrap_or(0);
                        }
                        Err(e) => {
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}{}{}{}",
                                    shell_state.color_scheme.error,
                                    "Error waiting for command: ",
                                    e,
                                    "\x1b[0m"
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
                            "{}{}{}{}{}",
                            shell_state.color_scheme.error,
                            "Error spawning command '",
                            expanded_args[0],
                            &format!("': {}\x1b[0m", e),
                            ""
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
        };
        let mut shell_state = crate::state::ShellState::new();
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
        };
        let mut shell_state = crate::state::ShellState::new();
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
        };
        let mut shell_state = crate::state::ShellState::new();
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
            },
            ShellCommand {
                args: vec!["cat".to_string()], // cat reads from stdin
                input: None,
                output: None,
                append: None,
            },
        ];
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute_pipeline(&commands, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_empty_pipeline() {
        let commands = vec![];
        let mut shell_state = crate::state::ShellState::new();
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
        }]);
        let mut shell_state = crate::state::ShellState::new();
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
            }])),
        };
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Check that function was stored
        assert!(shell_state.get_function("test_func").is_some());
    }

    #[test]
    fn test_execute_function_call() {
        // First define a function
        let mut shell_state = crate::state::ShellState::new();
        shell_state.define_function(
            "test_func".to_string(),
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                input: None,
                output: None,
                append: None,
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
        let mut shell_state = crate::state::ShellState::new();
        shell_state.define_function(
            "test_func".to_string(),
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "arg1".to_string()],
                input: None,
                output: None,
                append: None,
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
        let mut shell_state = crate::state::ShellState::new();
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
        let mut shell_state = crate::state::ShellState::new();

        // First define a function
        let define_ast = Ast::FunctionDefinition {
            name: "hello".to_string(),
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["printf".to_string(), "Hello from function".to_string()],
                input: None,
                output: None,
                append: None,
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
        let mut shell_state = crate::state::ShellState::new();

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
                }]),
            ])),
        };
        let exit_code = execute(define_ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Global variable should not be modified during function definition
        assert_eq!(shell_state.get_var("global_var"), Some("global_value".to_string()));

        // Call the function
        let call_ast = Ast::FunctionCall {
            name: "test_func".to_string(),
            args: vec![],
        };
        let exit_code = execute(call_ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // After function call, global variable should be modified since function assignments affect global scope
        assert_eq!(shell_state.get_var("global_var"), Some("modified_in_function".to_string()));
    }

    #[test]
    fn test_execute_nested_function_calls() {
        let mut shell_state = crate::state::ShellState::new();

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
        assert_eq!(shell_state.get_var("global_var"), Some("inner_modified".to_string()));
    }
}
