//! Tests for main execute() function and control flow (loops, functions, conditionals, etc.)

use crate::executor::execute;
use crate::parser::{Ast, ShellCommand};
use crate::state::ShellState;

// ========================================================================
// Function Tests
// ========================================================================

#[test]
fn test_execute_function_definition() {
    let ast = Ast::FunctionDefinition {
        name: "test_func".to_string(),
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["echo".to_string(), "hello".to_string()],
            redirections: Vec::new(),
            compound: None,
        }])),
    };
    let mut shell_state = ShellState::new();
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Check that function was stored
    assert!(shell_state.get_function("test_func").is_some());
}

#[test]
fn test_execute_function_call() {
    // First define a function
    let mut shell_state = ShellState::new();
    shell_state.define_function(
        "test_func".to_string(),
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["echo".to_string(), "hello".to_string()],
            redirections: Vec::new(),
            compound: None,
        }]),
    );

    // Now call the function
    let ast = Ast::FunctionCall {
        name: "test_func".to_string(),
        args: vec![],
    };
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_execute_function_call_with_args() {
    // First define a function that uses arguments
    let mut shell_state = ShellState::new();
    shell_state.define_function(
        "test_func".to_string(),
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["echo".to_string(), "arg1".to_string()],
            redirections: Vec::new(),
            compound: None,
        }]),
    );

    // Now call the function with arguments
    let ast = Ast::FunctionCall {
        name: "test_func".to_string(),
        args: vec!["hello".to_string()],
    };
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_execute_nonexistent_function() {
    let mut shell_state = ShellState::new();
    let ast = Ast::FunctionCall {
        name: "nonexistent".to_string(),
        args: vec![],
    };
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 1); // Should return error code
}

#[test]
fn test_execute_function_integration() {
    // Test full integration: define function, then call it
    let mut shell_state = ShellState::new();

    // First define a function
    let define_ast = Ast::FunctionDefinition {
        name: "hello".to_string(),
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["printf".to_string(), "Hello from function".to_string()],
            redirections: Vec::new(),
            compound: None,
        }])),
    };
    let exit_code = execute(define_ast, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Now call the function
    let call_ast = Ast::FunctionCall {
        name: "hello".to_string(),
        args: vec![],
    };
    let exit_code = execute(call_ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_execute_function_with_local_variables() {
    let mut shell_state = ShellState::new();

    // Set a global variable
    shell_state.set_var("global_var", "global_value".to_string());

    // Define a function that uses local variables
    let define_ast = Ast::FunctionDefinition {
        name: "test_func".to_string(),
        body: Box::new(Ast::Sequence(vec![
            Ast::LocalAssignment {
                var: "local_var".to_string(),
                value: "local_value".to_string(),
            },
            Ast::Assignment {
                var: "global_var".to_string(),
                value: "modified_in_function".to_string(),
            },
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["printf".to_string(), "success".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        ])),
    };
    let exit_code = execute(define_ast, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Global variable should not be modified during function definition
    assert_eq!(
        shell_state.get_var("global_var"),
        Some("global_value".to_string())
    );

    // Call the function
    let call_ast = Ast::FunctionCall {
        name: "test_func".to_string(),
        args: vec![],
    };
    let exit_code = execute(call_ast, &mut shell_state);
    assert_eq!(exit_code, 0);

    // After function call, global variable should be modified since function assignments affect global scope
    assert_eq!(
        shell_state.get_var("global_var"),
        Some("modified_in_function".to_string())
    );
}

#[test]
fn test_execute_nested_function_calls() {
    let mut shell_state = ShellState::new();

    // Set global variable
    shell_state.set_var("global_var", "global".to_string());

    // Define outer function
    let outer_func = Ast::FunctionDefinition {
        name: "outer".to_string(),
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "global_var".to_string(),
                value: "outer_modified".to_string(),
            },
            Ast::FunctionCall {
                name: "inner".to_string(),
                args: vec![],
            },
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["printf".to_string(), "outer_done".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        ])),
    };

    // Define inner function
    let inner_func = Ast::FunctionDefinition {
        name: "inner".to_string(),
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "global_var".to_string(),
                value: "inner_modified".to_string(),
            },
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["printf".to_string(), "inner_done".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        ])),
    };

    // Define both functions
    execute(outer_func, &mut shell_state);
    execute(inner_func, &mut shell_state);

    // Set initial global value
    shell_state.set_var("global_var", "initial".to_string());

    // Call outer function (which calls inner function)
    let call_ast = Ast::FunctionCall {
        name: "outer".to_string(),
        args: vec![],
    };
    let exit_code = execute(call_ast, &mut shell_state);
    assert_eq!(exit_code, 0);

    // After nested function calls, global variable should be modified by inner function
    // (bash behavior: function variable assignments affect global scope)
    assert_eq!(
        shell_state.get_var("global_var"),
        Some("inner_modified".to_string())
    );
}

