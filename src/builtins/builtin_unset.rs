use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct UnsetBuiltin;

impl super::Builtin for UnsetBuiltin {
    fn name(&self) -> &'static str {
        "unset"
    }

    fn description(&self) -> &'static str {
        "Unset shell variables"
    }

    fn run(&self, cmd: &ShellCommand, shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        if cmd.args.len() < 2 {
            let _ = writeln!(output_writer, "unset: missing variable name");
            1
        } else {
            shell_state.unset_var(&cmd.args[1]);
            0
        }
    }
}
