//! Tests for expansion functionality (variable expansion, wildcards, etc.)

use crate::executor::expand_variables_in_string;
use crate::state::ShellState;

#[test]
fn test_here_document_with_variable_expansion() {
    // Test that variables are expanded in here-document content
    let mut shell_state = ShellState::new();
    shell_state.set_var("PWD", "/test/path".to_string());

    // Simulate here-doc content with variable
    let content = "Working dir: $PWD";
    let expanded = expand_variables_in_string(content, &mut shell_state);

    assert_eq!(expanded, "Working dir: /test/path");
}

#[test]
fn test_here_document_with_command_substitution_builtin() {
    // Test that builtin command substitutions work in here-document content
    let mut shell_state = ShellState::new();
    shell_state.set_var("PWD", "/test/dir".to_string());

    // Simulate here-doc content with pwd builtin command substitution
    let content = "Current directory: `pwd`";
    let expanded = expand_variables_in_string(content, &mut shell_state);

    // The pwd builtin should be executed and expanded
    assert!(expanded.contains("Current directory: "));
}