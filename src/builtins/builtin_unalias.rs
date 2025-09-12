use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct UnaliasBuiltin;

impl super::Builtin for UnaliasBuiltin {
    fn name(&self) -> &'static str {
        "unalias"
    }

    fn description(&self) -> &'static str {
        "Remove alias definitions"
    }

    fn run(&self, cmd: &ShellCommand, shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        if cmd.args.len() < 2 {
            let _ = writeln!(output_writer, "unalias: missing alias name");
            1
        } else if cmd.args.len() > 2 {
            let _ = writeln!(output_writer, "unalias: too many arguments");
            1
        } else {
            let name = &cmd.args[1];
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
