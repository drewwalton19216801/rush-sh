use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

/// The no-op (`:`) builtin command.
///
/// The colon utility does nothing and always returns exit status 0.
/// It is primarily used for:
/// - Placeholder in control structures (e.g., `if :; then ...`)
/// - Forcing variable expansion without executing commands (e.g., `: ${VAR:?error}`)
/// - Creating infinite loops (e.g., `while :; do ...`)
/// - Consuming arguments that should be expanded but not used
///
/// # POSIX Compliance
///
/// The colon (`:`) is a POSIX special built-in command per IEEE Std 1003.1.
/// It accepts any number of arguments, which are subject to normal shell 
/// expansions (variable expansion, command substitution, etc.), but the 
/// arguments are then ignored. The command always exits with status 0.
///
/// # Examples
///
/// ```sh
/// # Basic usage - does nothing, returns 0
/// :
///
/// # With arguments - arguments are expanded but ignored
/// : "This is ignored"
/// : $VAR $(command)
///
/// # Force parameter expansion with error checking
/// : ${VAR:?Variable VAR is not set}
///
/// # Placeholder in if statement
/// if :; then
///     echo "This always executes"
/// fi
///
/// # Infinite loop
/// while :; do
///     echo "Press Ctrl+C to stop"
///     sleep 1
/// done
///
/// # With redirections - redirections are still applied
/// : > file.txt  # Creates or truncates file.txt
/// ```
pub struct ColonBuiltin;

impl super::Builtin for ColonBuiltin {
    fn name(&self) -> &'static str {
        ":"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Do nothing, successfully (POSIX no-op)"
    }

    fn run(
        &self,
        _cmd: &ShellCommand,
        _shell_state: &mut ShellState,
        _output_writer: &mut dyn Write,
    ) -> i32 {
        // The colon utility does nothing and always returns exit status 0.
        // Arguments are already expanded by the shell before reaching this point,
        // so we simply ignore them and return success.
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;
    use crate::parser::Redirection;

    #[test]
    fn test_colon_builtin_no_args() {
        let cmd = ShellCommand {
            args: vec![":".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = ColonBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert_eq!(output.len(), 0); // No output should be produced
    }

    #[test]
    fn test_colon_builtin_with_string_args() {
        let cmd = ShellCommand {
            args: vec![
                ":".to_string(),
                "arg1".to_string(),
                "arg2".to_string(),
                "arg3".to_string(),
            ],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = ColonBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert_eq!(output.len(), 0); // No output should be produced
    }

    #[test]
    fn test_colon_builtin_with_special_chars() {
        let cmd = ShellCommand {
            args: vec![
                ":".to_string(),
                "!@#$%^&*()".to_string(),
                "spaces here".to_string(),
                "tabs\there".to_string(),
            ],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = ColonBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert_eq!(output.len(), 0);
    }

    #[test]
    fn test_colon_builtin_with_numbers() {
        let cmd = ShellCommand {
            args: vec![
                ":".to_string(),
                "123".to_string(),
                "456".to_string(),
                "-789".to_string(),
            ],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = ColonBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert_eq!(output.len(), 0);
    }

    #[test]
    fn test_colon_builtin_always_returns_zero() {
        // Test multiple invocations to ensure it always returns 0
        let builtin = ColonBuiltin;
        let mut shell_state = ShellState::new();

        for _ in 0..10 {
            let cmd = ShellCommand {
                args: vec![":".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
            assert_eq!(exit_code, 0);
        }
    }

    #[test]
    fn test_colon_builtin_with_expanded_variables() {
        // Note: Variable expansion happens before the builtin is called,
        // so we test that the builtin handles already-expanded arguments
        let mut shell_state = ShellState::new();
        shell_state.set_var("TEST_VAR", "test_value".to_string());

        let cmd = ShellCommand {
            args: vec![
                ":".to_string(),
                "test_value".to_string(), // This would be the result of $TEST_VAR expansion
            ],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = ColonBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert_eq!(output.len(), 0);
    }

    #[test]
    fn test_colon_builtin_with_redirections() {
        // Test that the builtin works with redirections
        // (The actual redirection handling is done by execute_builtin, not the builtin itself)
        let cmd = ShellCommand {
            args: vec![":".to_string()],
            redirections: vec![Redirection::Output("/tmp/test_colon_output.txt".to_string())],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let builtin = ColonBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        // The builtin itself doesn't handle redirections, so output should still be empty
        assert_eq!(output.len(), 0);
    }

    #[test]
    fn test_colon_builtin_name() {
        let builtin = ColonBuiltin;
        assert_eq!(builtin.name(), ":");
        assert_eq!(builtin.names(), vec![":"]);
    }

    #[test]
    fn test_colon_builtin_description() {
        let builtin = ColonBuiltin;
        assert!(!builtin.description().is_empty());
        assert!(builtin.description().contains("no-op") || builtin.description().contains("nothing"));
    }

    #[test]
    fn test_colon_builtin_with_empty_strings() {
        let cmd = ShellCommand {
            args: vec![
                ":".to_string(),
                "".to_string(),
                "".to_string(),
            ],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = ColonBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert_eq!(output.len(), 0);
    }

    #[test]
    fn test_colon_builtin_with_very_long_args() {
        let long_string = "a".repeat(10000);
        let cmd = ShellCommand {
            args: vec![
                ":".to_string(),
                long_string.clone(),
                long_string.clone(),
            ],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = ColonBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert_eq!(output.len(), 0);
    }
}
