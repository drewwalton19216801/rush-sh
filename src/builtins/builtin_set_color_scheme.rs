use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::{ColorScheme, ShellState};

pub struct SetColorSchemeBuiltin;

impl super::Builtin for SetColorSchemeBuiltin {
    fn name(&self) -> &'static str {
        "set_color_scheme"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Set the color scheme (set_color_scheme default|dark|light)"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        if cmd.args.len() < 2 {
            if shell_state.colors_enabled {
                writeln!(
                    output_writer,
                    "{}{}{}",
                    shell_state.color_scheme.success, "Current color scheme: default", "\x1b[0m"
                )
                .unwrap_or(());
            } else {
                writeln!(output_writer, "Current color scheme: default").unwrap_or(());
            }
            return 0;
        }

        match cmd.args[1].as_str() {
            "default" => {
                shell_state.color_scheme = ColorScheme::default();
                if shell_state.colors_enabled {
                    writeln!(
                        output_writer,
                        "{}{}{}",
                        shell_state.color_scheme.success, "Color scheme set to default", "\x1b[0m"
                    )
                    .unwrap_or(());
                } else {
                    writeln!(output_writer, "Color scheme set to default").unwrap_or(());
                }
            }
            "dark" => {
                // Dark theme: brighter colors for dark backgrounds
                shell_state.color_scheme = ColorScheme {
                    prompt: "\x1b[92m".to_string(),     // Bright green
                    error: "\x1b[91m".to_string(),      // Bright red
                    success: "\x1b[92m".to_string(),    // Bright green
                    builtin: "\x1b[96m".to_string(),    // Bright cyan
                    directory: "\x1b[94m".to_string(),  // Bright blue
                };
                if shell_state.colors_enabled {
                    writeln!(
                        output_writer,
                        "{}{}{}",
                        shell_state.color_scheme.success, "Color scheme set to dark", "\x1b[0m"
                    )
                    .unwrap_or(());
                } else {
                    writeln!(output_writer, "Color scheme set to dark").unwrap_or(());
                }
            }
            "light" => {
                // Light theme: darker colors for light backgrounds
                shell_state.color_scheme = ColorScheme {
                    prompt: "\x1b[32m".to_string(),     // Dark green
                    error: "\x1b[31m".to_string(),      // Dark red
                    success: "\x1b[32m".to_string(),    // Dark green
                    builtin: "\x1b[36m".to_string(),    // Dark cyan
                    directory: "\x1b[34m".to_string(),  // Dark blue
                };
                if shell_state.colors_enabled {
                    writeln!(
                        output_writer,
                        "{}{}{}",
                        shell_state.color_scheme.success, "Color scheme set to light", "\x1b[0m"
                    )
                    .unwrap_or(());
                } else {
                    writeln!(output_writer, "Color scheme set to light").unwrap_or(());
                }
            }
            _ => {
                if shell_state.colors_enabled {
                    writeln!(
                        output_writer,
                        "{}{}{}",
                        shell_state.color_scheme.error,
                        "Available schemes: default, dark, light",
                        "\x1b[0m"
                    )
                    .unwrap_or(());
                } else {
                    writeln!(output_writer, "Available schemes: default, dark, light")
                        .unwrap_or(());
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
    fn test_set_color_scheme_builtin_default() {
        let cmd = ShellCommand {
            args: vec!["set_color_scheme".to_string(), "default".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = SetColorSchemeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        // Check that color scheme was set to default
        assert_eq!(shell_state.color_scheme.prompt, "\x1b[32m");
    }

    #[test]
    fn test_set_color_scheme_builtin_dark() {
        let cmd = ShellCommand {
            args: vec!["set_color_scheme".to_string(), "dark".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = SetColorSchemeBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        // Check that color scheme was set to dark (bright colors)
        assert_eq!(shell_state.color_scheme.prompt, "\x1b[92m");
    }
}
