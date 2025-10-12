use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct AliasBuiltin;

impl super::Builtin for AliasBuiltin {
    fn name(&self) -> &'static str {
        "alias"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Define or display aliases"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        if cmd.args.len() == 1 {
            // List all aliases
            let aliases = shell_state.get_all_aliases();
            if aliases.is_empty() {
                let _ = writeln!(output_writer);
            } else {
                for (name, value) in aliases {
                    let _ = writeln!(output_writer, "alias {}='{}'", name, value);
                }
            }
            0
        } else if cmd.args.len() == 2 {
            let arg = &cmd.args[1];
            if let Some(eq_pos) = arg.find('=') {
                // Set alias: alias name=value
                let name = arg[..eq_pos].to_string();
                let value = arg[eq_pos + 1..].to_string();
                shell_state.set_alias(&name, value);
                0
            } else {
                // Show specific alias
                if let Some(value) = shell_state.get_alias(arg) {
                    let _ = writeln!(output_writer, "alias {}='{}'", arg, value);
                    0
                } else {
                    let _ = writeln!(output_writer, "alias: {}: not found", arg);
                    1
                }
            }
        } else {
            let _ = writeln!(output_writer, "alias: too many arguments");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;

    #[test]
    fn test_execute_builtin_alias_set() {
        let cmd = ShellCommand {
            args: vec!["alias".to_string(), "ll=ls -l".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let builtin = AliasBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
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
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        shell_state.set_alias("ll", "ls -l".to_string());
        let builtin = AliasBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_builtin_alias_show() {
        let cmd = ShellCommand {
            args: vec!["alias".to_string(), "ll".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        shell_state.set_alias("ll", "ls -l".to_string());
        let builtin = AliasBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_builtin_alias_show_not_found() {
        let cmd = ShellCommand {
            args: vec!["alias".to_string(), "nonexistent".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let builtin = AliasBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1);
    }
}
