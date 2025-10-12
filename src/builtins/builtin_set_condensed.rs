use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct SetCondensedBuiltin;

impl super::Builtin for SetCondensedBuiltin {
    fn name(&self) -> &'static str {
        "set_condensed"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Enable or disable condensed cwd display in prompt (set_condensed on|off|status)"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        if cmd.args.len() < 2 {
            if shell_state.condensed_cwd {
                if shell_state.colors_enabled {
                    writeln!(
                        output_writer,
                        "{}Condensed cwd display is enabled\x1b[0m",
                        shell_state.color_scheme.success
                    )
                    .unwrap_or(());
                } else {
                    writeln!(output_writer, "Condensed cwd display is enabled").unwrap_or(());
                }
            } else {
                writeln!(output_writer, "Condensed cwd display is disabled").unwrap_or(());
            }
            return 0;
        }

        match cmd.args[1].as_str() {
            "on" | "enable" | "true" => {
                shell_state.condensed_cwd = true;
                if shell_state.colors_enabled {
                    writeln!(
                        output_writer,
                        "{}Condensed cwd display enabled\x1b[0m",
                        shell_state.color_scheme.success
                    )
                    .unwrap_or(());
                } else {
                    writeln!(output_writer, "Condensed cwd display enabled").unwrap_or(());
                }
            }
            "off" | "disable" | "false" => {
                shell_state.condensed_cwd = false;
                writeln!(output_writer, "Condensed cwd display disabled").unwrap_or(());
            }
            "status" => {
                if shell_state.condensed_cwd {
                    if shell_state.colors_enabled {
                        writeln!(
                            output_writer,
                            "{}Condensed cwd display is enabled\x1b[0m",
                            shell_state.color_scheme.success
                        )
                        .unwrap_or(());
                    } else {
                        writeln!(output_writer, "Condensed cwd display is enabled").unwrap_or(());
                    }
                } else {
                    writeln!(output_writer, "Condensed cwd display is disabled").unwrap_or(());
                }
            }
            _ => {
                if shell_state.colors_enabled {
                    writeln!(
                        output_writer,
                        "{}Usage: set_condensed on|off|status\x1b[0m",
                        shell_state.color_scheme.error
                    )
                    .unwrap_or(());
                } else {
                    writeln!(output_writer, "Usage: set_condensed on|off|status").unwrap_or(());
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
    fn test_set_condensed_builtin_enable() {
        let cmd = ShellCommand {
            args: vec!["set_condensed".to_string(), "on".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = SetCondensedBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert!(shell_state.condensed_cwd);
    }

    #[test]
    fn test_set_condensed_builtin_disable() {
        let cmd = ShellCommand {
            args: vec!["set_condensed".to_string(), "off".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = SetCondensedBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert!(!shell_state.condensed_cwd);
    }

    #[test]
    fn test_set_condensed_builtin_status() {
        let cmd = ShellCommand {
            args: vec!["set_condensed".to_string(), "status".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = SetCondensedBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("enabled")); // Should show current status
    }
}