// ========================================================================
// Break and Continue Integration Tests
// ========================================================================

#[test]
fn test_break_in_for_loop() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    
    // for i in 1 2 3 4 5; do
    //   output="$output$i"
    //   if [ $i = "3" ]; then break; fi
    // done
    let ast = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string(), "5".to_string()],
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "output".to_string(),
                value: "$output$i".to_string(),
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
    assert_eq!(shell_state.get_var("output"), Some("123".to_string()));
}

#[test]
fn test_continue_in_for_loop() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    
    // for i in 1 2 3 4 5; do
    //   if [ $i = "3" ]; then continue; fi
    //   output="$output$i"
    // done
    let ast = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string(), "5".to_string()],
        body: Box::new(Ast::Sequence(vec![
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
fn test_break_in_while_loop() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("i", "0".to_string());
    shell_state.set_var("output", "".to_string());
    
    // i=0
    // while [ $i -lt 10 ]; do
    //   i=$((i + 1))
    //   output="$output$i"
    //   if [ $i = "5" ]; then break; fi
    // done
    let ast = Ast::While {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$i".to_string(), "-lt".to_string(), "10".to_string()],
            redirections: vec![],
            compound: None,
        }])),
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "i".to_string(),
                value: "$((i + 1))".to_string(),
            },
            Ast::Assignment {
                var: "output".to_string(),
                value: "$output$i".to_string(),
            },
            Ast::If {
                branches: vec![(
                    Box::new(Ast::Pipeline(vec![ShellCommand {
                        args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "5".to_string()],
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
    assert_eq!(shell_state.get_var("output"), Some("12345".to_string()));
}

#[test]
fn test_continue_in_while_loop() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("i", "0".to_string());
    shell_state.set_var("output", "".to_string());
    
    // i=0
    // while [ $i -lt 5 ]; do
    //   i=$((i + 1))
    //   if [ $i = "3" ]; then continue; fi
    //   output="$output$i"
    // done
    let ast = Ast::While {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$i".to_string(), "-lt".to_string(), "5".to_string()],
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
fn test_break_nested_loops() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    
    // for i in 1 2 3; do
    //   for j in a b c; do
    //     output="$output$i$j"
    //     if [ $j = "b" ]; then break; fi
    //   done
    // done
    let inner_loop = Ast::For {
        variable: "j".to_string(),
        items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "output".to_string(),
                value: "$output$i$j".to_string(),
            },
            Ast::If {
                branches: vec![(
                    Box::new(Ast::Pipeline(vec![ShellCommand {
                        args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "b".to_string()],
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
    
    let outer_loop = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        body: Box::new(inner_loop),
    };
    
    let exit_code = execute(outer_loop, &mut shell_state);
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("output"), Some("1a1b2a2b3a3b".to_string()));
}

#[test]
fn test_break_2_nested_loops() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    
    // for i in 1 2 3; do
    //   for j in a b c; do
    //     output="$output$i$j"
    //     if [ $i = "2" ] && [ $j = "b" ]; then break 2; fi
    //   done
    // done
    let inner_loop = Ast::For {
        variable: "j".to_string(),
        items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "output".to_string(),
                value: "$output$i$j".to_string(),
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
                            args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "b".to_string()],
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
    
    let outer_loop = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        body: Box::new(inner_loop),
    };
    
    let exit_code = execute(outer_loop, &mut shell_state);
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("output"), Some("1a1b1c2a2b".to_string()));
}

#[test]
fn test_continue_nested_loops() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    
    // for i in 1 2 3; do
    //   for j in a b c; do
    //     if [ $j = "b" ]; then continue; fi
    //     output="$output$i$j"
    //   done
    // done
    let inner_loop = Ast::For {
        variable: "j".to_string(),
        items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        body: Box::new(Ast::Sequence(vec![
            Ast::If {
                branches: vec![(
                    Box::new(Ast::Pipeline(vec![ShellCommand {
                        args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "b".to_string()],
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
    
    let outer_loop = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        body: Box::new(inner_loop),
    };
    
    let exit_code = execute(outer_loop, &mut shell_state);
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("output"), Some("1a1c2a2c3a3c".to_string()));
}

#[test]
fn test_continue_2_nested_loops() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    
    // for i in 1 2 3; do
    //   for j in a b c; do
    //     if [ $i = "2" ] && [ $j = "b" ]; then continue 2; fi
    //     output="$output$i$j"
    //   done
    //   output="$output-"
    // done
    let inner_loop = Ast::For {
        variable: "j".to_string(),
        items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        body: Box::new(Ast::Sequence(vec![
            Ast::And {
                left: Box::new(Ast::Pipeline(vec![ShellCommand {
                    args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "2".to_string()],
                    redirections: vec![],
                    compound: None,
                }])),
                right: Box::new(Ast::If {
                    branches: vec![(
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "b".to_string()],
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
    
    let outer_loop = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        body: Box::new(Ast::Sequence(vec![
            inner_loop,
            Ast::Assignment {
                var: "output".to_string(),
                value: "$output$i-".to_string(),
            },
        ])),
    };
    
    let exit_code = execute(outer_loop, &mut shell_state);
    assert_eq!(exit_code, 0);
    // After 2a, continue 2 skips rest of inner loop and the "$i-" assignment, goes to next outer iteration
    assert_eq!(shell_state.get_var("output"), Some("1a1b1c1-2a3a3b3c3-".to_string()));
}

#[test]
fn test_break_preserves_exit_code() {
    let mut shell_state = ShellState::new();
    
    // for i in 1 2 3; do
    //   false
    //   break
    // done
    // echo $?
    let ast = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        body: Box::new(Ast::Sequence(vec![
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["false".to_string()],
                redirections: vec![],
                compound: None,
            }]),
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["break".to_string()],
                redirections: vec![],
                compound: None,
            }]),
        ])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    // break returns 0, so the loop's exit code should be 0
    assert_eq!(exit_code, 0);
}

#[test]
fn test_continue_preserves_exit_code() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("count", "0".to_string());
    
    // for i in 1 2; do
    //   count=$((count + 1))
    //   false
    //   continue
    // done
    let ast = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string()],
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "count".to_string(),
                value: "$((count + 1))".to_string(),
            },
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["false".to_string()],
                redirections: vec![],
                compound: None,
            }]),
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["continue".to_string()],
                redirections: vec![],
                compound: None,
            }]),
        ])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    // continue returns 0, so the loop's exit code should be 0
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("count"), Some("2".to_string()));
}

// ========================================================================
// Until Loop Tests
// ========================================================================

#[test]
fn test_until_basic_loop() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("i", "0".to_string());
    shell_state.set_var("output", "".to_string());
    
    // i=0; until [ $i = "3" ]; do output="$output$i"; i=$((i + 1)); done
    let ast = Ast::Until {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "3".to_string()],
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
        ])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("output"), Some("012".to_string()));
    assert_eq!(shell_state.get_var("i"), Some("3".to_string()));
}

