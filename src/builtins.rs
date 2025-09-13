use std::fs::File;
use std::io::{self, Write};

use crate::parser::ShellCommand;
use crate::state::ShellState;

mod builtin_alias;
mod builtin_bracket;
mod builtin_cd;
mod builtin_dirs;
mod builtin_dot;
mod builtin_env;
mod builtin_exit;
mod builtin_export;
mod builtin_help;
mod builtin_popd;
mod builtin_pushd;
mod builtin_pwd;
mod builtin_source;
mod builtin_test;
mod builtin_unalias;
mod builtin_unset;

pub trait Builtin {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32;
}

fn get_builtins() -> Vec<Box<dyn Builtin>> {
    vec![
        Box::new(builtin_cd::CdBuiltin),
        Box::new(builtin_pwd::PwdBuiltin),
        Box::new(builtin_env::EnvBuiltin),
        Box::new(builtin_exit::ExitBuiltin),
        Box::new(builtin_help::HelpBuiltin),
        Box::new(builtin_source::SourceBuiltin),
        Box::new(builtin_dot::DotBuiltin),
        Box::new(builtin_export::ExportBuiltin),
        Box::new(builtin_unset::UnsetBuiltin),
        Box::new(builtin_pushd::PushdBuiltin),
        Box::new(builtin_popd::PopdBuiltin),
        Box::new(builtin_dirs::DirsBuiltin),
        Box::new(builtin_alias::AliasBuiltin),
        Box::new(builtin_unalias::UnaliasBuiltin),
        Box::new(builtin_test::TestBuiltin),
        Box::new(builtin_bracket::BracketBuiltin),
    ]
}

pub fn is_builtin(cmd: &str) -> bool {
    get_builtins().iter().any(|b| b.name() == cmd)
}

