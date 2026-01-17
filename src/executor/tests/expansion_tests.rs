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

#[test]
fn test_last_background_pid_expansion() {
    let mut shell_state = ShellState::new();
    
    // Initially, $! should expand to empty string
    let expanded = expand_variables_in_string("PID: $!", &mut shell_state);
    assert_eq!(expanded, "PID: ");
    
    // Set last background PID
    shell_state.last_background_pid = Some(1234);
    let expanded = expand_variables_in_string("PID: $!", &mut shell_state);
    assert_eq!(expanded, "PID: 1234");
    
    // Test with brace syntax
    let expanded = expand_variables_in_string("PID: ${!}", &mut shell_state);
    assert_eq!(expanded, "PID: 1234");
}

#[test]
fn test_last_background_pid_in_arithmetic() {
    let mut shell_state = ShellState::new();
    
    // Set last background PID
    shell_state.last_background_pid = Some(100);
    
    // Test in arithmetic expansion
    let expanded = expand_variables_in_string("Result: $(($! + 50))", &mut shell_state);
    assert_eq!(expanded, "Result: 150");
}

#[test]
fn test_last_background_pid_multiple_references() {
    let mut shell_state = ShellState::new();
    shell_state.last_background_pid = Some(5678);
    
    // Test multiple references in one string
    let expanded = expand_variables_in_string("PID $! and again $! and ${!}", &mut shell_state);
    assert_eq!(expanded, "PID 5678 and again 5678 and 5678");
}