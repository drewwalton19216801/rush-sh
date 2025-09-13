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
        _cmd: &ShellCommand,
        _shell_state: &mut ShellState,
        _output_writer: &mut dyn Write,
    ) -> i32 {
        // For now, just return 0; main handles exit
        0
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
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = ExitBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
    }
}
