use std::env;
use std::io::Write;
use std::path::Path;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct CdBuiltin;

impl super::Builtin for CdBuiltin {
    fn name(&self) -> &'static str {
        "cd"
    }

    fn description(&self) -> &'static str {
        "Change directory"
    }

    fn run(&self, cmd: &ShellCommand, _shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        let dir = if cmd.args.len() > 1 {
            cmd.args[1].clone()
        } else {
            "~".to_string()
        };
        let path = if dir == "~" {
            env::var("HOME").unwrap_or_else(|_| "/".to_string())
        } else {
            dir
        };
        if let Err(e) = env::set_current_dir(Path::new(&path)) {
            let _ = writeln!(output_writer, "cd: {}: {}", path, e);
            1
        } else {
            0
        }
    }
}
