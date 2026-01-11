use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use crate::lexer::is_shell_keyword;
use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct TypeBuiltin;

impl super::Builtin for TypeBuiltin {
    fn name(&self) -> &'static str {
        "type"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Display information about command type"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        // Check if we have any arguments
        if cmd.args.len() < 2 {
            if shell_state.colors_enabled {
                let _ = writeln!(
                    output_writer,
                    "{}type: usage: type [-p] name [name ...]\x1b[0m",
                    shell_state.color_scheme.error
                );
            } else {
                let _ = writeln!(output_writer, "type: usage: type [-p] name [name ...]");
            }
            return 1;
        }

        // Parse options and arguments
        let mut path_only = false;
        let mut names = Vec::new();
        let mut i = 1;

        while i < cmd.args.len() {
            let arg = &cmd.args[i];
            if arg == "-p" {
                path_only = true;
            } else if arg.starts_with('-') {
                // Invalid option
                if shell_state.colors_enabled {
                    let _ = writeln!(
                        output_writer,
                        "{}type: {}: invalid option\x1b[0m",
                        shell_state.color_scheme.error, arg
                    );
                } else {
                    let _ = writeln!(output_writer, "type: {}: invalid option", arg);
                }
                return 1;
            } else {
                names.push(arg.clone());
            }
            i += 1;
        }

        // Check if we have any names to process
        if names.is_empty() {
            if shell_state.colors_enabled {
                let _ = writeln!(
                    output_writer,
                    "{}type: usage: type [-p] name [name ...]\x1b[0m",
                    shell_state.color_scheme.error
                );
            } else {
                let _ = writeln!(output_writer, "type: usage: type [-p] name [name ...]");
            }
            return 1;
        }

        let mut all_found = true;

        for name in &names {
            let mut found = false;

            if path_only {
                // With -p flag, only search for external commands in PATH
                if let Some(path) = find_in_path(name, shell_state) {
                    let _ = writeln!(output_writer, "{}", path);
                    found = true;
                }
                // For -p, we don't report "not found" for non-external commands
                // We just produce no output, but still mark as found if it exists as alias/keyword/function/builtin
                else if shell_state.get_alias(name).is_some()
                    || is_shell_keyword(name)
                    || shell_state.functions.contains_key(name)
                    || super::is_builtin(name)
                {
                    found = true;
                }
            } else {
                // Without -p flag, check in priority order and display type
                // 1. Check aliases
                if let Some(alias_value) = shell_state.get_alias(name) {
                    let _ = writeln!(output_writer, "{} is aliased to '{}'", name, alias_value);
                    found = true;
                }
                // 2. Check keywords
                else if is_shell_keyword(name) {
                    let _ = writeln!(output_writer, "{} is a shell keyword", name);
                    found = true;
                }
                // 3. Check functions
                else if shell_state.functions.contains_key(name) {
                    let _ = writeln!(output_writer, "{} is a function", name);
                    found = true;
                }
                // 4. Check built-ins
                else if super::is_builtin(name) {
                    let _ = writeln!(output_writer, "{} is a shell builtin", name);
                    found = true;
                }
                // 5. Check external commands in PATH
                else if let Some(path) = find_in_path(name, shell_state) {
                    let _ = writeln!(output_writer, "{} is {}", name, path);
                    found = true;
                }
            }

            // If not found, print error
            if !found {
                if shell_state.colors_enabled {
                    let _ = writeln!(
                        output_writer,
                        "{}{}: not found\x1b[0m",
                        shell_state.color_scheme.error, name
                    );
                } else {
                    let _ = writeln!(output_writer, "{}: not found", name);
                }
                all_found = false;
            }
        }

        if all_found {
            0
        } else {
            1
        }
    }
}