#[test]
fn test_until_condition_initially_true() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("executed", "no".to_string());
    
    // until true; do executed="yes"; done
    let ast = Ast::Until {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            redirections: vec![],
            compound: None,
        }])),
        body: Box::new(Ast::Assignment {
            var: "executed".to_string(),
            value: "yes".to_string(),
        }),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    // Body should not execute since condition is true (exit code 0)
    assert_eq!(shell_state.get_var("executed"), Some("no".to_string()));
}

#[test]
fn test_until_with_commands_in_body() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("count", "0".to_string());
    
    // count=0; until [ $count -ge 3 ]; do count=$((count + 1)); echo $count; done
    let ast = Ast::Until {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$count".to_string(), "-ge".to_string(), "3".to_string()],
            redirections: vec![],
            compound: None,
        }])),
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "count".to_string(),
                value: "$((count + 1))".to_string(),
            },
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "$count".to_string()],
                redirections: vec![],
                compound: None,
            }]),
        ])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("count"), Some("3".to_string()));
}

#[test]
fn test_until_with_variable_modification() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("x", "1".to_string());
    
    // x=1; until [ $x -gt 5 ]; do x=$((x * 2)); done
    let ast = Ast::Until {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$x".to_string(), "-gt".to_string(), "5".to_string()],
            redirections: vec![],
            compound: None,
        }])),
        body: Box::new(Ast::Assignment {
            var: "x".to_string(),
            value: "$((x * 2))".to_string(),
        }),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("x"), Some("8".to_string()));
}

