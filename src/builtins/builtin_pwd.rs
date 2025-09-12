use std::env;
use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct PwdBuiltin;

impl super::Builtin for PwdBuiltin {
    fn name(&self) -> &'static str {
        "pwd"
    }

    fn description(&self) -> &'static str {
        "Print working directory"
    }

    fn run(&self, _cmd: &ShellCommand, _shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        match env::current_dir() {
            Ok(path) => {
                let _ = writeln!(output_writer, "{}", path.display());
                0
            }
            Err(e) => {
                let _ = writeln!(output_writer, "pwd: {}", e);
                1
            }
        }
    }
}
