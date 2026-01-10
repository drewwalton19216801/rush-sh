use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct ExitBuiltin;

impl super::Builtin for ExitBuiltin {
    fn name(&self) -> &'static str {
        "exit"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Exit the shell"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        _output_writer: &mut dyn Write,
    ) -> i32 {
        // Parse exit code from arguments
        let exit_code = if cmd.args.len() > 1 {
            cmd.args[1].parse::<i32>().unwrap_or(0)
        } else {
            // Use last exit code if no argument provided
            shell_state.last_exit_code
        };

        // Set exit flag and code
        shell_state.exit_requested = true;
        shell_state.exit_code = exit_code;

        exit_code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;

    #[test]
    fn test_exit_builtin_run() {
        let cmd = ShellCommand {
            args: vec!["exit".to_string()],
            redirections: Vec::new(),
        };
        let mut shell_state = ShellState::new();
        let builtin = ExitBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
    }
}
