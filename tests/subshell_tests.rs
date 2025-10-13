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

// ============================================================================
// Phase 2: Subshell Redirection Tests
// ============================================================================

#[test]
fn test_subshell_output_redirection() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_subshell_out_{}.txt", timestamp);
    
    let mut shell_state = ShellState::new();
    
    // Parse and execute: (echo line1; echo line2) >file
    let input = format!("(echo line1; echo line2) >{}", temp_file);
    let tokens = lex(&input, &shell_state).unwrap();
    let ast = parse(tokens).unwrap();
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Verify both lines are in the file
    let contents = fs::read_to_string(&temp_file).unwrap();
    assert!(contents.contains("line1"), "file should contain line1");
    assert!(contents.contains("line2"), "file should contain line2");
    
    // Cleanup
    let _ = fs::remove_file(&temp_file);
}

#[test]
fn test_subshell_input_redirection() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let input_file = format!("/tmp/rush_subshell_in_{}.txt", timestamp);
    
    // Create input file
    fs::write(&input_file, "test input\n").unwrap();
    
    let mut shell_state = ShellState::new();
    
    // Parse and execute: (cat) <file
    let input = format!("(cat) <{}", input_file);
    let tokens = lex(&input, &shell_state).unwrap();
    let ast = parse(tokens).unwrap();
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Cleanup
    let _ = fs::remove_file(&input_file);
}

#[test]
fn test_subshell_fd_redirection_2_to_1() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use rush_sh::parser::FdRedirection;
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_subshell_fd_{}.txt", timestamp);
    
    let mut shell_state = ShellState::new();
    
    // Execute: (echo error >&2) >file 2>&1
    let ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["sh".to_string(), "-c".to_string(), "echo error >&2".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
            fd_redirections: vec![],
        }])),
        redirections: vec![
            FdRedirection::ToFile {
                fd: 1,
                filename: temp_file.clone(),
            },
            FdRedirection::DuplicateOutput {
                source_fd: 2,
                target_fd: 1,
            },
        ],
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Verify stderr was captured in the file
    let contents = fs::read_to_string(&temp_file).unwrap();
    assert!(contents.contains("error"), "file should contain stderr output");
    
    // Cleanup
    let _ = fs::remove_file(&temp_file);
}

#[test]
fn test_subshell_append_redirection() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_subshell_append_{}.txt", timestamp);
    
    // Create initial file
    fs::write(&temp_file, "initial\n").unwrap();
    
    let mut shell_state = ShellState::new();
    
    // Parse and execute: (echo appended) >>file
    let input = format!("(echo appended) >>{}", temp_file);
    let tokens = lex(&input, &shell_state).unwrap();
    let ast = parse(tokens).unwrap();
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Verify both initial and appended content
    let contents = fs::read_to_string(&temp_file).unwrap();
    assert!(contents.contains("initial"), "file should contain initial content");
    assert!(contents.contains("appended"), "file should contain appended content");
    
    // Cleanup
    let _ = fs::remove_file(&temp_file);
}

#[test]
fn test_subshell_multiple_redirections() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use rush_sh::parser::FdRedirection;
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let out_file = format!("/tmp/rush_subshell_multi_out_{}.txt", timestamp);
    let err_file = format!("/tmp/rush_subshell_multi_err_{}.txt", timestamp);
    
    let mut shell_state = ShellState::new();
    
    // Execute: (echo stdout; echo stderr >&2) >out 2>err
    let ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo stdout; echo stderr >&2".to_string(),
            ],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
            fd_redirections: vec![],
        }])),
        redirections: vec![
            FdRedirection::ToFile {
                fd: 1,
                filename: out_file.clone(),
            },
            FdRedirection::ToFile {
                fd: 2,
                filename: err_file.clone(),
            },
        ],
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Verify stdout went to out_file
    let out_contents = fs::read_to_string(&out_file).unwrap();
    assert!(out_contents.contains("stdout"), "out file should contain stdout");
    
    // Verify stderr went to err_file
    let err_contents = fs::read_to_string(&err_file).unwrap();
    assert!(err_contents.contains("stderr"), "err file should contain stderr");
    
    // Cleanup
    let _ = fs::remove_file(&out_file);
    let _ = fs::remove_file(&err_file);
}

#[test]
fn test_subshell_redirection_with_variable_expansion() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_subshell_var_{}.txt", timestamp);
    
    let mut shell_state = ShellState::new();
    shell_state.set_var("OUTFILE", temp_file.clone());
    
    // Parse and execute: (echo test) >$OUTFILE
    let input = "(echo test) >$OUTFILE";
    let tokens = lex(input, &shell_state).unwrap();
    let ast = parse(tokens).unwrap();
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Verify output was written
    let contents = fs::read_to_string(&temp_file).unwrap();
    assert!(contents.contains("test"), "file should contain output");
    
    // Cleanup
    let _ = fs::remove_file(&temp_file);
}

#[test]
fn test_subshell_redirection_order() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use rush_sh::parser::FdRedirection;
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_subshell_order_{}.txt", timestamp);
    
    let mut shell_state = ShellState::new();
    
    // Execute: (echo stdout; echo stderr >&2) 2>&1 >file
    // Expected: stderr goes to old stdout (terminal), stdout goes to file
    let ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo stdout_msg; echo stderr_msg >&2".to_string(),
            ],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
            fd_redirections: vec![],
        }])),
        redirections: vec![
            FdRedirection::DuplicateOutput {
                source_fd: 2,
                target_fd: 1,
            },
            FdRedirection::ToFile {
                fd: 1,
                filename: temp_file.clone(),
            },
        ],
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Verify file contains only stdout (stderr went to old stdout location)
    let contents = fs::read_to_string(&temp_file).unwrap();
    assert!(contents.contains("stdout_msg"), "file should contain stdout");
    assert!(!contents.contains("stderr_msg"), "file should not contain stderr");
    
    // Cleanup
    let _ = fs::remove_file(&temp_file);
}

#[test]
fn test_nested_subshell_with_redirections() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use rush_sh::parser::FdRedirection;
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_subshell_nested_{}.txt", timestamp);
    
    let mut shell_state = ShellState::new();
    
    // Execute: ((echo nested)) >file
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
        redirections: vec![FdRedirection::ToFile {
            fd: 1,
            filename: temp_file.clone(),
        }],
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Verify output was written
    let contents = fs::read_to_string(&temp_file).unwrap();
    assert!(contents.contains("nested"), "file should contain nested output");
    
    // Cleanup
    let _ = fs::remove_file(&temp_file);
}

#[test]
fn test_subshell_fd_close() {
    let _lock = FORK_LOCK.lock().unwrap();
    
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use rush_sh::parser::FdRedirection;
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_subshell_close_{}.txt", timestamp);
    
    let mut shell_state = ShellState::new();
    
    // Execute: (echo test) >file 2>&-
    // This closes stderr in the subshell
    let ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["echo".to_string(), "test".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
            fd_redirections: vec![],
        }])),
        redirections: vec![
            FdRedirection::ToFile {
                fd: 1,
                filename: temp_file.clone(),
            },
            FdRedirection::Close { fd: 2 },
        ],
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Verify stdout was written correctly
    let contents = fs::read_to_string(&temp_file).unwrap();
    assert!(contents.contains("test"), "file should contain output");
    
    // Cleanup
    let _ = fs::remove_file(&temp_file);
}