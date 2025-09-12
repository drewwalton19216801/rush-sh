use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct AliasBuiltin;

impl super::Builtin for AliasBuiltin {
    fn name(&self) -> &'static str {
        "alias"
    }

    fn description(&self) -> &'static str {
        "Define or display aliases"
    }

    fn run(&self, cmd: &ShellCommand, shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
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
