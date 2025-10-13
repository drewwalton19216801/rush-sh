//! Integration tests for subshell functionality
//! These tests verify POSIX-compliant subshell behavior

use rush_sh::{execute, Ast, ShellCommand, ShellState};
use rush_sh::lexer::lex;
use rush_sh::parser::parse;
use std::sync::Mutex;

// Mutex to serialize tests that fork processes
// This prevents race conditions when tests run in parallel
static FORK_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_execute_subshell_variable_isolation() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();
    shell_state.set_var("x", "1".to_string());

    // Execute: (x=2; echo $x)
    let ast = Ast::Subshell {
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "x".to_string(),
                value: "2".to_string(),
            },
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "$x".to_string()],
                input: None,
                output: None,
                append: None,
                here_doc_delimiter: None,
                here_doc_quoted: false,
                here_string_content: None,
                fd_redirections: vec![],
            }]),
        ])),
        redirections: vec![],
    };

    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Parent's x should still be 1 (subshell changes are isolated)
    assert_eq!(shell_state.get_var("x"), Some("1".to_string()));
}

#[test]
fn test_execute_subshell_exit_code() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();

    // Execute: (exit 42)
    let ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["exit".to_string(), "42".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
            fd_redirections: vec![],
        }])),
        redirections: vec![],
    };

    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 42);
}

#[test]
fn test_execute_nested_subshells() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();

    // Execute: ((echo nested))
    let ast = Ast::Subshell {
        body: Box::new(Ast::Subshell {
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "nested".to_string()],
                input: None,
                output: None,
                append: None,
                here_doc_delimiter: None,
                here_doc_quoted: false,
                here_string_content: None,
                fd_redirections: vec![],
            }])),
            redirections: vec![],
        }),
        redirections: vec![],
    };

    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_execute_subshell_inherits_functions() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();

    // Define a function in parent
    shell_state.define_function(
        "test_func".to_string(),
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["echo".to_string(), "from_function".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
            fd_redirections: vec![],
        }]),
    );

    // Execute: (test_func)
    let ast = Ast::Subshell {
        body: Box::new(Ast::FunctionCall {
            name: "test_func".to_string(),
            args: vec![],
        }),
        redirections: vec![],
    };

    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_execute_subshell_inherits_exported_vars() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();

    // Set and export a variable
    shell_state.set_exported_var("EXPORTED_VAR", "exported_value".to_string());

    // Execute: (echo $EXPORTED_VAR)
    let ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["echo".to_string(), "$EXPORTED_VAR".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
            fd_redirections: vec![],
        }])),
        redirections: vec![],
    };

    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_execute_subshell_multiple_commands() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();
    shell_state.set_var("x", "1".to_string());

    // Execute: (x=2; y=3; echo done)
    let ast = Ast::Subshell {
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "x".to_string(),
                value: "2".to_string(),
            },
            Ast::Assignment {
                var: "y".to_string(),
                value: "3".to_string(),
            },
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "done".to_string()],
                input: None,
                output: None,
                append: None,
                here_doc_delimiter: None,
                here_doc_quoted: false,
                here_string_content: None,
                fd_redirections: vec![],
            }]),
        ])),
        redirections: vec![],
    };

    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Parent's x should still be 1, y should not exist
    assert_eq!(shell_state.get_var("x"), Some("1".to_string()));
    assert_eq!(shell_state.get_var("y"), None);
}

#[test]
fn test_execute_subshell_positional_params() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();
    shell_state.set_positional_params(vec![
        "arg1".to_string(),
        "arg2".to_string(),
        "arg3".to_string(),
    ]);

    // Execute: (echo $1 $2 $3)
    let ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec![
                "echo".to_string(),
                "$1".to_string(),
                "$2".to_string(),
                "$3".to_string(),
            ],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
            fd_redirections: vec![],
        }])),
        redirections: vec![],
    };

    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Positional params should still be the same in parent
    assert_eq!(shell_state.get_var("1"), Some("arg1".to_string()));
    assert_eq!(shell_state.get_var("2"), Some("arg2".to_string()));
    assert_eq!(shell_state.get_var("3"), Some("arg3".to_string()));
}

#[test]
fn test_subshell_end_to_end_simple() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();
    shell_state.set_var("TEST", "parent_value".to_string());

    // Parse and execute: (TEST=child_value; echo $TEST)
    let tokens = lex("(TEST=child_value; echo $TEST)", &shell_state).unwrap();
    let ast = parse(tokens).unwrap();
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Parent variable should be unchanged
    assert_eq!(shell_state.get_var("TEST"), Some("parent_value".to_string()));
}

#[test]
fn test_subshell_end_to_end_nested() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();

    // Parse and execute: ((echo deeply nested))
    let tokens = lex("((echo deeply nested))", &shell_state).unwrap();
    let ast = parse(tokens).unwrap();
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}