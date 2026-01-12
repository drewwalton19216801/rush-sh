//! Tests for logical operators (&&, ||) and negation (!)

use crate::lexer::Token;
use crate::parser::{parse, Ast};

#[test]
fn test_negation_with_and_operator() {
    // Test: ! cmd1 && cmd2
    // Should parse as: (! cmd1) && cmd2
    let tokens = vec![
        Token::Bang,
        Token::Word("false".to_string()),
        Token::And,
        Token::Word("echo".to_string()),
        Token::Word("success".to_string()),
    ];
    let result = parse(tokens).unwrap();

    // Should be an And node with negation on the left
    if let Ast::And { left, right } = result {
        // Left should be a negation
        if let Ast::Negation { command } = *left {
            if let Ast::Pipeline(cmds) = *command {
                assert_eq!(cmds[0].args, vec!["false"]);
            } else {
                panic!("Expected Pipeline in negation");
            }
        } else {
            panic!("Expected Negation on left side of And");
        }

        // Right should be echo success
        if let Ast::Pipeline(cmds) = *right {
            assert_eq!(cmds[0].args, vec!["echo", "success"]);
        } else {
            panic!("Expected Pipeline on right side of And");
        }
    } else {
        panic!("Expected And node, got: {:?}", result);
    }
}

#[test]
fn test_negation_with_or_operator() {
    // Test: ! cmd1 || cmd2
    // Should parse as: (! cmd1) || cmd2
    let tokens = vec![
        Token::Bang,
        Token::Word("true".to_string()),
        Token::Or,
        Token::Word("echo".to_string()),
        Token::Word("fallback".to_string()),
    ];
    let result = parse(tokens).unwrap();

    // Should be an Or node with negation on the left
    if let Ast::Or { left, right } = result {
        // Left should be a negation
        if let Ast::Negation { command } = *left {
            if let Ast::Pipeline(cmds) = *command {
                assert_eq!(cmds[0].args, vec!["true"]);
            } else {
                panic!("Expected Pipeline in negation");
            }
        } else {
            panic!("Expected Negation on left side of Or");
        }

        // Right should be echo fallback
        if let Ast::Pipeline(cmds) = *right {
            assert_eq!(cmds[0].args, vec!["echo", "fallback"]);
        } else {
            panic!("Expected Pipeline on right side of Or");
        }
    } else {
        panic!("Expected Or node, got: {:?}", result);
    }
}

#[test]
fn test_negation_and_semicolon_sequence() {
    // Test: ! cmd1 && cmd2 ; cmd3
    // Should parse as: Sequence([(! cmd1) && cmd2, cmd3])
    let tokens = vec![
        Token::Bang,
        Token::Word("false".to_string()),
        Token::And,
        Token::Word("echo".to_string()),
        Token::Word("second".to_string()),
        Token::Semicolon,
        Token::Word("echo".to_string()),
        Token::Word("third".to_string()),
    ];
    let result = parse(tokens).unwrap();

    // Should be a Sequence with two commands
    if let Ast::Sequence(commands) = result {
        assert_eq!(commands.len(), 2);

        // First command should be (! false) && echo second
        if let Ast::And { left, right } = &commands[0] {
            if let Ast::Negation { command } = &**left {
                if let Ast::Pipeline(cmds) = &**command {
                    assert_eq!(cmds[0].args, vec!["false"]);
                } else {
                    panic!("Expected Pipeline in negation");
                }
            } else {
                panic!("Expected Negation");
            }

            if let Ast::Pipeline(cmds) = &**right {
                assert_eq!(cmds[0].args, vec!["echo", "second"]);
            } else {
                panic!("Expected Pipeline");
            }
        } else {
            panic!("Expected And node");
        }

        // Second command should be echo third
        if let Ast::Pipeline(cmds) = &commands[1] {
            assert_eq!(cmds[0].args, vec!["echo", "third"]);
        } else {
            panic!("Expected Pipeline");
        }
    } else {
        panic!("Expected Sequence, got: {:?}", result);
    }
}

