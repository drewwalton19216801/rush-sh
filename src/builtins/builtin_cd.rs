use std::env;
use std::io::Write;
use std::path::Path;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct CdBuiltin;

impl super::Builtin for CdBuiltin {
    fn name(&self) -> &'static str {
        "cd"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Change directory"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        _shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        let dir = if cmd.args.len() > 1 {
            cmd.args[1].clone()
        } else {
            "~".to_string()
        };
        let path = if dir == "~" {
            env::var("HOME").unwrap_or_else(|_| "/".to_string())
        } else {
            dir
        };
        if let Err(e) = env::set_current_dir(Path::new(&path)) {
            let _ = writeln!(output_writer, "cd: {}: {}", path, e);
            1
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;
    use std::env;

    #[test]
    fn test_cd_to_valid_directory() {
        let original_dir = env::current_dir().unwrap();
        let cmd = ShellCommand {
            args: vec!["cd".to_string(), "/tmp".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = CdBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        // Restore original directory
        let _ = env::set_current_dir(&original_dir);
    }

    #[test]
    fn test_cd_to_invalid_directory() {
        let cmd = ShellCommand {
            args: vec!["cd".to_string(), "/nonexistent".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = CdBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1);
        assert!(output.len() > 0); // Should have error message
    }

    #[test]
    fn test_cd_no_arguments() {
        let cmd = ShellCommand {
            args: vec!["cd".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = CdBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
    }
}
