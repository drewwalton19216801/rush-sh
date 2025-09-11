use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::parser::ShellCommand;
use crate::state::ShellState;

const BUILTINS: &[&str] = &[
    "cd", "echo", "pwd", "env", "exit", "help", "source", "export", "unset",
];

const BUILTIN_DESCRIPTIONS: &[(&str, &str)] = &[
    ("cd", "Change directory"),
    ("echo", "Print arguments"),
    ("pwd", "Print working directory"),
    ("env", "Print environment variables"),
    ("exit", "Exit the shell"),
    ("help", "Show this help message"),
    ("source", "Execute a script file with rush"),
    ("export", "Export variables to environment"),
    ("unset", "Unset shell variables"),
];

pub fn is_builtin(cmd: &str) -> bool {
    BUILTINS.contains(&cmd)
}

pub fn get_builtin_commands() -> Vec<String> {
    BUILTINS.iter().map(|s| s.to_string()).collect()
}

pub fn execute_builtin(
    cmd: &ShellCommand,
    shell_state: &mut ShellState,
    output_override: Option<Box<dyn std::io::Write>>,
) -> i32 {
    // Handle input redirection for built-ins that might need it
    let _input_content = if let Some(ref input_file) = cmd.input {
        match std::fs::read_to_string(input_file) {
            Ok(content) => Some(content),
            Err(e) => {
                eprintln!("Error reading input file '{}': {}", input_file, e);
                return 1;
            }
        }
    } else {
        None
    };

    // Prepare output destination
    let mut output_writer: Box<dyn Write> = if let Some(override_writer) = output_override {
        override_writer
    } else if let Some(ref output_file) = cmd.output {
        match File::create(output_file) {
            Ok(file) => Box::new(file),
            Err(e) => {
                eprintln!("Error creating output file '{}': {}", output_file, e);
                return 1;
            }
        }
    } else if let Some(ref append_file) = cmd.append {
        match File::options().append(true).create(true).open(append_file) {
            Ok(file) => Box::new(file),
            Err(e) => {
                eprintln!("Error opening append file '{}': {}", append_file, e);
                return 1;
            }
        }
    } else {
        Box::new(io::stdout())
    };

    match cmd.args[0].as_str() {
        "cd" => {
            // cd doesn't produce output, so redirections don't make sense for it
            // But we still handle it for consistency
            let dir = if cmd.args.len() > 1 {
                cmd.args[1].clone()
            } else {
                "~".to_string()
            };
            let path = if dir == "~" {
                env::var("HOME").unwrap_or_else(|_| "/".to_string())
            } else {
                dir
            };
            if let Err(e) = env::set_current_dir(Path::new(&path)) {
                let _ = writeln!(output_writer, "cd: {}: {}", path, e);
                1
            } else {
                0
            }
        }
        "echo" => {
            let output = cmd.args[1..].join(" ");
            let _ = writeln!(output_writer, "{}", output);
            0
        }
        "pwd" => match env::current_dir() {
            Ok(path) => {
                let _ = writeln!(output_writer, "{}", path.display());
                0
            }
            Err(e) => {
                let _ = writeln!(output_writer, "pwd: {}", e);
                1
            }
        },
        "env" => {
            // Show exported shell variables first
            for var_name in &shell_state.exported {
                if let Some(value) = shell_state.variables.get(var_name) {
                    let _ = writeln!(output_writer, "{}={}", var_name, value);
                }
            }
            // Then show environment variables (excluding those already shown)
            for (key, value) in env::vars() {
                if !shell_state.exported.contains(&key) {
                    let _ = writeln!(output_writer, "{}={}", key, value);
                }
            }
            0
        }
        "exit" => {
            // For now, just return 0; main handles exit
            0
        }
        "help" => {
            // Attempt to write the header, handling potential errors
            if writeln!(output_writer, "Rush Shell v{}", env!("CARGO_PKG_VERSION")).is_err()
                || writeln!(output_writer, "").is_err()
                || writeln!(output_writer, "Available built-in commands:").is_err() {
                return 1; // Return error if header write fails
            }

            // Iterate over the complete builtins list with descriptions
            for (cmd, desc) in BUILTIN_DESCRIPTIONS {
                // Use explicit formatting for better readability
                let formatted_line = format!("  {:<12} {}", cmd, desc);
                if writeln!(output_writer, "{}", formatted_line).is_err() {
                    return 1; // Return error if any command write fails
                }
            }
            0
        }
        "source" => {
            if cmd.args.len() < 2 {
                let _ = writeln!(output_writer, "source: missing script file operand");
                return 1;
            }
            let script_file = &cmd.args[1];

            match std::fs::read_to_string(script_file) {
                Ok(content) => {
                    let mut exit_code = 0;
                    for line in content.lines() {
                        let line = line.trim();
                        // Skip shebang lines and empty lines
                        if line.is_empty() || line.starts_with("#!") {
                            continue;
                        }
                        // Skip comment lines
                        if line.starts_with("#") {
                            continue;
                        }
                        // Execute the line using the same logic as main.rs
                        // Note: source builtin doesn't have access to shell state, so we create a temporary one
                        // This is a limitation - sourced scripts don't share variables with parent
                        let mut temp_state = crate::state::ShellState::new();
                        match crate::lexer::lex(line, &temp_state) {
                            Ok(tokens) => match crate::parser::parse(tokens) {
                                Ok(ast) => {
                                    exit_code = crate::executor::execute(ast, &mut temp_state);
                                }
                                Err(e) => {
                                    let _ = writeln!(output_writer, "Parse error: {}", e);
                                    return 1;
                                }
                            },
                            Err(e) => {
                                let _ = writeln!(output_writer, "Lex error: {}", e);
                                return 1;
                            }
                        }
                    }
                    exit_code
                }
                Err(e) => {
                    let _ = writeln!(output_writer, "source: {}: {}", script_file, e);
                    1
                }
            }
        }
        "export" => {
            if cmd.args.len() < 2 {
                // Print all exported variables
                for var_name in &shell_state.exported {
                    if let Some(value) = shell_state.variables.get(var_name) {
                        let _ = writeln!(output_writer, "export {}={}", var_name, value);
                    }
                }
                0
            } else {
                let arg = &cmd.args[1];
                if let Some(eq_pos) = arg.find('=') {
                    // export VAR=value
                    let var = arg[..eq_pos].to_string();
                    let value = arg[eq_pos + 1..].to_string();
                    shell_state.set_exported_var(&var, value);
                    0
                } else {
                    // export VAR (mark existing var as exported)
                    shell_state.export_var(arg);
                    0
                }
            }
        }
        "unset" => {
            if cmd.args.len() < 2 {
                let _ = writeln!(output_writer, "unset: missing variable name");
                1
            } else {
                shell_state.unset_var(&cmd.args[1]);
                0
            }
        }
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin() {
        assert!(is_builtin("cd"));
        assert!(is_builtin("echo"));
        assert!(is_builtin("pwd"));
        assert!(is_builtin("env"));
        assert!(is_builtin("exit"));
        assert!(is_builtin("help"));
        assert!(!is_builtin("ls"));
        assert!(!is_builtin("grep"));
    }

    #[test]
    fn test_execute_builtin_echo() {
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "hello".to_string(), "world".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_builtin_unknown() {
        let cmd = ShellCommand {
            args: vec!["unknown".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_get_builtin_commands() {
        let commands = get_builtin_commands();
        assert!(commands.contains(&"cd".to_string()));
        assert!(commands.contains(&"echo".to_string()));
        assert!(commands.contains(&"pwd".to_string()));
        assert!(commands.contains(&"env".to_string()));
        assert!(commands.contains(&"exit".to_string()));
        assert!(commands.contains(&"help".to_string()));
        assert!(commands.contains(&"source".to_string()));
        assert!(commands.contains(&"export".to_string()));
        assert!(commands.contains(&"unset".to_string()));
        assert_eq!(commands.len(), 9);
    }
}
