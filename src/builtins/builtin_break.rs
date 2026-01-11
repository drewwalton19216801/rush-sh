use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct BreakBuiltin;

impl super::Builtin for BreakBuiltin {
    fn name(&self) -> &'static str {
        "break"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Exit from the enclosing for, while, or until loop"
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
                    "{}break: only meaningful in a `for', `while', or `until' loop\x1b[0m",
                    shell_state.color_scheme.error
                );
            } else {
                eprintln!("break: only meaningful in a `for', `while', or `until' loop");
            }
            return 1;
        }

        // Parse the optional numeric argument [n]
        let break_level = if cmd.args.len() > 1 {
            match cmd.args[1].parse::<usize>() {
                Ok(n) if n > 0 => n,
                Ok(_) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}break: {}: loop count out of range\x1b[0m",
                            shell_state.color_scheme.error, cmd.args[1]
                        );
                    } else {
                        eprintln!("break: {}: loop count out of range", cmd.args[1]);
                    }
                    return 1;
                }
                Err(_) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}break: {}: numeric argument required\x1b[0m",
                            shell_state.color_scheme.error, cmd.args[1]
                        );
                    } else {
                        eprintln!("break: {}: numeric argument required", cmd.args[1]);
                    }
                    return 1;
                }
            }
        } else {
            1 // Default: break out of 1 loop level
        };

        // Check if break level exceeds current loop depth
        if break_level > shell_state.loop_depth {
            if shell_state.colors_enabled {
                eprintln!(
                    "{}break: {}: loop count out of range\x1b[0m",
                    shell_state.color_scheme.error, break_level
                );
            } else {
                eprintln!("break: {}: loop count out of range", break_level);
            }
            return 1;
        }

        // Set break state
        shell_state.set_break(break_level);

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
    fn test_break_builtin_basic() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["break".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Simulate being inside a loop
        shell_state.enter_loop();

        let builtin = BreakBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert!(shell_state.is_breaking());
        assert_eq!(shell_state.get_break_level(), 1);

        shell_state.exit_loop();
    }

    #[test]
    fn test_break_builtin_with_level() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["break".to_string(), "2".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Simulate being inside nested loops
        shell_state.enter_loop(); // depth = 1
        shell_state.enter_loop(); // depth = 2

        let builtin = BreakBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert!(shell_state.is_breaking());
        assert_eq!(shell_state.get_break_level(), 2);

        shell_state.exit_loop();
        shell_state.exit_loop();
    }

    #[test]
    fn test_break_builtin_invalid_argument() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["break".to_string(), "invalid".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Simulate being inside a loop
        shell_state.enter_loop();

        let builtin = BreakBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1); // Error code for invalid argument
        assert!(!shell_state.is_breaking()); // Should not set breaking flag on error

        shell_state.exit_loop();
    }

    #[test]
    fn test_break_builtin_outside_loop() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["break".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Do NOT enter a loop - loop_depth should be 0
        assert_eq!(shell_state.loop_depth, 0);

        let builtin = BreakBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1); // Error code for break outside loop
        assert!(!shell_state.is_breaking());
    }

    #[test]
    fn test_break_builtin_level_exceeds_depth() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["break".to_string(), "3".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Simulate being inside only 2 nested loops
        shell_state.enter_loop(); // depth = 1
        shell_state.enter_loop(); // depth = 2

        let builtin = BreakBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1); // Error code for level exceeds depth
        assert!(!shell_state.is_breaking());

        shell_state.exit_loop();
        shell_state.exit_loop();
    }

    #[test]
    fn test_break_builtin_zero_argument() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["break".to_string(), "0".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Simulate being inside a loop
        shell_state.enter_loop();

        let builtin = BreakBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1); // Error code for zero argument
        assert!(!shell_state.is_breaking());

        shell_state.exit_loop();
    }

    #[test]
    fn test_break_builtin_negative_argument() {
        let _lock = ENV_LOCK.lock().unwrap();

        let cmd = ShellCommand {
            args: vec!["break".to_string(), "-1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        // Simulate being inside a loop
        shell_state.enter_loop();

        let builtin = BreakBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1); // Error code for invalid argument
        assert!(!shell_state.is_breaking());

        shell_state.exit_loop();
    }

    #[test]
    fn test_break_in_until_loop() {
        let _lock = ENV_LOCK.lock().unwrap();

        use crate::executor::execute;
        use crate::parser::Ast;

        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        shell_state.set_var("i", "0".to_string());

        // until false; do output="$output$i"; i=$((i + 1)); if [ $i = "3" ]; then break; fi; done
        let ast = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["false".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i".to_string(),
                },
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
                            args: vec!["break".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                    )],
                    else_branch: None,
                },
            ])),
        };

        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("012".to_string()));
    }

    #[test]
    fn test_break_in_nested_until_loops() {
        let _lock = ENV_LOCK.lock().unwrap();

        use crate::executor::execute;
        use crate::parser::Ast;

        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        shell_state.set_var("i", "0".to_string());

        // Nested until loops with break
        let inner_loop = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "3".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i$j".to_string(),
                },
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
                            args: vec!["break".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                    )],
                    else_branch: None,
                },
            ])),
        };

        let outer_loop = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "2".to_string()],
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
        // Inner loop breaks at j=2, so we get: 10, 11 (then break), 20, 21 (then break)
        assert_eq!(shell_state.get_var("output"), Some("10112021".to_string()));
    }

    #[test]
    fn test_break_2_in_nested_until_loops() {
        let _lock = ENV_LOCK.lock().unwrap();

        use crate::executor::execute;
        use crate::parser::Ast;

        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        shell_state.set_var("i", "0".to_string());

        // Nested until loops with break 2
        let inner_loop = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "3".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i$j".to_string(),
                },
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
                                args: vec!["break".to_string(), "2".to_string()],
                                redirections: vec![],
                                compound: None,
                            }])),
                        )],
                        else_branch: None,
                    }),
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
        assert_eq!(shell_state.get_var("output"), Some("10111220".to_string()));
    }
}