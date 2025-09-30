use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct ShiftBuiltin;

impl super::Builtin for ShiftBuiltin {
    fn name(&self) -> &'static str {
        "shift"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Shift positional parameters"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        let shift_count = if cmd.args.len() > 1 {
            match cmd.args[1].parse::<usize>() {
                Ok(n) => n,
                Err(_) => {
                    let _ = writeln!(output_writer, "shift: invalid number: {}", cmd.args[1]);
                    return 1;
                }
            }
        } else {
            1 // Default shift count is 1
        };

        shell_state.shift_positional_params(shift_count);
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;

    #[test]
    fn test_shift_builtin_default() {
        let cmd = ShellCommand {
            args: vec!["shift".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_positional_params(vec![
            "arg1".to_string(),
            "arg2".to_string(),
            "arg3".to_string(),
        ]);

        assert_eq!(shell_state.get_positional_params().len(), 3);
        assert_eq!(shell_state.get_var("1"), Some("arg1".to_string()));
        assert_eq!(shell_state.get_var("2"), Some("arg2".to_string()));
        assert_eq!(shell_state.get_var("3"), Some("arg3".to_string()));

        let builtin = ShiftBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);

        assert_eq!(shell_state.get_positional_params().len(), 2);
        assert_eq!(shell_state.get_var("1"), Some("arg2".to_string()));
        assert_eq!(shell_state.get_var("2"), Some("arg3".to_string()));
        assert_eq!(shell_state.get_var("3"), None);
    }

    #[test]
    fn test_shift_builtin_custom_count() {
        let cmd = ShellCommand {
            args: vec!["shift".to_string(), "2".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_positional_params(vec![
            "arg1".to_string(),
            "arg2".to_string(),
            "arg3".to_string(),
            "arg4".to_string(),
        ]);

        assert_eq!(shell_state.get_positional_params().len(), 4);
        assert_eq!(shell_state.get_var("1"), Some("arg1".to_string()));
        assert_eq!(shell_state.get_var("2"), Some("arg2".to_string()));
        assert_eq!(shell_state.get_var("3"), Some("arg3".to_string()));
        assert_eq!(shell_state.get_var("4"), Some("arg4".to_string()));

        let builtin = ShiftBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);

        assert_eq!(shell_state.get_positional_params().len(), 2);
        assert_eq!(shell_state.get_var("1"), Some("arg3".to_string()));
        assert_eq!(shell_state.get_var("2"), Some("arg4".to_string()));
        assert_eq!(shell_state.get_var("3"), None);
        assert_eq!(shell_state.get_var("4"), None);
    }

    #[test]
    fn test_shift_builtin_invalid_number() {
        let cmd = ShellCommand {
            args: vec!["shift".to_string(), "invalid".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = ShiftBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("shift: invalid number: invalid"));
    }

    #[test]
    fn test_shift_builtin_no_args() {
        let cmd = ShellCommand {
            args: vec!["shift".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_positional_params(vec!["arg1".to_string()]);

        assert_eq!(shell_state.get_positional_params().len(), 1);
        assert_eq!(shell_state.get_var("1"), Some("arg1".to_string()));

        let builtin = ShiftBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);

        assert_eq!(shell_state.get_positional_params().len(), 0);
        assert_eq!(shell_state.get_var("1"), None);
    }
}
