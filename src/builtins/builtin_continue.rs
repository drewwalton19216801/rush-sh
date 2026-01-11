use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct ContinueBuiltin;

impl super::Builtin for ContinueBuiltin {
    fn name(&self) -> &'static str {
        "continue"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Resume the next iteration of the enclosing for, while, or until loop"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        _output_writer: &mut dyn Write,
    ) -> i32 {
        // Check if we're inside a loop
        if shell_state.loop_depth == 0 {
            if shell_state.colors_enabled {
                eprintln!(
                    "{}continue: only meaningful in a `for', `while', or `until' loop\x1b[0m",
                    shell_state.color_scheme.error
                );
            } else {
                eprintln!("continue: only meaningful in a `for', `while', or `until' loop");
            }
            return 1;
        }

        // Parse the optional numeric argument [n]
        let continue_level = if cmd.args.len() > 1 {
            match cmd.args[1].parse::<usize>() {
                Ok(n) if n > 0 => n,
                Ok(_) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}continue: {}: loop count out of range\x1b[0m",
                            shell_state.color_scheme.error, cmd.args[1]
                        );
                    } else {
                        eprintln!("continue: {}: loop count out of range", cmd.args[1]);
                    }
                    return 1;
                }
                Err(_) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}continue: {}: numeric argument required\x1b[0m",
                            shell_state.color_scheme.error, cmd.args[1]
                        );
                    } else {
                        eprintln!("continue: {}: numeric argument required", cmd.args[1]);
                    }
                    return 1;
                }
            }
        } else {
            1 // Default: continue to next iteration of current loop
        };

        // Check if continue level exceeds current loop depth
        if continue_level > shell_state.loop_depth {
            if shell_state.colors_enabled {
                eprintln!(
                    "{}continue: {}: loop count out of range\x1b[0m",
                    shell_state.color_scheme.error, continue_level
                );
            } else {
                eprintln!("continue: {}: loop count out of range", continue_level);
            }
            return 1;
        }

        // Set continue state
        shell_state.set_continue(continue_level);

        0
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
    fn test_continue_builtin_basic() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["continue".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Simulate being inside a loop
        shell_state.enter_loop();

        let builtin = ContinueBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert!(shell_state.is_continuing());
        assert_eq!(shell_state.get_continue_level(), 1);

        shell_state.exit_loop();
    }

    #[test]
    fn test_continue_builtin_with_level() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["continue".to_string(), "2".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Simulate being inside nested loops
        shell_state.enter_loop(); // depth = 1
        shell_state.enter_loop(); // depth = 2

        let builtin = ContinueBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert!(shell_state.is_continuing());
        assert_eq!(shell_state.get_continue_level(), 2);

        shell_state.exit_loop();
        shell_state.exit_loop();
    }

    #[test]
    fn test_continue_builtin_invalid_argument() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["continue".to_string(), "invalid".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Simulate being inside a loop
        shell_state.enter_loop();

        let builtin = ContinueBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1); // Error code for invalid argument
        assert!(!shell_state.is_continuing()); // Should not set continuing flag on error

        shell_state.exit_loop();
    }

    #[test]
    fn test_continue_builtin_outside_loop() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["continue".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Do NOT enter a loop - loop_depth should be 0
        assert_eq!(shell_state.loop_depth, 0);

        let builtin = ContinueBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1); // Error code for continue outside loop
        assert!(!shell_state.is_continuing());
    }

    #[test]
    fn test_continue_builtin_level_exceeds_depth() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["continue".to_string(), "3".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Simulate being inside only 2 nested loops
        shell_state.enter_loop(); // depth = 1
        shell_state.enter_loop(); // depth = 2

        let builtin = ContinueBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1); // Error code for level exceeds depth
        assert!(!shell_state.is_continuing());

        shell_state.exit_loop();
        shell_state.exit_loop();
    }

    #[test]
    fn test_continue_builtin_zero_argument() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["continue".to_string(), "0".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Simulate being inside a loop
        shell_state.enter_loop();

        let builtin = ContinueBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1); // Error code for zero argument
        assert!(!shell_state.is_continuing());

        shell_state.exit_loop();
    }

    #[test]
    fn test_continue_builtin_negative_argument() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["continue".to_string(), "-1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Simulate being inside a loop
        shell_state.enter_loop();

        let builtin = ContinueBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1); // Error code for invalid argument
        assert!(!shell_state.is_continuing());

        shell_state.exit_loop();
    }

    #[test]
    fn test_continue_in_until_loop() {
        let _lock = ENV_LOCK.lock().unwrap();

        use crate::executor::execute;
        use crate::parser::Ast;

        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        shell_state.set_var("i", "0".to_string());

        // until [ $i -ge 5 ]; do i=$((i + 1)); if [ $i = "3" ]; then continue; fi; output="$output$i"; done
        let ast = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "-ge".to_string(), "5".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
                Ast::If {
                    branches: vec![(
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "3".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["continue".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                    )],
                    else_branch: None,
                },
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i".to_string(),
                },
            ])),
        };

        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("1245".to_string()));
    }

    #[test]
    fn test_continue_in_nested_until_loops() {
        let _lock = ENV_LOCK.lock().unwrap();

        use crate::executor::execute;
        use crate::parser::Ast;

        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        shell_state.set_var("i", "0".to_string());

        // Nested until loops with continue
        let inner_loop = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "3".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "j".to_string(),
                    value: "$((j + 1))".to_string(),
                },
                Ast::If {
                    branches: vec![(
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "2".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["continue".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                    )],
                    else_branch: None,
                },
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i$j".to_string(),
                },
            ])),
        };

        let outer_loop = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "3".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
                Ast::Assignment {
                    var: "j".to_string(),
                    value: "0".to_string(),
                },
                inner_loop,
            ])),
        };

        let exit_code = execute(outer_loop, &mut shell_state);
        assert_eq!(exit_code, 0);
        // Inner loop continues at j=2, so we get: 11, 13 (skip 12), 21, 23 (skip 22), 31, 33 (skip 32)
        assert_eq!(shell_state.get_var("output"), Some("111321233133".to_string()));
    }

    #[test]
    fn test_continue_2_in_nested_until_loops() {
        let _lock = ENV_LOCK.lock().unwrap();

        use crate::executor::execute;
        use crate::parser::Ast;

        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        shell_state.set_var("i", "0".to_string());

        // Nested until loops with continue 2
        let inner_loop = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "3".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "j".to_string(),
                    value: "$((j + 1))".to_string(),
                },
                Ast::And {
                    left: Box::new(Ast::Pipeline(vec![ShellCommand {
                        args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "2".to_string()],
                        redirections: vec![],
                        compound: None,
                    }])),
                    right: Box::new(Ast::If {
                        branches: vec![(
                            Box::new(Ast::Pipeline(vec![ShellCommand {
                                args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "1".to_string()],
                                redirections: vec![],
                                compound: None,
                            }])),
                            Box::new(Ast::Pipeline(vec![ShellCommand {
                                args: vec!["continue".to_string(), "2".to_string()],
                                redirections: vec![],
                                compound: None,
                            }])),
                        )],
                        else_branch: None,
                    }),
                },
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i$j".to_string(),
                },
            ])),
        };

        let outer_loop = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "3".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
                Ast::Assignment {
                    var: "j".to_string(),
                    value: "0".to_string(),
                },
                inner_loop,
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output-".to_string(),
                },
            ])),
        };

        let exit_code = execute(outer_loop, &mut shell_state);
        assert_eq!(exit_code, 0);
        // After 21, continue 2 skips rest of inner loop and the "-" assignment, goes to next outer iteration
        assert_eq!(shell_state.get_var("output"), Some("111213-313233-".to_string()));
    }
}