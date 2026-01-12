//! Basic tokenization tests

use super::super::*;
use crate::state::ShellState;

#[test]
fn test_basic_word() {
    let shell_state = ShellState::new();
    let result = lex("ls", &shell_state).unwrap();
    assert_eq!(result, vec![Token::Word("ls".to_string())]);
}

#[test]
fn test_multiple_words() {
    let shell_state = ShellState::new();
    let result = lex("ls -la", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("ls".to_string()),
            Token::Word("-la".to_string())
        ]
    );
}

#[test]
fn test_pipe() {
    let shell_state = ShellState::new();
    let result = lex("ls | grep txt", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("ls".to_string()),
            Token::Pipe,
            Token::Word("grep".to_string()),
            Token::Word("txt".to_string())
        ]
    );
}

#[test]
fn test_empty_input() {
    let shell_state = ShellState::new();
    let result = lex("", &shell_state).unwrap();
    assert_eq!(result, Vec::<Token>::new());
}

#[test]
fn test_only_spaces() {
    let shell_state = ShellState::new();
    let result = lex("   ", &shell_state).unwrap();
    assert_eq!(result, Vec::<Token>::new());
}

#[test]
fn test_complex_pipeline() {
    let shell_state = ShellState::new();
    let result = lex(
        "cat input.txt | grep \"search term\" > output.txt",
        &shell_state,
    )
    .unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("cat".to_string()),
            Token::Word("input.txt".to_string()),
            Token::Pipe,
            Token::Word("grep".to_string()),
            Token::Word("search term".to_string()),
            Token::RedirOut,
            Token::Word("output.txt".to_string())
        ]
    );
}

#[test]
fn test_if_tokens() {
    let shell_state = ShellState::new();
    let result = lex("if true; then printf yes; fi", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::If,
            Token::Word("true".to_string()),
            Token::Semicolon,
            Token::Then,
            Token::Word("printf".to_string()),
            Token::Word("yes".to_string()),
            Token::Semicolon,
            Token::Fi,
        ]
    );
}

#[test]
fn test_local_keyword() {
    let shell_state = ShellState::new();
    let result = lex("local myvar", &shell_state).unwrap();
    assert_eq!(result, vec![Token::Local, Token::Word("myvar".to_string())]);
}

#[test]
fn test_local_keyword_in_function() {
    let shell_state = ShellState::new();
    let result = lex("local var=value", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![Token::Local, Token::Word("var=value".to_string())]
    );
}

#[test]
fn test_semicolons_outside_quotes() {
    // Test that semicolons outside quotes still work as command separators
    let shell_state = ShellState::new();
    let result = lex("echo hello; echo world", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("hello".to_string()),
            Token::Semicolon,
            Token::Word("echo".to_string()),
            Token::Word("world".to_string())
        ]
    );
}

// ===== Bang (!) Context-Aware Tests =====

#[test]
fn test_bang_as_argument() {
    // ! should be treated as a regular word when not at command start
    let shell_state = ShellState::new();
    let result = lex("echo !", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("!".to_string())
        ]
    );
}

#[test]
fn test_bang_as_argument_middle() {
    let shell_state = ShellState::new();
    let result = lex("echo hello !", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("hello".to_string()),
            Token::Word("!".to_string())
        ]
    );
}

#[test]
fn test_bang_as_argument_multiple() {
    let shell_state = ShellState::new();
    let result = lex("echo ! ! !", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("!".to_string()),
            Token::Word("!".to_string()),
            Token::Word("!".to_string())
        ]
    );
}

#[test]
fn test_bang_at_command_start() {
    // ! at command start should be Token::Bang for negation
    let shell_state = ShellState::new();
    let result = lex("! echo hello", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Bang,
            Token::Word("echo".to_string()),
            Token::Word("hello".to_string())
        ]
    );
}

