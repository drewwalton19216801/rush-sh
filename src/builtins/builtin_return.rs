use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct ReturnBuiltin;

impl super::Builtin for ReturnBuiltin {
    fn name(&self) -> &'static str {
        "return"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Return from a shell function"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        _output_writer: &mut dyn Write,
    ) -> i32 {
        // Check if we're inside a function
        if shell_state.function_depth == 0 {
            if shell_state.colors_enabled {
                eprintln!(
                    "{}return: can only be used in a function\x1b[0m",
                    shell_state.color_scheme.error
                );
            } else {
                eprintln!("return: can only be used in a function");
            }
            return 1;
        }

        // Parse exit code from arguments
        let exit_code = if cmd.args.len() > 1 {
            match cmd.args[1].parse::<i32>() {
                Ok(code) => code,
                Err(_) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}return: {}: numeric argument required\x1b[0m",
                            shell_state.color_scheme.error, cmd.args[1]
                        );
                    } else {
                        eprintln!("return: {}: numeric argument required", cmd.args[1]);
                    }
                    return 2;
                }
            }
        } else {
            // Use last exit code if no argument provided
            shell_state.last_exit_code
        };

        // Set return state
        shell_state.set_return(exit_code);

        exit_code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;
    use std::sync::Mutex;

    // Mutex to serialize tests that modify shell state
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_return_builtin_basic() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let cmd = ShellCommand {
            args: vec!["return".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.last_exit_code = 42;
        
        // Simulate being inside a function
        shell_state.enter_function();
        
        let builtin = ReturnBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        
        assert_eq!(exit_code, 42); // Should use last_exit_code
        assert!(shell_state.is_returning());
        assert_eq!(shell_state.get_return_value(), Some(42));
        
        shell_state.exit_function();
    }

    #[test]
    fn test_return_builtin_with_explicit_code() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let cmd = ShellCommand {
            args: vec!["return".to_string(), "5".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        
        // Simulate being inside a function
        shell_state.enter_function();
        
        let builtin = ReturnBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        
        assert_eq!(exit_code, 5);
        assert!(shell_state.is_returning());
        assert_eq!(shell_state.get_return_value(), Some(5));
        
        shell_state.exit_function();
    }

    #[test]
    fn test_return_builtin_invalid_argument() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let cmd = ShellCommand {
            args: vec!["return".to_string(), "invalid".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        
        // Simulate being inside a function
        shell_state.enter_function();
        
        let builtin = ReturnBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        
        assert_eq!(exit_code, 2); // Error code for invalid argument
        assert!(!shell_state.is_returning()); // Should not set returning flag on error
        
        shell_state.exit_function();
    }

    #[test]
    fn test_return_builtin_outside_function() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let cmd = ShellCommand {
            args: vec!["return".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        
        // Do NOT enter a function - function_depth should be 0
        assert_eq!(shell_state.function_depth, 0);
        
        let builtin = ReturnBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        
        assert_eq!(exit_code, 1); // Error code for return outside function
        assert!(!shell_state.is_returning());
    }

    #[test]
    fn test_return_builtin_nested_functions() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let cmd = ShellCommand {
            args: vec!["return".to_string(), "10".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        
        // Simulate nested function calls
        shell_state.enter_function(); // depth = 1
        shell_state.enter_function(); // depth = 2
        
        assert_eq!(shell_state.function_depth, 2);
        
        let builtin = ReturnBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        
        assert_eq!(exit_code, 10);
        assert!(shell_state.is_returning());
        assert_eq!(shell_state.get_return_value(), Some(10));
        
        shell_state.exit_function();
        shell_state.exit_function();
    }

    #[test]
    fn test_return_builtin_value_propagation() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let mut shell_state = ShellState::new();
        
        // Simulate being inside a function
        shell_state.enter_function();
        
        // Return with value 123
        let cmd = ShellCommand {
            args: vec!["return".to_string(), "123".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        
        let builtin = ReturnBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        
        assert_eq!(exit_code, 123);
        assert!(shell_state.is_returning());
        assert_eq!(shell_state.get_return_value(), Some(123));
        
        // Clear return state
        shell_state.clear_return();
        assert!(!shell_state.is_returning());
        assert_eq!(shell_state.get_return_value(), None);
        
        shell_state.exit_function();
    }

    #[test]
    fn test_return_builtin_zero_code() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let cmd = ShellCommand {
            args: vec!["return".to_string(), "0".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        
        // Simulate being inside a function
        shell_state.enter_function();
        
        let builtin = ReturnBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        
        assert_eq!(exit_code, 0);
        assert!(shell_state.is_returning());
        assert_eq!(shell_state.get_return_value(), Some(0));
        
        shell_state.exit_function();
    }

    #[test]
    fn test_return_builtin_negative_code() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let cmd = ShellCommand {
            args: vec!["return".to_string(), "-1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        
        // Simulate being inside a function
        shell_state.enter_function();
        
        let builtin = ReturnBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        
        assert_eq!(exit_code, -1);
        assert!(shell_state.is_returning());
        assert_eq!(shell_state.get_return_value(), Some(-1));
        
        shell_state.exit_function();
    }
}