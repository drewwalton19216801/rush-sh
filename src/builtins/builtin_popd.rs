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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;

    #[test]
    fn test_execute_builtin_popd() {
        let mut shell_state = crate::state::ShellState::new();
        shell_state.dir_stack.push("/tmp".to_string());

        let cmd = ShellCommand {
            args: vec!["popd".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let builtin = PopdBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        // Should have popped from stack
        assert_eq!(shell_state.dir_stack.len(), 0);
        // Note: We don't test actual directory change as it may not work in test environment
    }
}