/// Search for a command in PATH
fn find_in_path(name: &str, shell_state: &ShellState) -> Option<String> {
    // Get PATH from shell state
    let path_var = shell_state.get_var("PATH")?;

    // Split PATH by colon
    for dir in path_var.split(':') {
        if dir.is_empty() {
            continue;
        }

        // Construct full path
        let full_path = format!("{}/{}", dir, name);

        // Check if file exists and is executable
        if let Ok(metadata) = fs::metadata(&full_path) {
            if metadata.is_file() {
                // Check if executable (any execute bit set)
                let permissions = metadata.permissions();
                if permissions.mode() & 0o111 != 0 {
                    return Some(full_path);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;
    use crate::parser::Ast;

    #[test]
    fn test_type_no_args() {
        let cmd = ShellCommand {
            args: vec!["type".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("usage"));
    }

    #[test]
    fn test_type_invalid_option() {
        let cmd = ShellCommand {
            args: vec!["type".to_string(), "-x".to_string(), "cd".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("invalid option"));
    }

    #[test]
    fn test_type_alias() {
        let cmd = ShellCommand {
            args: vec!["type".to_string(), "ll".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_alias("ll", "ls -l".to_string());
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("ll is aliased to 'ls -l'"));
    }

    #[test]
    fn test_type_keyword() {
        let cmd = ShellCommand {
            args: vec!["type".to_string(), "if".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("if is a shell keyword"));
    }

    #[test]
    fn test_type_function() {
        let cmd = ShellCommand {
            args: vec!["type".to_string(), "myfunc".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        // Define a dummy function with an empty body
        let empty_body = Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            redirections: Vec::new(),
            compound: None,
        }]);
        shell_state.define_function("myfunc".to_string(), empty_body);
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("myfunc is a function"));
    }

    #[test]
    fn test_type_builtin() {
        let cmd = ShellCommand {
            args: vec!["type".to_string(), "cd".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("cd is a shell builtin"));
    }

    #[test]
    fn test_type_external_command() {
        let cmd = ShellCommand {
            args: vec!["type".to_string(), "ls".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        // Set a PATH that includes /bin
        shell_state.set_var("PATH", "/bin:/usr/bin".to_string());
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("ls is /"));
    }

    #[test]
    fn test_type_not_found() {
        let cmd = ShellCommand {
            args: vec!["type".to_string(), "nonexistent_command".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_var("PATH", "/bin:/usr/bin".to_string());
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("nonexistent_command: not found"));
    }

    #[test]
    fn test_type_multiple_args() {
        let cmd = ShellCommand {
            args: vec![
                "type".to_string(),
                "cd".to_string(),
                "if".to_string(),
                "ls".to_string(),
            ],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_var("PATH", "/bin:/usr/bin".to_string());
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("cd is a shell builtin"));
        assert!(output_str.contains("if is a shell keyword"));
        assert!(output_str.contains("ls is /"));
    }

    #[test]
    fn test_type_multiple_args_with_not_found() {
        let cmd = ShellCommand {
            args: vec![
                "type".to_string(),
                "cd".to_string(),
                "nonexistent".to_string(),
                "ls".to_string(),
            ],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_var("PATH", "/bin:/usr/bin".to_string());
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // Should return 1 because one was not found
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("cd is a shell builtin"));
        assert!(output_str.contains("nonexistent: not found"));
        assert!(output_str.contains("ls is /"));
    }

    #[test]
    fn test_type_path_only_option() {
        let cmd = ShellCommand {
            args: vec!["type".to_string(), "-p".to_string(), "ls".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_var("PATH", "/bin:/usr/bin".to_string());
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        // With -p, should only show the path, not "ls is /bin/ls"
        assert!(output_str.starts_with('/'));
        assert!(output_str.contains("/ls"));
        assert!(!output_str.contains("ls is"));
    }

    #[test]
    fn test_type_path_only_builtin() {
        let cmd = ShellCommand {
            args: vec!["type".to_string(), "-p".to_string(), "cd".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        // With -p, builtins should produce no output
        assert!(output_str.trim().is_empty());
    }

    #[test]
    fn test_type_path_only_alias() {
        let cmd = ShellCommand {
            args: vec!["type".to_string(), "-p".to_string(), "ll".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_alias("ll", "ls -l".to_string());
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        // With -p, aliases should produce no output
        assert!(output_str.trim().is_empty());
    }

    #[test]
    fn test_type_priority_order() {
        let cmd = ShellCommand {
            args: vec!["type".to_string(), "test".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        // Set an alias for 'test' (should take priority over builtin)
        shell_state.set_alias("test", "echo testing".to_string());
        shell_state.set_var("PATH", "/bin:/usr/bin".to_string());
        let builtin = TypeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        // Should report as alias, not builtin
        assert!(output_str.contains("test is aliased to 'echo testing'"));
    }

    #[test]
    fn test_type_all_keywords() {
        let keywords = vec![
            "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until", "do",
            "done", "{", "}", "!", "in",
        ];
        let mut shell_state = ShellState::new();
        let builtin = TypeBuiltin;

        for keyword in keywords {
            let cmd = ShellCommand {
                args: vec!["type".to_string(), keyword.to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
            assert_eq!(exit_code, 0);
            let output_str = String::from_utf8(output).unwrap();
            assert!(
                output_str.contains(&format!("{} is a shell keyword", keyword)),
                "Failed for keyword: {}",
                keyword
            );
        }
    }
}