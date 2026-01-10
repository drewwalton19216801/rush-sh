use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct UnsetBuiltin;

impl super::Builtin for UnsetBuiltin {
    fn name(&self) -> &'static str {
        "unset"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Unset shell variables"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        if cmd.args.len() < 2 {
            let _ = writeln!(output_writer, "unset: missing variable name");
            1
        } else {
            shell_state.unset_var(&cmd.args[1]);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;

    #[test]
    fn test_unset_builtin_unset_variable() {
        let cmd = ShellCommand {
            args: vec!["unset".to_string(), "TEST_VAR".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_var("TEST_VAR", "test_value".to_string());
        assert_eq!(
            shell_state.get_var("TEST_VAR"),
            Some("test_value".to_string())
        );
        let builtin = UnsetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("TEST_VAR"), None);
    }

    #[test]
    fn test_unset_builtin_no_args() {
        let cmd = ShellCommand {
            args: vec!["unset".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = UnsetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("unset: missing variable name"));
    }
}
