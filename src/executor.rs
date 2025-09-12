use os_pipe::pipe;
use std::fs::File;
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

            while let Some(&c) = next_ch {
                if c.is_alphanumeric() || c == '_' {
                    var_name.push(c);
                    chars.next(); // consume the character
                    next_ch = chars.peek();
                } else {
                    break;
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
                    eprintln!("Error opening input file '{}': {}", input_file, e);
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
                    eprintln!("Error creating output file '{}': {}", output_file, e);
                    return 1;
                }
            }
        } else if let Some(ref append_file) = cmd.append {
            match File::options().append(true).create(true).open(append_file) {
                Ok(file) => {
                    command.stdout(Stdio::from(file));
                }
                Err(e) => {
                    eprintln!("Error opening append file '{}': {}", append_file, e);
                    return 1;
                }
            }
        }

        match command.spawn() {
            Ok(mut child) => match child.wait() {
                Ok(status) => status.code().unwrap_or(0),
                Err(e) => {
                    eprintln!("Error waiting for command: {}", e);
                    1
                }
            },
            Err(e) => {
                eprintln!("Command spawn error: {}", e);
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
                        eprintln!("Error creating pipe for builtin: {}", e);
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
                            eprintln!("Error opening input file '{}': {}", input_file, e);
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
                            eprintln!("Error creating output file '{}': {}", output_file, e);
                            return 1;
                        }
                    }
                } else if let Some(ref append_file) = cmd.append {
                    match File::options().append(true).create(true).open(append_file) {
                        Ok(file) => {
                            command.stdout(Stdio::from(file));
                        }
                        Err(e) => {
                            eprintln!("Error opening append file '{}': {}", append_file, e);
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
                            eprintln!("Error waiting for command: {}", e);
                            exit_code = 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error spawning command '{}': {}", expanded_args[0], e);
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
            args: vec!["echo".to_string(), "test".to_string()],
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
                args: vec!["echo".to_string(), "hello".to_string()],
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
}