pub fn get_builtin_commands() -> Vec<String> {
    get_builtins()
        .iter()
        .map(|b| b.name().to_string())
        .collect()
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

    let builtins = get_builtins();
    if let Some(builtin) = builtins.into_iter().find(|b| b.name() == cmd.args[0]) {
        builtin.run(cmd, shell_state, &mut *output_writer)
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin() {
        assert!(is_builtin("cd"));
        assert!(is_builtin("pwd"));
        assert!(is_builtin("env"));
        assert!(is_builtin("exit"));
        assert!(is_builtin("help"));
        assert!(is_builtin("alias"));
        assert!(is_builtin("unalias"));
        assert!(is_builtin("test"));
        assert!(is_builtin("["));
        assert!(is_builtin("."));
        assert!(!is_builtin("ls"));
        assert!(!is_builtin("grep"));
        assert!(!is_builtin("echo"));
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
        assert!(commands.contains(&"pwd".to_string()));
        assert!(commands.contains(&"env".to_string()));
        assert!(commands.contains(&"exit".to_string()));
        assert!(commands.contains(&"help".to_string()));
        assert!(commands.contains(&"source".to_string()));
        assert!(commands.contains(&"export".to_string()));
        assert!(commands.contains(&"unset".to_string()));
        assert!(commands.contains(&"pushd".to_string()));
        assert!(commands.contains(&"popd".to_string()));
        assert!(commands.contains(&"dirs".to_string()));
        assert!(commands.contains(&"alias".to_string()));
        assert!(commands.contains(&"unalias".to_string()));
        assert!(commands.contains(&"test".to_string()));
        assert!(commands.contains(&"[".to_string()));
        assert!(commands.contains(&".".to_string()));
        assert_eq!(commands.len(), 16);
    }

    #[test]
    fn test_execute_builtin_pushd() {
        let original_dir = std::env::current_dir().unwrap();
        let cmd = ShellCommand {
            args: vec!["pushd".to_string(), "/tmp".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 0);
        // Should have pushed original dir to stack
        assert_eq!(shell_state.dir_stack.len(), 1);
        assert_eq!(shell_state.dir_stack[0], original_dir.to_string_lossy());

        // Restore original directory for test cleanup
        let _ = std::env::set_current_dir(&original_dir);
    }

    #[test]
    fn test_execute_builtin_popd() {
        let mut shell_state = crate::state::ShellState::new();
        shell_state.dir_stack.push("/tmp".to_string());

        let cmd = ShellCommand {
            args: vec!["popd".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 0);
        // Should have popped from stack
        assert_eq!(shell_state.dir_stack.len(), 0);
        // Note: We don't test actual directory change as it may not work in test environment
    }

    #[test]
    fn test_execute_builtin_dirs() {
        let cmd = ShellCommand {
            args: vec!["dirs".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_builtin_alias_set() {
        let cmd = ShellCommand {
            args: vec!["alias".to_string(), "ll=ls -l".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_alias("ll"), Some(&"ls -l".to_string()));
    }

    #[test]
    fn test_execute_builtin_alias_list() {
        let cmd = ShellCommand {
            args: vec!["alias".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        shell_state.set_alias("ll", "ls -l".to_string());
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_builtin_alias_show() {
        let cmd = ShellCommand {
            args: vec!["alias".to_string(), "ll".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        shell_state.set_alias("ll", "ls -l".to_string());
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_builtin_alias_show_not_found() {
        let cmd = ShellCommand {
            args: vec!["alias".to_string(), "nonexistent".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_execute_builtin_unalias() {
        let mut shell_state = crate::state::ShellState::new();
        shell_state.set_alias("test_alias", "ls -l".to_string());

        // Verify alias exists
        assert_eq!(
            shell_state.get_alias("test_alias"),
            Some(&"ls -l".to_string())
        );

        // Remove the alias
        let cmd = ShellCommand {
            args: vec!["unalias".to_string(), "test_alias".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 0);

        // Verify alias is removed
        assert_eq!(shell_state.get_alias("test_alias"), None);
    }

    #[test]
    fn test_execute_builtin_unalias_not_found() {
        let cmd = ShellCommand {
            args: vec!["unalias".to_string(), "nonexistent".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_execute_builtin_unalias_no_args() {
        let cmd = ShellCommand {
            args: vec!["unalias".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_execute_builtin_unalias_too_many_args() {
        let cmd = ShellCommand {
            args: vec![
                "unalias".to_string(),
                "arg1".to_string(),
                "arg2".to_string(),
            ],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_execute_builtin_source_variable_sharing() {
        use std::fs;

        // Create a temporary script file
        let temp_script = "/tmp/test_source_vars.sh";
        let script_content = "TEST_VAR_FROM_SOURCE=shared_value\nANOTHER_VAR=another_value";
        fs::write(temp_script, script_content).unwrap();

        let cmd = ShellCommand {
            args: vec!["source".to_string(), temp_script.to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 0);

        // Verify that variables are now available in the shell state
        assert_eq!(shell_state.get_var("TEST_VAR_FROM_SOURCE"), Some("shared_value".to_string()));
        assert_eq!(shell_state.get_var("ANOTHER_VAR"), Some("another_value".to_string()));

        // Clean up
        fs::remove_file(temp_script).unwrap();
    }

    #[test]
    fn test_execute_builtin_dot_variable_sharing() {
        use std::fs;

        // Create a temporary script file
        let temp_script = "/tmp/test_dot_vars.sh";
        let script_content = "DOT_TEST_VAR=dot_shared\nDOT_VAR2=dot_value";
        fs::write(temp_script, script_content).unwrap();

        let cmd = ShellCommand {
            args: vec![".".to_string(), temp_script.to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 0);

        // Verify that variables are now available in the shell state
        assert_eq!(shell_state.get_var("DOT_TEST_VAR"), Some("dot_shared".to_string()));
        assert_eq!(shell_state.get_var("DOT_VAR2"), Some("dot_value".to_string()));

        // Clean up
        fs::remove_file(temp_script).unwrap();
    }
}