#[test]
fn test_until_nested_loops() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    shell_state.set_var("i", "0".to_string());
    
    let inner_loop = Ast::Until {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "2".to_string()],
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
    assert_eq!(shell_state.get_var("output"), Some("10112021".to_string()));
}

#[test]
fn test_until_with_break() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("i", "0".to_string());
    shell_state.set_var("output", "".to_string());
    
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
fn test_until_with_continue() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("i", "0".to_string());
    shell_state.set_var("output", "".to_string());
    
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
fn test_until_empty_body() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("i", "0".to_string());
    
    // until true; do :; done (empty body with true condition)
    let ast = Ast::Until {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            redirections: vec![],
            compound: None,
        }])),
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_until_with_command_substitution() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("count", "0".to_string());
    shell_state.set_var("output", "".to_string());
    
    // until [ $(echo $count) = "3" ]; do output="$output$count"; count=$((count + 1)); done
    let ast = Ast::Until {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$(echo $count)".to_string(), "=".to_string(), "3".to_string()],
            redirections: vec![],
            compound: None,
        }])),
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "output".to_string(),
                value: "$output$count".to_string(),
            },
            Ast::Assignment {
                var: "count".to_string(),
                value: "$((count + 1))".to_string(),
            },
        ])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("output"), Some("012".to_string()));
}

#[test]
fn test_until_with_arithmetic_condition() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("x", "1".to_string());
    shell_state.set_var("output", "".to_string());
    
    // x=1; until [ $((x * 2)) -gt 10 ]; do output="$output$x"; x=$((x + 1)); done
    let ast = Ast::Until {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$((x * 2))".to_string(), "-gt".to_string(), "10".to_string()],
            redirections: vec![],
            compound: None,
        }])),
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "output".to_string(),
                value: "$output$x".to_string(),
            },
            Ast::Assignment {
                var: "x".to_string(),
                value: "$((x + 1))".to_string(),
            },
        ])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("output"), Some("12345".to_string()));
}

#[test]
fn test_until_inside_for() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    
    // for i in 1 2; do j=0; until [ $j = "2" ]; do output="$output$i$j"; j=$((j + 1)); done; done
    let inner_until = Ast::Until {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "2".to_string()],
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
        ])),
    };
    
    let outer_for = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string()],
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "j".to_string(),
                value: "0".to_string(),
            },
            inner_until,
        ])),
    };
    
    let exit_code = execute(outer_for, &mut shell_state);
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("output"), Some("10112021".to_string()));
}

#[test]
fn test_for_inside_until() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    shell_state.set_var("i", "0".to_string());
    
    // i=0; until [ $i = "2" ]; do for j in a b; do output="$output$i$j"; done; i=$((i + 1)); done
    let inner_for = Ast::For {
        variable: "j".to_string(),
        items: vec!["a".to_string(), "b".to_string()],
        body: Box::new(Ast::Assignment {
            var: "output".to_string(),
            value: "$output$i$j".to_string(),
        }),
    };
    
    let outer_until = Ast::Until {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "2".to_string()],
            redirections: vec![],
            compound: None,
        }])),
        body: Box::new(Ast::Sequence(vec![
            inner_for,
            Ast::Assignment {
                var: "i".to_string(),
                value: "$((i + 1))".to_string(),
            },
        ])),
    };
    
    let exit_code = execute(outer_until, &mut shell_state);
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("output"), Some("0a0b1a1b".to_string()));
}

#[test]
fn test_until_inside_while() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    shell_state.set_var("i", "0".to_string());
    
    let inner_until = Ast::Until {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "2".to_string()],
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
        ])),
    };
    
    let outer_while = Ast::While {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$i".to_string(), "-lt".to_string(), "2".to_string()],
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
            inner_until,
        ])),
    };
    
    let exit_code = execute(outer_while, &mut shell_state);
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("output"), Some("10112021".to_string()));
}

