use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct UnaliasBuiltin;

impl super::Builtin for UnaliasBuiltin {
    fn name(&self) -> &'static str {
        "unalias"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Remove alias definitions. Use -a to remove all aliases."
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        let args = &cmd.args;
        if args.len() < 2 {
            let _ = writeln!(output_writer, "unalias: missing operand");
            1
        } else if args[1] == "-a" {
            if args.len() > 2 {
                let _ = writeln!(output_writer, "unalias: too many arguments");
                1
            } else {
                shell_state.aliases.clear();
                0
            }
        } else if args.len() > 2 {
            let _ = writeln!(output_writer, "unalias: too many arguments");
            1
        } else {
            let name = &args[1];
            if shell_state.get_alias(name).is_some() {
                shell_state.remove_alias(name);
                0
            } else {
                let _ = writeln!(output_writer, "unalias: {}: not found", name);
                1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;

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
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let builtin = UnaliasBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
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
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let builtin = UnaliasBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_execute_builtin_unalias_no_args() {
        let cmd = ShellCommand {
            args: vec!["unalias".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let builtin = UnaliasBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
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
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let builtin = UnaliasBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1);
    }
    #[test]
    fn test_execute_builtin_unalias_all() {
        let mut shell_state = crate::state::ShellState::new();
        shell_state.set_alias("test_alias1", "ls -l".to_string());
        shell_state.set_alias("test_alias2", "echo hi".to_string());

        // Verify aliases exist
        assert_eq!(shell_state.get_all_aliases().len(), 2);

        let cmd = ShellCommand {
            args: vec!["unalias".to_string(), "-a".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let builtin = UnaliasBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert!(output.is_empty());

        // Verify all aliases removed
        assert!(shell_state.get_all_aliases().is_empty());
    }

    #[test]
    fn test_execute_builtin_unalias_all_no_aliases() {
        let cmd = ShellCommand {
            args: vec!["unalias".to_string(), "-a".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let builtin = UnaliasBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert!(output.is_empty());

        // Verify still empty
        assert!(shell_state.get_all_aliases().is_empty());
    }

    #[test]
    fn test_execute_builtin_unalias_all_too_many_args() {
        let cmd = ShellCommand {
            args: vec!["unalias".to_string(), "-a".to_string(), "extra".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let builtin = UnaliasBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1);

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("too many arguments"));
    }
}
