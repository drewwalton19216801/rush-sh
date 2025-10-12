use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct SetColorsBuiltin;

impl super::Builtin for SetColorsBuiltin {
    fn name(&self) -> &'static str {
        "set_colors"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Enable or disable colored output (set_colors on|off)"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        if cmd.args.len() < 2 {
            if shell_state.colors_enabled {
                writeln!(output_writer, "Colors are currently enabled").unwrap_or(());
            } else {
                writeln!(output_writer, "Colors are currently disabled").unwrap_or(());
            }
            return 0;
        }

        match cmd.args[1].as_str() {
            "on" | "enable" | "true" => {
                shell_state.colors_enabled = true;
                if shell_state.colors_enabled {
                    writeln!(
                        output_writer,
                        "{}Colors enabled\x1b[0m",
                        shell_state.color_scheme.success
                    )
                    .unwrap_or(());
                } else {
                    writeln!(output_writer, "Colors enabled").unwrap_or(());
                }
            }
            "off" | "disable" | "false" => {
                shell_state.colors_enabled = false;
                writeln!(output_writer, "Colors disabled").unwrap_or(());
            }
            "status" => {
                if shell_state.colors_enabled {
                    if shell_state.colors_enabled {
                        writeln!(
                            output_writer,
                            "{}Colors are enabled\x1b[0m",
                            shell_state.color_scheme.success
                        )
                        .unwrap_or(());
                    } else {
                        writeln!(output_writer, "Colors are enabled").unwrap_or(());
                    }
                } else {
                    writeln!(output_writer, "Colors are disabled").unwrap_or(());
                }
            }
            _ => {
                if shell_state.colors_enabled {
                    writeln!(
                        output_writer,
                        "{}Usage: set_colors on|off|status\x1b[0m",
                        shell_state.color_scheme.error
                    )
                    .unwrap_or(());
                } else {
                    writeln!(output_writer, "Usage: set_colors on|off|status").unwrap_or(());
                }
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
    fn test_set_colors_builtin_enable() {
        let cmd = ShellCommand {
            args: vec!["set_colors".to_string(), "on".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = SetColorsBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert!(shell_state.colors_enabled);
    }

    #[test]
    fn test_set_colors_builtin_disable() {
        let cmd = ShellCommand {
            args: vec!["set_colors".to_string(), "off".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = SetColorsBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert!(!shell_state.colors_enabled);
    }
}
