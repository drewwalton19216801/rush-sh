use std::env;
use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct EnvBuiltin;

impl super::Builtin for EnvBuiltin {
    fn name(&self) -> &'static str {
        "env"
    }

    fn description(&self) -> &'static str {
        "Print environment variables"
    }

    fn run(&self, _cmd: &ShellCommand, shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        // Show exported shell variables first
        for var_name in &shell_state.exported {
            if let Some(value) = shell_state.variables.get(var_name) {
                let _ = writeln!(output_writer, "{}={}", var_name, value);
            }
        }
        // Then show environment variables (excluding those already shown)
        for (key, value) in env::vars() {
            if !shell_state.exported.contains(&key) {
                let _ = writeln!(output_writer, "{}={}", key, value);
            }
        }
        0
    }
}
