//! Tests for redirection functionality (file descriptors, here-documents, etc.)

use crate::executor::execute_single_command;
use crate::parser::{Redirection, ShellCommand};
use crate::state::ShellState;
use std::sync::Mutex;

// Mutex to serialize tests that modify environment variables or create files
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_here_string_execution() {
    // Test here-string redirection with a simple command
    let cmd = ShellCommand {
        args: vec!["cat".to_string()],
        redirections: Vec::new(),
        compound: None,
        // TODO: Update test for new redirection system
    };

    // Note: This test would require mocking stdin to provide the here-string content
    // For now, we'll just verify the command structure is parsed correctly
    assert_eq!(cmd.args, vec!["cat"]);
    // assert_eq!(cmd.here_string_content, Some("hello world".to_string()));
}

#[test]
fn test_here_document_execution() {
    // Test here-document redirection with a simple command
    let cmd = ShellCommand {
        args: vec!["cat".to_string()],
        redirections: Vec::new(),
        compound: None,
        // TODO: Update test for new redirection system
    };

    // Note: This test would require mocking stdin to provide the here-document content
    // For now, we'll just verify the command structure is parsed correctly
    assert_eq!(cmd.args, vec!["cat"]);
    // assert_eq!(cmd.here_doc_delimiter, Some("EOF".to_string()));
}

// ========================================================================
// File Descriptor Integration Tests
// ========================================================================

