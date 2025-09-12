use std::fs;
use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct BracketBuiltin;

impl super::Builtin for BracketBuiltin {
    fn name(&self) -> &'static str {
        "["
    }

    fn description(&self) -> &'static str {
        "Evaluate conditional expressions (same as test)"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        _shell_state: &mut ShellState,
        _output_writer: &mut dyn Write,
    ) -> i32 {
        // Skip the command name (args[0] is "[")
        let args = &cmd.args[1..];

        // Check if the last argument is "]"
        if args.is_empty() || args.last().unwrap() != "]" {
            return 2; // Invalid usage - missing closing bracket
        }

        // Remove the closing bracket from arguments
        let test_args = &args[..args.len() - 1];

        if test_args.is_empty() {
            // No arguments - false
            return 1;
        }

        // Parse the first argument as an option
        if let Some(option) = test_args[0].strip_prefix('-') {
            match option {
                "z" => {
                    // Test if string is empty
                    if test_args.len() < 2 {
                        return 2; // Invalid usage
                    }
                    return if test_args[1].is_empty() { 0 } else { 1 };
                }
                "n" => {
                    // Test if string is not empty
                    if test_args.len() < 2 {
                        return 2; // Invalid usage
                    }
                    return if !test_args[1].is_empty() { 0 } else { 1 };
                }
                "f" => {
                    // Test if file exists and is regular file
                    if test_args.len() < 2 {
                        return 2; // Invalid usage
                    }
                    match fs::metadata(&test_args[1]) {
                        Ok(metadata) => {
                            if metadata.is_file() {
                                0
                            } else {
                                1
                            }
                        }
                        Err(_) => 1, // File doesn't exist
                    }
                }
                "d" => {
                    // Test if file exists and is directory
                    if test_args.len() < 2 {
                        return 2; // Invalid usage
                    }
                    match fs::metadata(&test_args[1]) {
                        Ok(metadata) => {
                            if metadata.is_dir() {
                                0
                            } else {
                                1
                            }
                        }
                        Err(_) => 1, // File doesn't exist
                    }
                }
                "e" => {
                    // Test if file exists
                    if test_args.len() < 2 {
                        return 2; // Invalid usage
                    }
                    match fs::metadata(&test_args[1]) {
                        Ok(_) => 0,
                        Err(_) => 1,
                    }
                }
                _ => {
                    // Invalid option
                    return 2;
                }
            }
        } else {
            // Check for numeric comparison operators
            if test_args.len() >= 3 {
                if let Some(operator) = test_args[1].strip_prefix('-') {
                    match operator {
                        "eq" => {
                            let left = test_args[0].parse::<i32>();
                            let right = test_args[2].parse::<i32>();
                            match (left, right) {
                                (Ok(l), Ok(r)) => return if l == r { 0 } else { 1 },
                                _ => return 2, // Invalid numeric arguments
                            }
                        }
                        "ne" => {
                            let left = test_args[0].parse::<i32>();
                            let right = test_args[2].parse::<i32>();
                            match (left, right) {
                                (Ok(l), Ok(r)) => return if l != r { 0 } else { 1 },
                                _ => return 2, // Invalid numeric arguments
                            }
                        }
                        "lt" => {
                            let left = test_args[0].parse::<i32>();
                            let right = test_args[2].parse::<i32>();
                            match (left, right) {
                                (Ok(l), Ok(r)) => return if l < r { 0 } else { 1 },
                                _ => return 2, // Invalid numeric arguments
                            }
                        }
                        "le" => {
                            let left = test_args[0].parse::<i32>();
                            let right = test_args[2].parse::<i32>();
                            match (left, right) {
                                (Ok(l), Ok(r)) => return if l <= r { 0 } else { 1 },
                                _ => return 2, // Invalid numeric arguments
                            }
                        }
                        "gt" => {
                            let left = test_args[0].parse::<i32>();
                            let right = test_args[2].parse::<i32>();
                            match (left, right) {
                                (Ok(l), Ok(r)) => return if l > r { 0 } else { 1 },
                                _ => return 2, // Invalid numeric arguments
                            }
                        }
                        "ge" => {
                            let left = test_args[0].parse::<i32>();
                            let right = test_args[2].parse::<i32>();
                            match (left, right) {
                                (Ok(l), Ok(r)) => return if l >= r { 0 } else { 1 },
                                _ => return 2, // Invalid numeric arguments
                            }
                        }
                        _ => {
                            // Invalid operator
                            return 2;
                        }
                    }
                }
            }
            // No valid option or numeric comparison found
            return 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;
    use std::fs::File;

    #[test]
    fn test_bracket_builtin_name() {
        let builtin = BracketBuiltin;
        assert_eq!(builtin.name(), "[");
    }

    #[test]
    fn test_bracket_builtin_description() {
        let builtin = BracketBuiltin;
        assert_eq!(
            builtin.description(),
            "Evaluate conditional expressions (same as test)"
        );
    }

    #[test]
    fn test_bracket_z_option_empty_string() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "[".to_string(),
                "-z".to_string(),
                "".to_string(),
                "]".to_string(),
            ],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // Empty string should return true (0)
    }

    #[test]
    fn test_bracket_z_option_non_empty_string() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "[".to_string(),
                "-z".to_string(),
                "hello".to_string(),
                "]".to_string(),
            ],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // Non-empty string should return false (1)
    }

    #[test]
    fn test_bracket_missing_closing_bracket() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "-z".to_string(), "hello".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 2); // Missing closing bracket should return error (2)
    }

    #[test]
    fn test_bracket_z_option_missing_argument() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "-z".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 2); // Missing argument should return error (2)
    }

    #[test]
    fn test_bracket_n_option_empty_string() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "[".to_string(),
                "-n".to_string(),
                "".to_string(),
                "]".to_string(),
            ],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // Empty string should return false (1)
    }

    #[test]
    fn test_bracket_n_option_non_empty_string() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "[".to_string(),
                "-n".to_string(),
                "hello".to_string(),
                "]".to_string(),
            ],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // Non-empty string should return true (0)
    }

    #[test]
    fn test_bracket_e_option_existing_file() {
        let builtin = BracketBuiltin;
        // Create a temporary file
        let temp_path = "/tmp/test_bracket_file.txt";
        File::create(temp_path).unwrap();

        let cmd = ShellCommand {
            args: vec![
                "[".to_string(),
                "-e".to_string(),
                temp_path.to_string(),
                "]".to_string(),
            ],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // Existing file should return true (0)

        // Clean up
        std::fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn test_bracket_e_option_non_existing_file() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "[".to_string(),
                "-e".to_string(),
                "/non/existing/file".to_string(),
                "]".to_string(),
            ],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // Non-existing file should return false (1)
    }

    #[test]
    fn test_bracket_invalid_option() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "[".to_string(),
                "-x".to_string(),
                "arg".to_string(),
                "]".to_string(),
            ],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 2); // Invalid option should return error (2)
    }

    #[test]
    fn test_bracket_no_arguments() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // No arguments should return false (1)
    }

    #[test]
    fn test_bracket_eq_equal() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "2".to_string(), "-eq".to_string(), "2".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // 2 == 2 should return true (0)
    }

    #[test]
    fn test_bracket_eq_not_equal() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "2".to_string(), "-eq".to_string(), "3".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // 2 == 3 should return false (1)
    }

    #[test]
    fn test_bracket_ne_equal() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "2".to_string(), "-ne".to_string(), "2".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // 2 != 2 should return false (1)
    }

    #[test]
    fn test_bracket_ne_not_equal() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "2".to_string(), "-ne".to_string(), "3".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // 2 != 3 should return true (0)
    }

    #[test]
    fn test_bracket_lt_less() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "2".to_string(), "-lt".to_string(), "3".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // 2 < 3 should return true (0)
    }

    #[test]
    fn test_bracket_lt_greater() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "3".to_string(), "-lt".to_string(), "2".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // 3 < 2 should return false (1)
    }

    #[test]
    fn test_bracket_le_less() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "2".to_string(), "-le".to_string(), "3".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // 2 <= 3 should return true (0)
    }

    #[test]
    fn test_bracket_le_equal() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "2".to_string(), "-le".to_string(), "2".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // 2 <= 2 should return true (0)
    }

    #[test]
    fn test_bracket_le_greater() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "3".to_string(), "-le".to_string(), "2".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // 3 <= 2 should return false (1)
    }

    #[test]
    fn test_bracket_gt_greater() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "3".to_string(), "-gt".to_string(), "2".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // 3 > 2 should return true (0)
    }

    #[test]
    fn test_bracket_gt_less() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "2".to_string(), "-gt".to_string(), "3".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // 2 > 3 should return false (1)
    }

    #[test]
    fn test_bracket_ge_greater() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "3".to_string(), "-ge".to_string(), "2".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // 3 >= 2 should return true (0)
    }

    #[test]
    fn test_bracket_ge_equal() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "2".to_string(), "-ge".to_string(), "2".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // 2 >= 2 should return true (0)
    }

    #[test]
    fn test_bracket_ge_less() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "2".to_string(), "-ge".to_string(), "3".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // 2 >= 3 should return false (1)
    }

    #[test]
    fn test_bracket_numeric_invalid_left_operand() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "abc".to_string(), "-eq".to_string(), "2".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 2); // Invalid left operand should return error (2)
    }

    #[test]
    fn test_bracket_numeric_invalid_right_operand() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "2".to_string(), "-eq".to_string(), "abc".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 2); // Invalid right operand should return error (2)
    }

    #[test]
    fn test_bracket_numeric_invalid_operator() {
        let builtin = BracketBuiltin;
        let cmd = ShellCommand {
            args: vec!["[".to_string(), "2".to_string(), "-invalid".to_string(), "3".to_string(), "]".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 2); // Invalid operator should return error (2)
    }
}
