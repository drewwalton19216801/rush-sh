use std::fs;
use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct TestBuiltin;

impl super::Builtin for TestBuiltin {
    fn name(&self) -> &'static str {
        "test"
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["test", "["]
    }

    fn description(&self) -> &'static str {
        "Evaluate conditional expressions"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        _shell_state: &mut ShellState,
        _output_writer: &mut dyn Write,
    ) -> i32 {
        // Handle both "test" and "[" commands
        let is_bracket = cmd.args[0] == "[";
        let args = if is_bracket {
            // For "[", skip the command name and expect closing "]"
            if cmd.args.len() < 2 || cmd.args.last().unwrap() != "]" {
                return 2; // Invalid usage - missing closing bracket
            }
            &cmd.args[1..cmd.args.len() - 1] // Skip "[" and "]"
        } else {
            // For "test", skip the command name
            &cmd.args[1..]
        };

        if args.is_empty() {
            // No arguments - false
            return 1;
        }

        // Parse the first argument as an option
        if let Some(option) = args[0].strip_prefix('-') {
            match option {
                "z" => {
                    // Test if string is empty
                    if args.len() < 2 {
                        return 2; // Invalid usage
                    }
                    if args[1].is_empty() { 0 } else { 1 }
                }
                "n" => {
                    // Test if string is not empty
                    if args.len() < 2 {
                        return 2; // Invalid usage
                    }
                    if !args[1].is_empty() { 0 } else { 1 }
                }
                "f" => {
                    // Test if file exists and is regular file
                    if args.len() < 2 {
                        return 2; // Invalid usage
                    }
                    match fs::metadata(&args[1]) {
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
                    if args.len() < 2 {
                        return 2; // Invalid usage
                    }
                    match fs::metadata(&args[1]) {
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
                    if args.len() < 2 {
                        return 2; // Invalid usage
                    }
                    match fs::metadata(&args[1]) {
                        Ok(_) => 0,
                        Err(_) => 1,
                    }
                }
                _ => {
                    // Invalid option
                    2
                }
            }
        } else {
            // Check for string or numeric comparison operators
            if args.len() >= 3 {
                // Check for string comparison operators first (=, !=)
                if args[1] == "=" {
                    return if args[0] == args[2] { 0 } else { 1 };
                } else if args[1] == "!=" {
                    return if args[0] != args[2] { 0 } else { 1 };
                }
                
                // Check for numeric comparison operators
                if let Some(operator) = args[1].strip_prefix('-') {
                    match operator {
                        "eq" => {
                            let left = args[0].parse::<i32>();
                            let right = args[2].parse::<i32>();
                            match (left, right) {
                                (Ok(l), Ok(r)) => return if l == r { 0 } else { 1 },
                                _ => return 2, // Invalid numeric arguments
                            }
                        }
                        "ne" => {
                            let left = args[0].parse::<i32>();
                            let right = args[2].parse::<i32>();
                            match (left, right) {
                                (Ok(l), Ok(r)) => return if l != r { 0 } else { 1 },
                                _ => return 2, // Invalid numeric arguments
                            }
                        }
                        "lt" => {
                            let left = args[0].parse::<i32>();
                            let right = args[2].parse::<i32>();
                            match (left, right) {
                                (Ok(l), Ok(r)) => return if l < r { 0 } else { 1 },
                                _ => return 2, // Invalid numeric arguments
                            }
                        }
                        "le" => {
                            let left = args[0].parse::<i32>();
                            let right = args[2].parse::<i32>();
                            match (left, right) {
                                (Ok(l), Ok(r)) => return if l <= r { 0 } else { 1 },
                                _ => return 2, // Invalid numeric arguments
                            }
                        }
                        "gt" => {
                            let left = args[0].parse::<i32>();
                            let right = args[2].parse::<i32>();
                            match (left, right) {
                                (Ok(l), Ok(r)) => return if l > r { 0 } else { 1 },
                                _ => return 2, // Invalid numeric arguments
                            }
                        }
                        "ge" => {
                            let left = args[0].parse::<i32>();
                            let right = args[2].parse::<i32>();
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
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;
    use std::fs::File;

    #[test]
    fn test_test_builtin_name() {
        let builtin = TestBuiltin;
        assert_eq!(builtin.name(), "test");
    }

    #[test]
    fn test_test_builtin_description() {
        let builtin = TestBuiltin;
        assert_eq!(builtin.description(), "Evaluate conditional expressions");
    }

    #[test]
    fn test_z_option_empty_string() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec!["test".to_string(), "-z".to_string(), "".to_string()],
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
    fn test_z_option_non_empty_string() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec!["test".to_string(), "-z".to_string(), "hello".to_string()],
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
    fn test_z_option_missing_argument() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec!["test".to_string(), "-z".to_string()],
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
    fn test_n_option_empty_string() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec!["test".to_string(), "-n".to_string(), "".to_string()],
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
    fn test_n_option_non_empty_string() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec!["test".to_string(), "-n".to_string(), "hello".to_string()],
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
    fn test_n_option_missing_argument() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec!["test".to_string(), "-n".to_string()],
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
    fn test_e_option_existing_file() {
        let builtin = TestBuiltin;
        // Create a temporary file
        let temp_path = "/tmp/test_file.txt";
        File::create(temp_path).unwrap();

        let cmd = ShellCommand {
            args: vec!["test".to_string(), "-e".to_string(), temp_path.to_string()],
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
    fn test_e_option_non_existing_file() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "-e".to_string(),
                "/non/existing/file".to_string(),
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
    fn test_f_option_regular_file() {
        let builtin = TestBuiltin;
        // Create a temporary file
        let temp_path = "/tmp/test_regular_file.txt";
        File::create(temp_path).unwrap();

        let cmd = ShellCommand {
            args: vec!["test".to_string(), "-f".to_string(), temp_path.to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // Regular file should return true (0)

        // Clean up
        std::fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn test_f_option_directory() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec!["test".to_string(), "-f".to_string(), "/tmp".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // Directory should return false (1) for -f
    }

    #[test]
    fn test_d_option_directory() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec!["test".to_string(), "-d".to_string(), "/tmp".to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0); // Directory should return true (0) for -d
    }

    #[test]
    fn test_d_option_regular_file() {
        let builtin = TestBuiltin;
        // Create a temporary file
        let temp_path = "/tmp/test_regular_file_for_d.txt";
        File::create(temp_path).unwrap();

        let cmd = ShellCommand {
            args: vec!["test".to_string(), "-d".to_string(), temp_path.to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1); // Regular file should return false (1) for -d

        // Clean up
        std::fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn test_invalid_option() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec!["test".to_string(), "-x".to_string(), "arg".to_string()],
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
    fn test_no_arguments() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec!["test".to_string()],
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
    fn test_eq_equal() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "2".to_string(),
                "-eq".to_string(),
                "2".to_string(),
            ],
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
    fn test_eq_not_equal() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "2".to_string(),
                "-eq".to_string(),
                "3".to_string(),
            ],
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
    fn test_ne_equal() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "2".to_string(),
                "-ne".to_string(),
                "2".to_string(),
            ],
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
    fn test_ne_not_equal() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "2".to_string(),
                "-ne".to_string(),
                "3".to_string(),
            ],
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
    fn test_lt_less() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "2".to_string(),
                "-lt".to_string(),
                "3".to_string(),
            ],
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
    fn test_lt_greater() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "3".to_string(),
                "-lt".to_string(),
                "2".to_string(),
            ],
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
    fn test_le_less() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "2".to_string(),
                "-le".to_string(),
                "3".to_string(),
            ],
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
    fn test_le_equal() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "2".to_string(),
                "-le".to_string(),
                "2".to_string(),
            ],
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
    fn test_le_greater() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "3".to_string(),
                "-le".to_string(),
                "2".to_string(),
            ],
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
    fn test_gt_greater() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "3".to_string(),
                "-gt".to_string(),
                "2".to_string(),
            ],
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
    fn test_gt_less() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "2".to_string(),
                "-gt".to_string(),
                "3".to_string(),
            ],
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
    fn test_ge_greater() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "3".to_string(),
                "-ge".to_string(),
                "2".to_string(),
            ],
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
    fn test_ge_equal() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "2".to_string(),
                "-ge".to_string(),
                "2".to_string(),
            ],
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
    fn test_ge_less() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "2".to_string(),
                "-ge".to_string(),
                "3".to_string(),
            ],
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
    fn test_numeric_invalid_left_operand() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "abc".to_string(),
                "-eq".to_string(),
                "2".to_string(),
            ],
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
    fn test_numeric_invalid_right_operand() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "2".to_string(),
                "-eq".to_string(),
                "abc".to_string(),
            ],
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
    fn test_numeric_invalid_operator() {
        let builtin = TestBuiltin;
        let cmd = ShellCommand {
            args: vec![
                "test".to_string(),
                "2".to_string(),
                "-invalid".to_string(),
                "3".to_string(),
            ],
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