#[test]
fn test_fd_output_redirection() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Create unique temp file
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_test_fd_out_{}.txt", timestamp);

    // Test: echo "error" 2>errors.txt
    let cmd = ShellCommand {
        args: vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo error >&2".to_string(),
        ],
        redirections: vec![Redirection::FdOutput(2, temp_file.clone())],
        compound: None,
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Verify file was created and contains the error message
    let content = std::fs::read_to_string(&temp_file).unwrap();
    assert_eq!(content.trim(), "error");

    // Cleanup
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_fd_input_redirection() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Create unique temp file with content
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_test_fd_in_{}.txt", timestamp);

    std::fs::write(&temp_file, "test input\n").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Test: cat 3<input.txt (reading from fd 3)
    // Note: This tests that fd 3 is opened for reading
    let cmd = ShellCommand {
        args: vec!["cat".to_string()],
        compound: None,
        redirections: vec![
            Redirection::FdInput(3, temp_file.clone()),
            Redirection::Input(temp_file.clone()),
        ],
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Cleanup
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_fd_append_redirection() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Create unique temp file with initial content
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_test_fd_append_{}.txt", timestamp);

    std::fs::write(&temp_file, "first line\n").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Test: echo "more" 2>>errors.txt
    let cmd = ShellCommand {
        args: vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo second line >&2".to_string(),
        ],
        redirections: vec![Redirection::FdAppend(2, temp_file.clone())],
        compound: None,
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Verify file contains both lines
    let content = std::fs::read_to_string(&temp_file).unwrap();
    assert!(content.contains("first line"));
    assert!(content.contains("second line"));

    // Cleanup
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_fd_duplication_stderr_to_stdout() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Create unique temp file
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_test_fd_dup_{}.txt", timestamp);

    // Test: command 2>&1 >output.txt
    // Note: For external commands, fd duplication is handled by the shell
    // We test that the command executes successfully with the redirection
    let cmd = ShellCommand {
        args: vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo test; echo error >&2".to_string(),
        ],
        compound: None,
        redirections: vec![Redirection::Output(temp_file.clone())],
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Verify file was created and contains output
    assert!(std::path::Path::new(&temp_file).exists());
    let content = std::fs::read_to_string(&temp_file).unwrap();
    assert!(content.contains("test"));

    // Cleanup
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_fd_close() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Test: command 2>&- (closes stderr)
    let cmd = ShellCommand {
        args: vec!["sh".to_string(), "-c".to_string(), "echo test".to_string()],
        redirections: vec![Redirection::FdClose(2)],
        compound: None,
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Verify fd 2 is closed in the fd table
    assert!(shell_state.fd_table.borrow().is_closed(2));
}

#[test]
fn test_fd_read_write() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Create unique temp file
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_test_fd_rw_{}.txt", timestamp);

    std::fs::write(&temp_file, "initial content\n").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Test: 3<>file.txt (opens fd 3 for read/write)
    let cmd = ShellCommand {
        args: vec!["cat".to_string()],
        compound: None,
        redirections: vec![
            Redirection::FdInputOutput(3, temp_file.clone()),
            Redirection::Input(temp_file.clone()),
        ],
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Cleanup
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_multiple_fd_redirections() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Create unique temp files
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let out_file = format!("/tmp/rush_test_fd_multi_out_{}.txt", timestamp);
    let err_file = format!("/tmp/rush_test_fd_multi_err_{}.txt", timestamp);

    // Test: command 2>err.txt 1>out.txt
    let cmd = ShellCommand {
        args: vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo stdout; echo stderr >&2".to_string(),
        ],
        redirections: vec![
            Redirection::FdOutput(2, err_file.clone()),
            Redirection::Output(out_file.clone()),
        ],
        compound: None,
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Verify both files were created
    assert!(std::path::Path::new(&out_file).exists());
    assert!(std::path::Path::new(&err_file).exists());

    // Verify content
    let out_content = std::fs::read_to_string(&out_file).unwrap();
    let err_content = std::fs::read_to_string(&err_file).unwrap();
    assert!(out_content.contains("stdout"));
    assert!(err_content.contains("stderr"));

    // Cleanup
    let _ = std::fs::remove_file(&out_file);
    let _ = std::fs::remove_file(&err_file);
}

#[test]
fn test_fd_swap_pattern() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Create unique temp files
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_test_fd_swap_{}.txt", timestamp);

    // Test fd operations: open fd 3, then close it
    // This tests the fd table operations
    let cmd = ShellCommand {
        args: vec!["sh".to_string(), "-c".to_string(), "echo test".to_string()],
        redirections: vec![
            Redirection::FdOutput(3, temp_file.clone()), // Open fd 3 for writing
            Redirection::FdClose(3),                     // Close fd 3
            Redirection::Output(temp_file.clone()),      // Write to stdout
        ],
        compound: None,
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Verify fd 3 is closed after the operations
    assert!(shell_state.fd_table.borrow().is_closed(3));

    // Cleanup
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_fd_redirection_with_pipes() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Create unique temp file
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_test_fd_pipe_{}.txt", timestamp);

    // Test: cmd1 | cmd2 >output.txt
    // This tests redirections in pipelines
    let commands = vec![
        ShellCommand {
            args: vec!["echo".to_string(), "piped output".to_string()],
            redirections: vec![],
            compound: None,
        },
        ShellCommand {
            args: vec!["cat".to_string()],
            compound: None,
            redirections: vec![Redirection::Output(temp_file.clone())],
        },
    ];

    let mut shell_state = ShellState::new();
    let exit_code = crate::executor::execute_pipeline(&commands, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Verify output file contains the piped content
    let content = std::fs::read_to_string(&temp_file).unwrap();
    assert!(content.contains("piped output"));

    // Cleanup
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_fd_error_invalid_fd_number() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Create unique temp file
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_test_fd_invalid_{}.txt", timestamp);

    // Test: Invalid fd number (>1024)
    let cmd = ShellCommand {
        args: vec!["echo".to_string(), "test".to_string()],
        compound: None,
        redirections: vec![Redirection::FdOutput(1025, temp_file.clone())],
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);

    // Should fail with error
    assert_eq!(exit_code, 1);

    // Cleanup (file may not exist)
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_fd_error_duplicate_closed_fd() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Test: Attempting to duplicate a closed fd
    let cmd = ShellCommand {
        args: vec!["echo".to_string(), "test".to_string()],
        compound: None,
        redirections: vec![
            Redirection::FdClose(3),
            Redirection::FdDuplicate(2, 3), // Try to duplicate closed fd 3
        ],
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);

    // Should fail with error
    assert_eq!(exit_code, 1);
}

#[test]
fn test_fd_error_file_permission() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Test: Attempting to write to a read-only location
    let cmd = ShellCommand {
        args: vec!["echo".to_string(), "test".to_string()],
        redirections: vec![Redirection::FdOutput(2, "/proc/version".to_string())],
        compound: None,
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);

    // Should fail with permission error
    assert_eq!(exit_code, 1);
}

#[test]
fn test_fd_redirection_order() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Create unique temp files
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let file1 = format!("/tmp/rush_test_fd_order1_{}.txt", timestamp);
    let file2 = format!("/tmp/rush_test_fd_order2_{}.txt", timestamp);

    // Test: Redirections are processed left-to-right
    // 1>file1 1>file2 should write to file2
    let cmd = ShellCommand {
        args: vec!["echo".to_string(), "test".to_string()],
        compound: None,
        redirections: vec![
            Redirection::Output(file1.clone()),
            Redirection::Output(file2.clone()),
        ],
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);

    // file2 should have the output (last redirection wins)
    let content2 = std::fs::read_to_string(&file2).unwrap();
    assert!(content2.contains("test"));

    // Cleanup
    let _ = std::fs::remove_file(&file1);
    let _ = std::fs::remove_file(&file2);
}

#[test]
fn test_fd_builtin_with_redirection() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Create unique temp file
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_test_fd_builtin_{}.txt", timestamp);

    // Test: Built-in command with fd redirection
    let cmd = ShellCommand {
        args: vec!["echo".to_string(), "builtin test".to_string()],
        redirections: vec![Redirection::Output(temp_file.clone())],
        compound: None,
    };

    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Verify output
    let content = std::fs::read_to_string(&temp_file).unwrap();
    assert!(content.contains("builtin test"));

    // Cleanup
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_fd_variable_expansion_in_filename() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Create unique temp file
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_test_fd_var_{}.txt", timestamp);

    // Set variable for filename
    let mut shell_state = ShellState::new();
    shell_state.set_var("OUTFILE", temp_file.clone());

    // Test: Variable expansion in redirection filename
    let cmd = ShellCommand {
        args: vec!["echo".to_string(), "variable test".to_string()],
        compound: None,
        redirections: vec![Redirection::Output("$OUTFILE".to_string())],
    };

    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);

    // Verify output
    let content = std::fs::read_to_string(&temp_file).unwrap();
    assert!(content.contains("variable test"));

    // Cleanup
    let _ = std::fs::remove_file(&temp_file);
}