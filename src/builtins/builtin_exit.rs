use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct ExitBuiltin;

impl super::Builtin for ExitBuiltin {
    fn name(&self) -> &'static str {
        "exit"
    }

    fn description(&self) -> &'static str {
        "Exit the shell"
    }

    fn run(&self, _cmd: &ShellCommand, _shell_state: &mut ShellState, _output_writer: &mut dyn Write) -> i32 {
        // For now, just return 0; main handles exit
        0
    }
}