#[test]
fn test_bang_after_semicolon() {
    let shell_state = ShellState::new();
    let result = lex("echo hello; ! false", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("hello".to_string()),
            Token::Semicolon,
            Token::Bang,
            Token::Word("false".to_string())
        ]
    );
}

#[test]
fn test_bang_after_newline() {
    let shell_state = ShellState::new();
    let result = lex("echo hello\n! false", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("hello".to_string()),
            Token::Newline,
            Token::Bang,
            Token::Word("false".to_string())
        ]
    );
}

#[test]
fn test_bang_after_and_operator() {
    let shell_state = ShellState::new();
    let result = lex("true && ! false", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("true".to_string()),
            Token::And,
            Token::Bang,
            Token::Word("false".to_string())
        ]
    );
}

#[test]
fn test_bang_after_or_operator() {
    let shell_state = ShellState::new();
    let result = lex("false || ! false", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("false".to_string()),
            Token::Or,
            Token::Bang,
            Token::Word("false".to_string())
        ]
    );
}

#[test]
fn test_bang_after_pipe() {
    let shell_state = ShellState::new();
    let result = lex("echo test | ! grep test", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("test".to_string()),
            Token::Pipe,
            Token::Word("!".to_string()),
            Token::Word("grep".to_string()),
            Token::Word("test".to_string())
        ]
    );
}

#[test]
fn test_bang_mixed_contexts() {
    // Test both negation and argument usage in same line
    let shell_state = ShellState::new();
    let result = lex("! echo ! && echo !", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Bang,
            Token::Word("echo".to_string()),
            Token::Word("!".to_string()),
            Token::And,
            Token::Word("echo".to_string()),
            Token::Word("!".to_string())
        ]
    );
}

#[test]
fn test_bang_after_then() {
    let shell_state = ShellState::new();
    let result = lex("if true; then ! false; fi", &shell_state).unwrap();
    assert!(result.contains(&Token::Bang));
    // Verify Bang comes after Then
    let then_pos = result.iter().position(|t| t == &Token::Then).unwrap();
    let bang_pos = result.iter().position(|t| t == &Token::Bang).unwrap();
    assert!(bang_pos > then_pos);
}

#[test]
fn test_bang_after_else() {
    let shell_state = ShellState::new();
    let result = lex("if false; then echo a; else ! true; fi", &shell_state).unwrap();
    assert!(result.contains(&Token::Bang));
    // Verify Bang comes after Else
    let else_pos = result.iter().position(|t| t == &Token::Else).unwrap();
    let bang_pos = result.iter().position(|t| t == &Token::Bang).unwrap();
    assert!(bang_pos > else_pos);
}

#[test]
fn test_bang_after_do() {
    let shell_state = ShellState::new();
    let result = lex("while true; do ! false; done", &shell_state).unwrap();
    assert!(result.contains(&Token::Bang));
    // Verify Bang comes after Do
    let do_pos = result.iter().position(|t| t == &Token::Do).unwrap();
    let bang_pos = result.iter().position(|t| t == &Token::Bang).unwrap();
    assert!(bang_pos > do_pos);
}

#[test]
fn test_bang_in_subshell() {
    let shell_state = ShellState::new();
    let result = lex("(! echo test)", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::LeftParen,
            Token::Bang,
            Token::Word("echo".to_string()),
            Token::Word("test".to_string()),
            Token::RightParen
        ]
    );
}

#[test]
fn test_bang_in_command_group() {
    let shell_state = ShellState::new();
    let result = lex("{ ! echo test; }", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::LeftBrace,
            Token::Bang,
            Token::Word("echo".to_string()),
            Token::Word("test".to_string()),
            Token::Semicolon,
            Token::RightBrace
        ]
    );
}

#[test]
fn test_bang_as_part_of_word() {
    // When ! is part of a word (no space before it), it should be included in the word
    let shell_state = ShellState::new();
    let result = lex("echo hello!", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("hello!".to_string())
        ]
    );
}