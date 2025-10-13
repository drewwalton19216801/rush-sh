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

    fn run(
        &self,
        _cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        // Write header with color if enabled
        let header = format!("Rush Shell v{}", env!("CARGO_PKG_VERSION"));
        if shell_state.colors_enabled {
            if writeln!(
                output_writer,
                "{}{}\x1b[0m",
                shell_state.color_scheme.success, header
            )
            .is_err()
                || writeln!(output_writer).is_err()
                || writeln!(
                    output_writer,
                    "{}Available built-in commands:\x1b[0m",
                    shell_state.color_scheme.builtin
                )
                .is_err()
            {
                return 1;
            }
        } else if writeln!(output_writer, "{}", header).is_err()
            || writeln!(output_writer).is_err()
            || writeln!(output_writer, "Available built-in commands:").is_err()
        {
            return 1;
        }

        // Iterate over builtins with color if enabled
        let builtins = super::get_builtins();
        for builtin in builtins {
            let formatted_line = format!("  {:<12} {}", builtin.name(), builtin.description());
            if shell_state.colors_enabled {
                if writeln!(
                    output_writer,
                    "{}{}\x1b[0m",
                    shell_state.color_scheme.builtin, formatted_line
                )
                .is_err()
                {
                    return 1;
                }
            } else if writeln!(output_writer, "{}", formatted_line).is_err() {
                return 1;
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
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
            fd_redirections: vec![],
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
