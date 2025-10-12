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

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Push directory onto stack and change to it"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;
    use std::sync::Mutex;

    // Import the DIR_CHANGE_LOCK from main.rs tests
    // Since we can't directly access it, we'll create our own for this module
    static DIR_CHANGE_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_execute_builtin_pushd() {
        // Lock to prevent parallel tests from interfering with directory changes
        let _lock = DIR_CHANGE_LOCK.lock().unwrap();

        // First, ensure we're in a safe directory that definitely exists
        std::env::set_current_dir("/tmp").unwrap();

        let original_dir = std::env::current_dir().unwrap();
        let cmd = ShellCommand {
            args: vec!["pushd".to_string(), "/tmp".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_string_content: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let builtin = PushdBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        // Should have pushed original dir to stack
        assert_eq!(shell_state.dir_stack.len(), 1);
        assert_eq!(shell_state.dir_stack[0], original_dir.to_string_lossy());

        // Restore original directory for test cleanup
        let _ = std::env::set_current_dir(&original_dir);
    }
}
