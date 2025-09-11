use std::fs::File;
use std::process::{Command, Stdio};

use super::parser::{Ast, ShellCommand};

pub fn execute(ast: Ast) -> i32 {
    match ast {
        Ast::Pipeline(commands) => {
            if commands.is_empty() {
                return 0;
            }

            if commands.len() == 1 {
                // Single command, handle redirections
                execute_single_command(&commands[0])
            } else {
                // Pipeline
                execute_pipeline(&commands)
            }
        }
        Ast::Sequence(asts) => {
            let mut exit_code = 0;
            for ast in asts {
                exit_code = execute(ast);
            }
            exit_code
        }
        Ast::If {
            branches,
            else_branch,
        } => {
            for (condition, then_branch) in branches {
                let cond_exit = execute(*condition);
                if cond_exit == 0 {
                    return execute(*then_branch);
                }
            }
            if let Some(else_b) = else_branch {
                execute(*else_b)
            } else {
                0
            }
        }
        Ast::Case {
            word,
            cases,
            default,
        } => {
            // For now, simple implementation without glob matching
            for (pattern, branch) in cases {
                if word == pattern {
                    return execute(branch);
                }
            }
            if let Some(def) = default {
                execute(*def)
            } else {
                0
            }
        }
    }
}

fn execute_single_command(cmd: &ShellCommand) -> i32 {
    if cmd.args.is_empty() {
        return 0;
    }

    if crate::builtins::is_builtin(&cmd.args[0]) {
        crate::builtins::execute_builtin(cmd)
    } else {
        let mut command = Command::new(&cmd.args[0]);
        command.args(&cmd.args[1..]);

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

fn execute_pipeline(commands: &[ShellCommand]) -> i32 {
    let mut exit_code = 0;
    let mut previous_stdout = None;

    for (i, cmd) in commands.iter().enumerate() {
        if cmd.args.is_empty() {
            continue;
        }

        let is_last = i == commands.len() - 1;

        if crate::builtins::is_builtin(&cmd.args[0]) {
            // Built-ins in pipelines are tricky - for now, execute them separately
            // This is not perfect but better than nothing
            if let Some(_prev) = previous_stdout {
                // We can't easily pipe to built-ins, so just execute
                eprintln!("Warning: Built-in in pipeline may not work as expected");
            }
            exit_code = crate::builtins::execute_builtin(cmd);
            previous_stdout = None;
        } else {
            let mut command = Command::new(&cmd.args[0]);
            command.args(&cmd.args[1..]);

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
                    eprintln!("Error spawning command '{}': {}", cmd.args[0], e);
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
        let exit_code = execute_single_command(&cmd);
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
        let exit_code = execute_single_command(&cmd);
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
        let exit_code = execute_single_command(&cmd);
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
        let exit_code = execute_pipeline(&commands);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_empty_pipeline() {
        let commands = vec![];
        let exit_code = execute(Ast::Pipeline(commands));
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
        let exit_code = execute(ast);
        assert_eq!(exit_code, 0);
    }
}
