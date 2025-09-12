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

pub struct PopdBuiltin;

impl super::Builtin for PopdBuiltin {
    fn name(&self) -> &'static str {
        "popd"
    }

    fn description(&self) -> &'static str {
        "Pop directory from stack and change to it"
    }

    fn run(&self, _cmd: &ShellCommand, shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        if shell_state.dir_stack.is_empty() {
            let _ = writeln!(output_writer, "popd: directory stack empty");
            1
        } else {
            // Pop directory from stack
            let dir = shell_state.dir_stack.pop().unwrap();
            // Change to that directory
            if let Err(e) = env::set_current_dir(Path::new(&dir)) {
                let _ = writeln!(output_writer, "popd: {}: {}", dir, e);
                1
            } else {
                // Print the stack
                print_dir_stack(shell_state, output_writer);
                0
            }
        }
    }
}
