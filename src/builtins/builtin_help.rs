use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct HelpBuiltin;

impl super::Builtin for HelpBuiltin {
    fn name(&self) -> &'static str {
        "help"
    }

    fn description(&self) -> &'static str {
        "Show this help message"
    }

    fn run(&self, _cmd: &ShellCommand, _shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        // Attempt to write the header, handling potential errors
        if writeln!(output_writer, "Rush Shell v{}", env!("CARGO_PKG_VERSION")).is_err()
            || writeln!(output_writer, "").is_err()
            || writeln!(output_writer, "Available built-in commands:").is_err()
        {
            return 1; // Return error if header write fails
        }

        // Iterate over the complete builtins list with descriptions
        let builtins = super::get_builtins();
        for builtin in builtins {
            // Use explicit formatting for better readability
            let formatted_line = format!("  {:<12} {}", builtin.name(), builtin.description());
            if writeln!(output_writer, "{}", formatted_line).is_err() {
                return 1; // Return error if any command write fails
            }
        }
        0
    }
}
