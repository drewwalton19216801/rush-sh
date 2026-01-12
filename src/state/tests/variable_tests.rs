//! Tests for variable scoping and management

use crate::state::ShellState;

#[test]
fn test_local_variable_scoping() {
    let mut state = ShellState::new();

    // Set a global variable
    state.set_var("global_var", "global_value".to_string());
    assert_eq!(
        state.get_var("global_var"),
        Some("global_value".to_string())
    );

    // Push local scope
    state.push_local_scope();

    // Set a local variable with the same name
    state.set_local_var("global_var", "local_value".to_string());
    assert_eq!(state.get_var("global_var"), Some("local_value".to_string()));

    // Set another local variable
    state.set_local_var("local_var", "local_only".to_string());
    assert_eq!(state.get_var("local_var"), Some("local_only".to_string()));

    // Pop local scope
    state.pop_local_scope();

    // Should be back to global variable
    assert_eq!(
        state.get_var("global_var"),
        Some("global_value".to_string())
    );
    assert_eq!(state.get_var("local_var"), None);
}

#[test]
fn test_nested_local_scopes() {
    let mut state = ShellState::new();

    // Set global variable
    state.set_var("test_var", "global".to_string());

    // Push first local scope
    state.push_local_scope();
    state.set_local_var("test_var", "level1".to_string());
    assert_eq!(state.get_var("test_var"), Some("level1".to_string()));

    // Push second local scope
    state.push_local_scope();
    state.set_local_var("test_var", "level2".to_string());
    assert_eq!(state.get_var("test_var"), Some("level2".to_string()));

    // Pop second scope
    state.pop_local_scope();
    assert_eq!(state.get_var("test_var"), Some("level1".to_string()));

    // Pop first scope
    state.pop_local_scope();
    assert_eq!(state.get_var("test_var"), Some("global".to_string()));
}

#[test]
fn test_variable_set_in_local_scope() {
    let mut state = ShellState::new();

    // No local scope initially
    state.set_var("test_var", "global".to_string());
    assert_eq!(state.get_var("test_var"), Some("global".to_string()));

    // Push local scope and set local variable
    state.push_local_scope();
    state.set_local_var("test_var", "local".to_string());
    assert_eq!(state.get_var("test_var"), Some("local".to_string()));

    // Pop scope
    state.pop_local_scope();
    assert_eq!(state.get_var("test_var"), Some("global".to_string()));
}

#[test]
fn test_positional_parameters() {
    let mut state = ShellState::new();
    state.set_positional_params(vec![
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]);

    assert_eq!(state.get_var("1"), Some("arg1".to_string()));
    assert_eq!(state.get_var("2"), Some("arg2".to_string()));
    assert_eq!(state.get_var("3"), Some("arg3".to_string()));
    assert_eq!(state.get_var("4"), None);
    assert_eq!(state.get_var("#"), Some("3".to_string()));
    assert_eq!(state.get_var("*"), Some("arg1 arg2 arg3".to_string()));
    assert_eq!(state.get_var("@"), Some("arg1 arg2 arg3".to_string()));
}

#[test]
fn test_positional_parameters_empty() {
    let mut state = ShellState::new();
    state.set_positional_params(vec![]);

    assert_eq!(state.get_var("1"), None);
    assert_eq!(state.get_var("#"), Some("0".to_string()));
    assert_eq!(state.get_var("*"), Some("".to_string()));
    assert_eq!(state.get_var("@"), Some("".to_string()));
}

#[test]
fn test_shift_positional_params() {
    let mut state = ShellState::new();
    state.set_positional_params(vec![
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]);

    assert_eq!(state.get_var("1"), Some("arg1".to_string()));
    assert_eq!(state.get_var("2"), Some("arg2".to_string()));
    assert_eq!(state.get_var("3"), Some("arg3".to_string()));

    state.shift_positional_params(1);

    assert_eq!(state.get_var("1"), Some("arg2".to_string()));
    assert_eq!(state.get_var("2"), Some("arg3".to_string()));
    assert_eq!(state.get_var("3"), None);
    assert_eq!(state.get_var("#"), Some("2".to_string()));

    state.shift_positional_params(2);

    assert_eq!(state.get_var("1"), None);
    assert_eq!(state.get_var("#"), Some("0".to_string()));
}

#[test]
fn test_push_positional_param() {
    let mut state = ShellState::new();
    state.set_positional_params(vec!["arg1".to_string()]);

    assert_eq!(state.get_var("1"), Some("arg1".to_string()));
    assert_eq!(state.get_var("#"), Some("1".to_string()));

    state.push_positional_param("arg2".to_string());

    assert_eq!(state.get_var("1"), Some("arg1".to_string()));
    assert_eq!(state.get_var("2"), Some("arg2".to_string()));
    assert_eq!(state.get_var("#"), Some("2".to_string()));
}