#[test]
fn test_nested_logical_operators() {
    // Test: cmd1 && ! cmd2 || cmd3
    // Should parse as: (cmd1 && (! cmd2)) || cmd3
    let tokens = vec![
        Token::Word("true".to_string()),
        Token::And,
        Token::Bang,
        Token::Word("false".to_string()),
        Token::Or,
        Token::Word("echo".to_string()),
        Token::Word("fallback".to_string()),
    ];
    let result = parse(tokens).unwrap();

    // Should be an Or node
    if let Ast::Or { left, right } = result {
        // Left should be: true && ! false
        if let Ast::And {
            left: and_left,
            right: and_right,
        } = *left
        {
            // and_left should be true
            if let Ast::Pipeline(cmds) = *and_left {
                assert_eq!(cmds[0].args, vec!["true"]);
            } else {
                panic!("Expected Pipeline");
            }

            // and_right should be ! false
            if let Ast::Negation { command } = *and_right {
                if let Ast::Pipeline(cmds) = *command {
                    assert_eq!(cmds[0].args, vec!["false"]);
                } else {
                    panic!("Expected Pipeline in negation");
                }
            } else {
                panic!("Expected Negation");
            }
        } else {
            panic!("Expected And node on left side of Or");
        }

        // Right should be echo fallback
        if let Ast::Pipeline(cmds) = *right {
            assert_eq!(cmds[0].args, vec!["echo", "fallback"]);
        } else {
            panic!("Expected Pipeline");
        }
    } else {
        panic!("Expected Or node, got: {:?}", result);
    }
}

#[test]
fn test_multiple_and_operators_in_sequence() {
    // Test: cmd1 && cmd2 && cmd3
    // Should parse as: (cmd1 && cmd2) && cmd3 (left-associative)
    let tokens = vec![
        Token::Word("true".to_string()),
        Token::And,
        Token::Word("echo".to_string()),
        Token::Word("second".to_string()),
        Token::And,
        Token::Word("echo".to_string()),
        Token::Word("third".to_string()),
    ];
    let result = parse(tokens).unwrap();

    // Should be an And node with left-associative structure
    if let Ast::And { left, right } = result {
        // Left should be: true && echo second
        if let Ast::And {
            left: inner_left,
            right: inner_right,
        } = *left
        {
            if let Ast::Pipeline(cmds) = *inner_left {
                assert_eq!(cmds[0].args, vec!["true"]);
            } else {
                panic!("Expected Pipeline");
            }

            if let Ast::Pipeline(cmds) = *inner_right {
                assert_eq!(cmds[0].args, vec!["echo", "second"]);
            } else {
                panic!("Expected Pipeline");
            }
        } else {
            panic!("Expected nested And node on left");
        }

        // Right should be: echo third
        if let Ast::Pipeline(cmds) = *right {
            assert_eq!(cmds[0].args, vec!["echo", "third"]);
        } else {
            panic!("Expected Pipeline on right");
        }
    } else {
        panic!("Expected And node, got: {:?}", result);
    }
}

#[test]
fn test_negation_in_pipeline() {
    // Test: ! cmd1 | cmd2
    // Negation should apply to the entire pipeline
    let tokens = vec![
        Token::Bang,
        Token::Word("grep".to_string()),
        Token::Word("pattern".to_string()),
        Token::Pipe,
        Token::Word("wc".to_string()),
        Token::Word("-l".to_string()),
    ];
    let result = parse(tokens).unwrap();

    // Should be a Negation wrapping a Pipeline
    if let Ast::Negation { command } = result {
        if let Ast::Pipeline(cmds) = *command {
            assert_eq!(cmds.len(), 2);
            assert_eq!(cmds[0].args, vec!["grep", "pattern"]);
            assert_eq!(cmds[1].args, vec!["wc", "-l"]);
        } else {
            panic!("Expected Pipeline in negation");
        }
    } else {
        panic!("Expected Negation, got: {:?}", result);
    }
}