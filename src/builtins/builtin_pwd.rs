use std::env;
use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct PwdBuiltin;

impl super::Builtin for PwdBuiltin {
    fn name(&self) -> &'static str {
        "pwd"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Print working directory"
    }

    fn run(
        &self,
        _cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        match env::current_dir() {
            Ok(path) => {
                if shell_state.colors_enabled {
                    let _ = writeln!(
                        output_writer,
                        "{}{}\x1b[0m",
                        shell_state.color_scheme.directory,
                        path.display()
                    );
                } else {
                    let _ = writeln!(output_writer, "{}", path.display());
                }
                0
            }
            Err(e) => {
                if shell_state.colors_enabled {
                    let _ = writeln!(
                        output_writer,
                        "{}pwd: {}\x1b[0m",
                        shell_state.color_scheme.error,
                        e
                    );
                } else {
                    let _ = writeln!(output_writer, "pwd: {}", e);
                }
                1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;

    #[test]
    fn test_pwd_builtin_run() {
        let cmd = ShellCommand {
            args: vec!["pwd".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = PwdBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("/")); // Should contain a path separator
    }
}
