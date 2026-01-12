//! Tests for compound commands (subshells and command groups)

use crate::lexer::Token;
use crate::parser::{parse, Ast};

#[test]
fn test_subshell_with_and_operator_and_sequence() {
    // Test: (cmd1) && cmd2 ; cmd3
    // Should parse as: Sequence([(cmd1) && cmd2, cmd3])
    let tokens = vec![
        Token::LeftParen,
        Token::Word("true".to_string()),
        Token::RightParen,
        Token::And,
        Token::Word("echo".to_string()),
        Token::Word("second".to_string()),
        Token::Semicolon,
        Token::Word("echo".to_string()),
        Token::Word("third".to_string()),
    ];
    let result = parse(tokens).unwrap();

    // Should be a Sequence
    if let Ast::Sequence(commands) = result {
        assert_eq!(commands.len(), 2);

        // First should be (true) && echo second
        if let Ast::And { .. } = &commands[0] {
            // Structure is correct
        } else {
            panic!("Expected And node");
        }

        // Second should be echo third
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
fn test_command_group_with_or_operator_and_sequence() {
    // Test: { cmd1; } || cmd2 ; cmd3
    // Should parse as: Sequence([{ cmd1; } || cmd2, cmd3])
    let tokens = vec![
        Token::LeftBrace,
        Token::Word("false".to_string()),
        Token::Semicolon,
        Token::RightBrace,
        Token::Or,
        Token::Word("echo".to_string()),
        Token::Word("second".to_string()),
        Token::Semicolon,
        Token::Word("echo".to_string()),
        Token::Word("third".to_string()),
    ];
    let result = parse(tokens).unwrap();

    // Should be a Sequence
    if let Ast::Sequence(commands) = result {
        assert_eq!(commands.len(), 2);

        // First should be { false; } || echo second
        if let Ast::Or { .. } = &commands[0] {
            // Structure is correct
        } else {
            panic!("Expected Or node");
        }

        // Second should be echo third
        if let Ast::Pipeline(cmds) = &commands[1] {
            assert_eq!(cmds[0].args, vec!["echo", "third"]);
        } else {
            panic!("Expected Pipeline");
        }
    } else {
        panic!("Expected Sequence, got: {:?}", result);
    }
}