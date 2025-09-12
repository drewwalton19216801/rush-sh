use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct EchoBuiltin;

impl super::Builtin for EchoBuiltin {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn description(&self) -> &'static str {
        "Print arguments"
    }

    fn run(&self, cmd: &ShellCommand, _shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        let output = cmd.args[1..].join(" ");
        let _ = writeln!(output_writer, "{}", output);
        0
    }
}
