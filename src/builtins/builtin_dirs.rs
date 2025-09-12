use std::io::Write;

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

pub struct DirsBuiltin;

impl super::Builtin for DirsBuiltin {
    fn name(&self) -> &'static str {
        "dirs"
    }

    fn description(&self) -> &'static str {
        "Display directory stack"
    }

    fn run(&self, _cmd: &ShellCommand, shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        print_dir_stack(shell_state, output_writer);
        0
    }
}
