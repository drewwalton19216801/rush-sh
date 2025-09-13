use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct HelpBuiltin;

impl super::Builtin for HelpBuiltin {
    fn name(&self) -> &'static str {
        "help"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;

    #[test]
    fn test_help_builtin_run() {
        let cmd = ShellCommand {
            args: vec!["help".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = HelpBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Rush Shell"));
        assert!(output_str.contains("Available built-in commands"));
        assert!(output_str.contains("help"));
    }
}