#[test]
fn test_while_inside_until() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    shell_state.set_var("i", "0".to_string());
    
    let inner_while = Ast::While {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$j".to_string(), "-lt".to_string(), "2".to_string()],
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
        ])),
    };
    
    let outer_until = Ast::Until {
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
            inner_while,
        ])),
    };
    
    let exit_code = execute(outer_until, &mut shell_state);
    assert_eq!(exit_code, 0);
    assert_eq!(shell_state.get_var("output"), Some("10112021".to_string()));
}

#[test]
fn test_until_preserves_exit_code() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("i", "0".to_string());
    
    // until [ $i = "1" ]; do i=$((i + 1)); false; done
    let ast = Ast::Until {
        condition: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "1".to_string()],
            redirections: vec![],
            compound: None,
        }])),
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "i".to_string(),
                value: "$((i + 1))".to_string(),
            },
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["false".to_string()],
                redirections: vec![],
                compound: None,
            }]),
        ])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    // Last command in body was false (exit 1), so loop should return 1
    assert_eq!(exit_code, 1);
}

// ========================================================================
// Control-Flow in Logical Chains Tests (&&, ||)
// ========================================================================

#[test]
fn test_and_with_return_in_lhs() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("executed", "no".to_string());
    
    // Define a function that returns early
    shell_state.define_function(
        "early_return".to_string(),
        Ast::Sequence(vec![
            Ast::Assignment {
                var: "executed".to_string(),
                value: "yes".to_string(),
            },
            Ast::Return { value: Some("5".to_string()) },
        ]),
    );
    
    // Call function in && chain: early_return && echo "should not execute"
    let ast = Ast::FunctionCall {
        name: "early_return".to_string(),
        args: vec![],
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 5);
    assert_eq!(shell_state.get_var("executed"), Some("yes".to_string()));
}

#[test]
fn test_and_with_exit_in_lhs() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("rhs_executed", "no".to_string());
    
    // exit 42 && rhs_executed=yes
    let ast = Ast::And {
        left: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["exit".to_string(), "42".to_string()],
            redirections: vec![],
            compound: None,
        }])),
        right: Box::new(Ast::Assignment {
            var: "rhs_executed".to_string(),
            value: "yes".to_string(),
        }),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 42);
    assert_eq!(shell_state.get_var("rhs_executed"), Some("no".to_string()));
    assert!(shell_state.exit_requested);
}

#[test]
fn test_and_with_break_in_lhs() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    
    // for i in 1 2 3; do
    //   (break && output="${output}bad") && output="${output}$i"
    // done
    let ast = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        body: Box::new(Ast::And {
            left: Box::new(Ast::And {
                left: Box::new(Ast::Pipeline(vec![ShellCommand {
                    args: vec!["break".to_string()],
                    redirections: vec![],
                    compound: None,
                }])),
                right: Box::new(Ast::Assignment {
                    var: "output".to_string(),
                    value: "${output}bad".to_string(),
                }),
            }),
            right: Box::new(Ast::Assignment {
                var: "output".to_string(),
                value: "${output}$i".to_string(),
            }),
        }),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    // RHS should not execute after break
    assert_eq!(shell_state.get_var("output"), Some("".to_string()));
}

#[test]
fn test_and_with_continue_in_lhs() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    
    // for i in 1 2 3; do
    //   continue && output="${output}bad"
    //   output="${output}$i"
    // done
    let ast = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        body: Box::new(Ast::Sequence(vec![
            Ast::And {
                left: Box::new(Ast::Pipeline(vec![ShellCommand {
                    args: vec!["continue".to_string()],
                    redirections: vec![],
                    compound: None,
                }])),
                right: Box::new(Ast::Assignment {
                    var: "output".to_string(),
                    value: "${output}bad".to_string(),
                }),
            },
            Ast::Assignment {
                var: "output".to_string(),
                value: "${output}$i".to_string(),
            },
        ])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    // RHS of && should not execute, and subsequent assignment should not execute either
    assert_eq!(shell_state.get_var("output"), Some("".to_string()));
}

