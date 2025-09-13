use std::env;
use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct EnvBuiltin;

impl super::Builtin for EnvBuiltin {
    fn name(&self) -> &'static str {
        "env"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Print environment variables"
    }

    fn run(
        &self,
        _cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        // Show exported shell variables first
        for var_name in &shell_state.exported {
            if let Some(value) = shell_state.variables.get(var_name) {
                if shell_state.colors_enabled {
                    let _ = writeln!(
                        output_writer,
                        "{}{}{}={}",
                        shell_state.color_scheme.success, var_name, "\x1b[0m", value
                    );
                } else {
                    let _ = writeln!(output_writer, "{}={}", var_name, value);
                }
            }
        }
        // Then show environment variables (excluding those already shown)
        for (key, value) in env::vars() {
            if !shell_state.exported.contains(&key) {
                if shell_state.colors_enabled {
                    let _ = writeln!(
                        output_writer,
                        "{}{}{}={}",
                        shell_state.color_scheme.builtin, key, "\x1b[0m", value
                    );
                } else {
                    let _ = writeln!(output_writer, "{}={}", key, value);
                }
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
    fn test_env_builtin_run() {
        let cmd = ShellCommand {
            args: vec!["env".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_exported_var("TEST_VAR", "test_value".to_string());
        let builtin = EnvBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("TEST_VAR=test_value"));
    }
}
