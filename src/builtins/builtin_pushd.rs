use std::env;
use std::io::Write;
use std::path::Path;

use crate::parser::ShellCommand;
use crate::state::ShellState;

fn print_dir_stack(shell_state: &ShellState, writer: &mut dyn Write) {
    // Get current directory
    if let Ok(current) = std::env::current_dir() {
        let current_str = current.to_string_lossy().to_string();
        // Print current directory first
        let _ = write!(writer, "{}", current_str);
        // Then print stack in reverse order (top of stack first)
        for dir in shell_state.dir_stack.iter().rev() {
            let _ = write!(writer, " {}", dir);
        }
        let _ = writeln!(writer);
    }
}

pub struct PushdBuiltin;

impl super::Builtin for PushdBuiltin {
    fn name(&self) -> &'static str {
        "pushd"
    }

    fn description(&self) -> &'static str {
        "Push directory onto stack and change to it"
    }

    fn run(&self, cmd: &ShellCommand, shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        if cmd.args.len() < 2 {
            let _ = writeln!(output_writer, "pushd: missing directory operand");
            1
        } else {
            let dir = &cmd.args[1];
            let path = if dir == "~" {
                env::var("HOME").unwrap_or_else(|_| "/".to_string())
            } else {
                dir.clone()
            };

            // Get current directory before changing
            let current_dir = match env::current_dir() {
                Ok(path) => path.to_string_lossy().to_string(),
                Err(e) => {
                    let _ = writeln!(
                        output_writer,
                        "pushd: failed to get current directory: {}",
                        e
                    );
                    return 1;
                }
            };

            // Change to new directory
            if let Err(e) = env::set_current_dir(Path::new(&path)) {
                let _ = writeln!(output_writer, "pushd: {}: {}", path, e);
                1
            } else {
                // Push old directory to stack
                shell_state.dir_stack.push(current_dir);
                // Print the stack
                print_dir_stack(shell_state, output_writer);
                0
            }
        }
    }
}