#[test]
fn test_or_with_return_in_lhs() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("executed", "no".to_string());
    
    // Define a function that returns early with non-zero
    shell_state.define_function(
        "early_return".to_string(),
        Ast::Sequence(vec![
            Ast::Assignment {
                var: "executed".to_string(),
                value: "yes".to_string(),
            },
            Ast::Return { value: Some("5".to_string()) },
        ]),
    );
    
    // Call function in || chain: early_return || echo "should not execute"
    let ast = Ast::FunctionCall {
        name: "early_return".to_string(),
        args: vec![],
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 5);
    assert_eq!(shell_state.get_var("executed"), Some("yes".to_string()));
}

#[test]
fn test_or_with_exit_in_lhs() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("rhs_executed", "no".to_string());
    
    // exit 42 || rhs_executed=yes
    let ast = Ast::Or {
        left: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["exit".to_string(), "42".to_string()],
            redirections: vec![],
            compound: None,
        }])),
        right: Box::new(Ast::Assignment {
            var: "rhs_executed".to_string(),
            value: "yes".to_string(),
        }),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 42);
    assert_eq!(shell_state.get_var("rhs_executed"), Some("no".to_string()));
    assert!(shell_state.exit_requested);
}

#[test]
fn test_or_with_break_in_lhs() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    
    // for i in 1 2 3; do
    //   (false || break) || output="${output}$i"
    // done
    let ast = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        body: Box::new(Ast::Or {
            left: Box::new(Ast::Or {
                left: Box::new(Ast::Pipeline(vec![ShellCommand {
                    args: vec!["false".to_string()],
                    redirections: vec![],
                    compound: None,
                }])),
                right: Box::new(Ast::Pipeline(vec![ShellCommand {
                    args: vec!["break".to_string()],
                    redirections: vec![],
                    compound: None,
                }])),
            }),
            right: Box::new(Ast::Assignment {
                var: "output".to_string(),
                value: "${output}$i".to_string(),
            }),
        }),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    // RHS should not execute after break
    assert_eq!(shell_state.get_var("output"), Some("".to_string()));
}

#[test]
fn test_or_with_continue_in_lhs() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("output", "".to_string());
    
    // for i in 1 2 3; do
    //   (false || continue) || output="${output}bad"
    //   output="${output}$i"
    // done
    let ast = Ast::For {
        variable: "i".to_string(),
        items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        body: Box::new(Ast::Sequence(vec![
            Ast::Or {
                left: Box::new(Ast::Pipeline(vec![ShellCommand {
                    args: vec!["false".to_string()],
                    redirections: vec![],
                    compound: None,
                }])),
                right: Box::new(Ast::Pipeline(vec![ShellCommand {
                    args: vec!["continue".to_string()],
                    redirections: vec![],
                    compound: None,
                }])),
            },
            Ast::Assignment {
                var: "output".to_string(),
                value: "${output}$i".to_string(),
            },
        ])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    // Both RHS of || and subsequent assignment should not execute
    assert_eq!(shell_state.get_var("output"), Some("".to_string()));
}

#[test]
fn test_logical_chain_flag_cleanup() {
    let mut shell_state = ShellState::new();
    
    // Verify in_logical_chain is false initially
    assert!(!shell_state.in_logical_chain);
    
    // Execute a simple && chain
    let ast = Ast::And {
        left: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            redirections: vec![],
            compound: None,
        }])),
        right: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    execute(ast, &mut shell_state);
    
    // Verify in_logical_chain is reset to false after execution
    assert!(!shell_state.in_logical_chain);
}

#[test]
fn test_logical_chain_flag_cleanup_with_return() {
    let mut shell_state = ShellState::new();
    
    // Define a function that returns
    shell_state.define_function(
        "test_return".to_string(),
        Ast::Return { value: Some("0".to_string()) },
    );
    
    // Execute && chain with return in LHS
    let ast = Ast::And {
        left: Box::new(Ast::FunctionCall {
            name: "test_return".to_string(),
            args: vec![],
        }),
        right: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["echo".to_string(), "should not execute".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    // Execute in function context
    shell_state.enter_function();
    execute(ast, &mut shell_state);
    shell_state.exit_function();
    
    // Verify in_logical_chain is reset even with early return
    assert!(!shell_state.in_logical_chain);
}