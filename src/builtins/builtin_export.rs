use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct ExportBuiltin;

impl super::Builtin for ExportBuiltin {
    fn name(&self) -> &'static str {
        "export"
    }

    fn description(&self) -> &'static str {
        "Export variables to environment"
    }

    fn run(&self, cmd: &ShellCommand, shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        if cmd.args.len() < 2 {
            // Print all exported variables
            for var_name in &shell_state.exported {
                if let Some(value) = shell_state.variables.get(var_name) {
                    let _ = writeln!(output_writer, "export {}={}", var_name, value);
                }
            }
            0
        } else {
            let arg = &cmd.args[1];
            if let Some(eq_pos) = arg.find('=') {
                // export VAR=value
                let var = arg[..eq_pos].to_string();
                let value = arg[eq_pos + 1..].to_string();
                shell_state.set_exported_var(&var, value);
                0
            } else {
                // export VAR (mark existing var as exported)
                shell_state.export_var(arg);
                0
            }
        }
    }
}
