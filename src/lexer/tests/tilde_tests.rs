//! Tilde expansion tests

use super::super::*;
use crate::state::ShellState;
use std::env;
use std::sync::Mutex;

// Mutex to serialize tests that modify environment variables
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_tilde_expansion_unquoted() {
    let _lock = ENV_LOCK.lock().unwrap();
    let shell_state = ShellState::new();
    let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    let result = lex("echo ~", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![Token::Word("echo".to_string()), Token::Word(home)]
    );
}

#[test]
fn test_tilde_expansion_single_quoted() {
    let shell_state = ShellState::new();
    let result = lex("echo '~'", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("~".to_string())
        ]
    );
}

#[test]
fn test_tilde_expansion_double_quoted() {
    let shell_state = ShellState::new();
    let result = lex("echo \"~\"", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("~".to_string())
        ]
    );
}

#[test]
fn test_tilde_expansion_mixed_quotes() {
    let _lock = ENV_LOCK.lock().unwrap();
    let shell_state = ShellState::new();
    let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    let result = lex("echo ~ '~' \"~\"", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word(home),
            Token::Word("~".to_string()),
            Token::Word("~".to_string())
        ]
    );
}

#[test]
fn test_tilde_expansion_pwd() {
    let mut shell_state = ShellState::new();

    // Set PWD variable
    let test_pwd = "/test/current/dir";
    shell_state.set_var("PWD", test_pwd.to_string());

    let result = lex("echo ~+", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word(test_pwd.to_string())
        ]
    );
}

#[test]
fn test_tilde_expansion_oldpwd() {
    let mut shell_state = ShellState::new();

    // Set OLDPWD variable
    let test_oldpwd = "/test/old/dir";
    shell_state.set_var("OLDPWD", test_oldpwd.to_string());

    let result = lex("echo ~-", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word(test_oldpwd.to_string())
        ]
    );
}

#[test]
fn test_tilde_expansion_pwd_unset() {
    let _lock = ENV_LOCK.lock().unwrap();
    let shell_state = ShellState::new();

    // When PWD is not set, ~+ should expand to current directory
    let result = lex("echo ~+", &shell_state).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], Token::Word("echo".to_string()));

    // The second token should be a valid path (either from env::current_dir or literal ~+)
    if let Token::Word(path) = &result[1] {
        // Should either be a path or the literal ~+
        assert!(path.starts_with('/') || path == "~+");
    } else {
        panic!("Expected Word token");
    }
}

#[test]
fn test_tilde_expansion_oldpwd_unset() {
    // Lock to prevent parallel tests from interfering with environment variables
    let _lock = ENV_LOCK.lock().unwrap();

    // Save and clear OLDPWD
    let original_oldpwd = env::var("OLDPWD").ok();
    unsafe {
        env::remove_var("OLDPWD");
    }

    let shell_state = ShellState::new();

    // When OLDPWD is not set, ~- should remain as literal
    let result = lex("echo ~-", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("~-".to_string())
        ]
    );

    // Restore OLDPWD
    unsafe {
        if let Some(oldpwd) = original_oldpwd {
            env::set_var("OLDPWD", oldpwd);
        }
    }
}

#[test]
fn test_tilde_expansion_pwd_in_quotes() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("PWD", "/test/dir".to_string());

    // Single quotes should prevent expansion
    let result = lex("echo '~+'", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("~+".to_string())
        ]
    );

    // Double quotes should also prevent expansion
    let result = lex("echo \"~+\"", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("~+".to_string())
        ]
    );
}

#[test]
fn test_tilde_expansion_oldpwd_in_quotes() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("OLDPWD", "/test/old".to_string());

    // Single quotes should prevent expansion
    let result = lex("echo '~-'", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("~-".to_string())
        ]
    );

    // Double quotes should also prevent expansion
    let result = lex("echo \"~-\"", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("~-".to_string())
        ]
    );
}

#[test]
fn test_tilde_expansion_mixed() {
    let _lock = ENV_LOCK.lock().unwrap();
    let mut shell_state = ShellState::new();
    let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    shell_state.set_var("PWD", "/current".to_string());
    shell_state.set_var("OLDPWD", "/previous".to_string());

    let result = lex("echo ~ ~+ ~-", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word(home),
            Token::Word("/current".to_string()),
            Token::Word("/previous".to_string())
        ]
    );
}

#[test]
fn test_tilde_expansion_not_at_start() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("PWD", "/test".to_string());

    // Tilde should not expand when not at start of word
    let result = lex("echo prefix~+", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("prefix~+".to_string())
        ]
    );
}

#[test]
fn test_tilde_expansion_username() {
    let shell_state = ShellState::new();

    // Test with root username (special case: /root instead of /home/root)
    let result = lex("echo ~root", &shell_state).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], Token::Word("echo".to_string()));

    // The expansion should either be /root or literal ~root (if /root doesn't exist)
    if let Token::Word(path) = &result[1] {
        assert!(path == "/root" || path == "~root");
    } else {
        panic!("Expected Word token");
    }
}

#[test]
fn test_tilde_expansion_username_with_path() {
    let shell_state = ShellState::new();

    // Test ~username/path expansion
    let result = lex("echo ~root/documents", &shell_state).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], Token::Word("echo".to_string()));

    // Should expand to /root/documents or ~root/documents
    if let Token::Word(path) = &result[1] {
        assert!(path == "/root/documents" || path == "~root/documents");
    } else {
        panic!("Expected Word token");
    }
}

#[test]
fn test_tilde_expansion_nonexistent_user() {
    let shell_state = ShellState::new();

    // Test with a username that definitely doesn't exist
    let result = lex("echo ~nonexistentuser12345", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("~nonexistentuser12345".to_string())
        ]
    );
}

#[test]
fn test_tilde_expansion_username_in_quotes() {
    let shell_state = ShellState::new();

    // Single quotes should prevent expansion
    let result = lex("echo '~root'", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("~root".to_string())
        ]
    );

    // Double quotes should also prevent expansion
    let result = lex("echo \"~root\"", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("~root".to_string())
        ]
    );
}

#[test]
fn test_tilde_expansion_mixed_with_username() {
    let _lock = ENV_LOCK.lock().unwrap();
    let mut shell_state = ShellState::new();
    let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    shell_state.set_var("PWD", "/current".to_string());

    // Test mixing different tilde expansions
    let result = lex("echo ~ ~+ ~root", &shell_state).unwrap();
    assert_eq!(result.len(), 4);
    assert_eq!(result[0], Token::Word("echo".to_string()));
    assert_eq!(result[1], Token::Word(home));
    assert_eq!(result[2], Token::Word("/current".to_string()));

    // The ~root expansion depends on whether /root exists
    if let Token::Word(path) = &result[3] {
        assert!(path == "/root" || path == "~root");
    } else {
        panic!("Expected Word token");
    }
}

#[test]
fn test_tilde_expansion_username_with_special_chars() {
    let shell_state = ShellState::new();

    // Test that special characters terminate username collection
    let result = lex("echo ~user@host", &shell_state).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], Token::Word("echo".to_string()));

    // Should try to expand ~user and then append @host
    if let Token::Word(path) = &result[1] {
        // The path should contain @host at the end
        assert!(path.contains("@host") || path == "~user@host");
    } else {
        panic!("Expected Word token");
    }
}