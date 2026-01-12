//! Quote handling tests

use super::super::*;
use crate::state::ShellState;

#[test]
fn test_double_quotes() {
    let shell_state = ShellState::new();
    let result = lex("echo \"hello world\"", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("hello world".to_string())
        ]
    );
}

#[test]
fn test_single_quotes() {
    let shell_state = ShellState::new();
    let result = lex("echo 'hello world'", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("hello world".to_string())
        ]
    );
}

#[test]
fn test_unclosed_double_quote() {
    // Lexer doesn't handle unclosed quotes as errors, just treats as literal
    let shell_state = ShellState::new();
    let result = lex("echo \"hello", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("hello".to_string())
        ]
    );
}

#[test]
fn test_single_quotes_with_semicolons() {
    // Test that semicolons inside single quotes are preserved as part of the string
    let shell_state = ShellState::new();
    let result = lex("trap 'echo \"A\"; echo \"B\"' EXIT", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("trap".to_string()),
            Token::Word("echo \"A\"; echo \"B\"".to_string()),
            Token::Word("EXIT".to_string())
        ]
    );
}

#[test]
fn test_double_quotes_with_semicolons() {
    // Test that semicolons inside double quotes are preserved as part of the string
    let shell_state = ShellState::new();
    let result = lex("echo \"command1; command2\"", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("command1; command2".to_string())
        ]
    );
}

#[test]
fn test_bang_in_single_quotes() {
    let shell_state = ShellState::new();
    let result = lex("echo '!'", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("!".to_string())
        ]
    );
}

#[test]
fn test_bang_in_double_quotes() {
    let shell_state = ShellState::new();
    let result = lex("echo \"!\"", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("!".to_string())
        ]
    );
}