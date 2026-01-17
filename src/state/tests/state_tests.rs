//! Tests for general state management

use crate::state::ShellState;
use std::env;
use std::sync::Mutex;

// Mutex to serialize tests that modify environment variables
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_shell_state_basic() {
    let mut state = ShellState::new();
    state.set_var("TEST_VAR", "test_value".to_string());
    assert_eq!(state.get_var("TEST_VAR"), Some("test_value".to_string()));
}

#[test]
fn test_special_variables() {
    let mut state = ShellState::new();
    state.set_last_exit_code(42);
    state.set_script_name("test_script");

    assert_eq!(state.get_var("?"), Some("42".to_string()));
    assert_eq!(state.get_var("$"), Some(state.shell_pid.to_string()));
    assert_eq!(state.get_var("0"), Some("test_script".to_string()));
}

#[test]
fn test_export_variable() {
    let mut state = ShellState::new();
    state.set_var("EXPORT_VAR", "export_value".to_string());
    state.export_var("EXPORT_VAR");

    let child_env = state.get_env_for_child();
    assert_eq!(
        child_env.get("EXPORT_VAR"),
        Some(&"export_value".to_string())
    );
}

#[test]
fn test_unset_variable() {
    let mut state = ShellState::new();
    state.set_var("UNSET_VAR", "value".to_string());
    state.export_var("UNSET_VAR");

    assert!(state.variables.contains_key("UNSET_VAR"));
    assert!(state.exported.contains("UNSET_VAR"));

    state.unset_var("UNSET_VAR");

    assert!(!state.variables.contains_key("UNSET_VAR"));
    assert!(!state.exported.contains("UNSET_VAR"));
}

#[test]
fn test_get_user_hostname() {
    let state = ShellState::new();
    let user_hostname = state.get_user_hostname();
    // Should contain @ since it's user@hostname format
    assert!(user_hostname.contains('@'));
}

#[test]
fn test_get_prompt() {
    let state = ShellState::new();
    let prompt = state.get_prompt();
    // Should end with $ and contain @
    assert!(prompt.ends_with(" $ "));
    assert!(prompt.contains('@'));
}

#[test]
fn test_condensed_cwd_environment_variable() {
    let _lock = ENV_LOCK.lock().unwrap();
    
    // Save original state
    let original_rush_condensed = env::var("RUSH_CONDENSED").ok();
    
    // Test default behavior (should be true for backward compatibility)
    unsafe {
        env::remove_var("RUSH_CONDENSED");
    }
    let state = ShellState::new();
    assert!(state.condensed_cwd);

    // Test explicit true
    unsafe {
        env::set_var("RUSH_CONDENSED", "true");
    }
    let state = ShellState::new();
    assert!(state.condensed_cwd);

    // Test explicit false
    unsafe {
        env::set_var("RUSH_CONDENSED", "false");
    }
    let state = ShellState::new();
    assert!(!state.condensed_cwd);

    // Restore original state
    unsafe {
        if let Some(val) = original_rush_condensed {
            env::set_var("RUSH_CONDENSED", val);
        } else {
            env::remove_var("RUSH_CONDENSED");
        }
    }
}

#[test]
fn test_get_full_cwd() {
    let state = ShellState::new();
    let full_cwd = state.get_full_cwd();
    assert!(!full_cwd.is_empty());
    // Should contain path separators (either / or \ depending on platform)
    assert!(full_cwd.contains('/') || full_cwd.contains('\\'));
}

#[test]
fn test_prompt_with_condensed_setting() {
    let _lock = ENV_LOCK.lock().unwrap();
    
    // Save original state
    let original_rush_condensed = env::var("RUSH_CONDENSED").ok();
    
    // Ensure RUSH_CONDENSED is not set so we get the default behavior
    unsafe {
        env::remove_var("RUSH_CONDENSED");
    }
    
    let mut state = ShellState::new();

    // Test with condensed enabled (default)
    assert!(state.condensed_cwd);
    let prompt_condensed = state.get_prompt();
    assert!(prompt_condensed.contains('@'));

    // Test with condensed disabled
    state.condensed_cwd = false;
    let prompt_full = state.get_prompt();
    assert!(prompt_full.contains('@'));

    // Both should end with "$ " (or "# " for root)
    assert!(prompt_condensed.ends_with("$ ") || prompt_condensed.ends_with("# "));
    assert!(prompt_full.ends_with("$ ") || prompt_full.ends_with("# "));
    
    // Restore original state
    unsafe {
        if let Some(val) = original_rush_condensed {
            env::set_var("RUSH_CONDENSED", val);
        } else {
            env::remove_var("RUSH_CONDENSED");
        }
    }
}

#[test]
fn test_lineno_special_variable() {
    let mut state = ShellState::new();
    state.current_line_number = 42;
    
    assert_eq!(state.get_var("LINENO"), Some("42".to_string()));
}