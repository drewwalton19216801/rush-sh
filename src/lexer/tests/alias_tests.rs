//! Alias expansion tests

use super::super::*;
use crate::state::ShellState;
use std::collections::HashSet;

#[test]
fn test_expand_aliases_simple() {
    let mut shell_state = ShellState::new();
    shell_state.set_alias("ll", "ls -l".to_string());
    let tokens = vec![Token::Word("ll".to_string())];
    let result = expand_aliases(tokens, &shell_state, &mut HashSet::new()).unwrap();
    assert_eq!(
        result,
        vec![Token::Word("ls".to_string()), Token::Word("-l".to_string())]
    );
}

#[test]
fn test_expand_aliases_with_args() {
    let mut shell_state = ShellState::new();
    shell_state.set_alias("ll", "ls -l".to_string());
    let tokens = vec![
        Token::Word("ll".to_string()),
        Token::Word("/tmp".to_string()),
    ];
    let result = expand_aliases(tokens, &shell_state, &mut HashSet::new()).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("ls".to_string()),
            Token::Word("-l".to_string()),
            Token::Word("/tmp".to_string())
        ]
    );
}

#[test]
fn test_expand_aliases_no_alias() {
    let shell_state = ShellState::new();
    let tokens = vec![Token::Word("ls".to_string())];
    let result = expand_aliases(tokens.clone(), &shell_state, &mut HashSet::new()).unwrap();
    assert_eq!(result, tokens);
}

#[test]
fn test_expand_aliases_chained() {
    // Test that chained aliases work correctly: a -> b -> a (command)
    // This is NOT recursion in bash - it expands a to b, then b to a (the command),
    // and then tries to execute command 'a' which doesn't exist.
    let mut shell_state = ShellState::new();
    shell_state.set_alias("a", "b".to_string());
    shell_state.set_alias("b", "a".to_string());
    let tokens = vec![Token::Word("a".to_string())];
    let result = expand_aliases(tokens, &shell_state, &mut HashSet::new());
    // Should succeed and expand to just "a" (the command, not the alias)
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), vec![Token::Word("a".to_string())]);
}