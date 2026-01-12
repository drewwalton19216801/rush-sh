//! Tests for file descriptor table operations

use crate::state::FileDescriptorTable;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::sync::Mutex;

// Mutex to serialize tests that create temporary files
static FILE_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_fd_table_creation() {
    let fd_table = FileDescriptorTable::new();
    assert!(!fd_table.is_open(0));
    assert!(!fd_table.is_open(1));
    assert!(!fd_table.is_open(2));
}

#[test]
fn test_fd_table_open_file() {
    let mut fd_table = FileDescriptorTable::new();

    // Create a temporary file
    let temp_file = "/tmp/rush_test_fd_open.txt";
    std::fs::write(temp_file, "test content").unwrap();

    // Open file for reading
    let result = fd_table.open_fd(3, temp_file, true, false, false, false, false);
    assert!(result.is_ok());
    assert!(fd_table.is_open(3));

    // Clean up
    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_fd_table_open_file_for_writing() {
    let mut fd_table = FileDescriptorTable::new();

    // Create a temporary file path
    let temp_file = "/tmp/rush_test_fd_write.txt";

    // Open file for writing
    let result = fd_table.open_fd(4, temp_file, false, true, false, true, false);
    assert!(result.is_ok());
    assert!(fd_table.is_open(4));

    // Clean up
    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_fd_table_invalid_fd_number() {
    let mut fd_table = FileDescriptorTable::new();

    // Test invalid fd numbers
    let result = fd_table.open_fd(-1, "/tmp/test.txt", true, false, false, false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid file descriptor"));

    let result = fd_table.open_fd(1025, "/tmp/test.txt", true, false, false, false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid file descriptor"));
}

#[test]
fn test_fd_table_duplicate_fd() {
    let mut fd_table = FileDescriptorTable::new();

    // Create a temporary file
    let temp_file = "/tmp/rush_test_fd_dup.txt";
    std::fs::write(temp_file, "test content").unwrap();

    // Open file on fd 3
    fd_table
        .open_fd(3, temp_file, true, false, false, false, false)
        .unwrap();
    assert!(fd_table.is_open(3));

    // Duplicate fd 3 to fd 4
    let result = fd_table.duplicate_fd(3, 4);
    assert!(result.is_ok());
    assert!(fd_table.is_open(4));

    // Clean up
    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_fd_table_duplicate_to_self() {
    let mut fd_table = FileDescriptorTable::new();

    // Create a temporary file
    let temp_file = "/tmp/rush_test_fd_dup_self.txt";
    std::fs::write(temp_file, "test content").unwrap();

    // Open file on fd 3
    fd_table
        .open_fd(3, temp_file, true, false, false, false, false)
        .unwrap();

    // Duplicate fd 3 to itself (should be no-op)
    let result = fd_table.duplicate_fd(3, 3);
    assert!(result.is_ok());
    assert!(fd_table.is_open(3));

    // Clean up
    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_fd_table_duplicate_closed_fd() {
    let mut fd_table = FileDescriptorTable::new();

    // Try to duplicate a closed fd
    let result = fd_table.duplicate_fd(3, 4);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not open"));
}

#[test]
fn test_fd_table_close_fd() {
    let mut fd_table = FileDescriptorTable::new();

    // Create a temporary file
    let temp_file = "/tmp/rush_test_fd_close.txt";
    std::fs::write(temp_file, "test content").unwrap();

    // Open file on fd 3
    fd_table
        .open_fd(3, temp_file, true, false, false, false, false)
        .unwrap();
    assert!(fd_table.is_open(3));

    // Close fd 3
    let result = fd_table.close_fd(3);
    assert!(result.is_ok());
    assert!(fd_table.is_closed(3));
    assert!(!fd_table.is_open(3));

    // Clean up
    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_fd_table_save_and_restore() {
    let mut fd_table = FileDescriptorTable::new();

    // Save stdin (fd 0)
    let result = fd_table.save_fd(0);
    assert!(result.is_ok());

    // Restore stdin
    let result = fd_table.restore_fd(0);
    assert!(result.is_ok());
}

#[test]
fn test_fd_table_save_all_and_restore_all() {
    let _lock = FILE_LOCK.lock().unwrap();
    let mut fd_table = FileDescriptorTable::new();

    // Create unique temporary files
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file1 = format!("/tmp/rush_test_fd_save1_{}.txt", timestamp);
    let temp_file2 = format!("/tmp/rush_test_fd_save2_{}.txt", timestamp);

    std::fs::write(&temp_file1, "test content 1").unwrap();
    std::fs::write(&temp_file2, "test content 2").unwrap();

    // Open files on fd 50 and 51
    // Manually dup2 to ensure these FDs are valid for save_fd()
    // Using higher FDs to avoid conflict with parallel tests using 0-9
    let f1 = File::open(&temp_file1).unwrap();
    let f2 = File::open(&temp_file2).unwrap();
    unsafe {
        libc::dup2(f1.as_raw_fd(), 50);
        libc::dup2(f2.as_raw_fd(), 51);
    }

    fd_table
        .open_fd(50, &temp_file1, true, false, false, false, false)
        .unwrap();
    fd_table
        .open_fd(51, &temp_file2, true, false, false, false, false)
        .unwrap();

    // Save all fds
    let result = fd_table.save_all_fds();
    assert!(result.is_ok());

    // Restore all fds
    let result = fd_table.restore_all_fds();
    assert!(result.is_ok());

    // Clean up
    unsafe {
        libc::close(50);
        libc::close(51);
    }
    let _ = std::fs::remove_file(&temp_file1);
    let _ = std::fs::remove_file(&temp_file2);
}

#[test]
fn test_fd_table_clear() {
    let mut fd_table = FileDescriptorTable::new();

    // Create a temporary file
    let temp_file = "/tmp/rush_test_fd_clear.txt";
    std::fs::write(temp_file, "test content").unwrap();

    // Open file on fd 50 (was 3)
    // Manual setup not strictly needed for clear() test as it checks map?
    // But clear() might close FDs?
    // FileDescriptorTable::clear() just clears map. File drops.

    fd_table
        .open_fd(50, temp_file, true, false, false, false, false)
        .unwrap();
    assert!(fd_table.is_open(50));

    // Clear all fds
    fd_table.clear();
    assert!(!fd_table.is_open(3));

    // Clean up
    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_fd_table_get_stdio() {
    let mut fd_table = FileDescriptorTable::new();

    // Create a temporary file
    let temp_file = "/tmp/rush_test_fd_stdio.txt";
    std::fs::write(temp_file, "test content").unwrap();

    // Open file on fd 3
    fd_table
        .open_fd(3, temp_file, true, false, false, false, false)
        .unwrap();

    // Get Stdio for fd 3
    let stdio = fd_table.get_stdio(3);
    assert!(stdio.is_some());

    // Get Stdio for non-existent fd
    let stdio = fd_table.get_stdio(5);
    assert!(stdio.is_none());

    // Clean up
    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_fd_table_multiple_operations() {
    let mut fd_table = FileDescriptorTable::new();

    // Create temporary files
    let temp_file1 = "/tmp/rush_test_fd_multi1.txt";
    let temp_file2 = "/tmp/rush_test_fd_multi2.txt";
    std::fs::write(temp_file1, "test content 1").unwrap();
    std::fs::write(temp_file2, "test content 2").unwrap();

    // Open file on fd 3
    fd_table
        .open_fd(3, temp_file1, true, false, false, false, false)
        .unwrap();
    assert!(fd_table.is_open(3));

    // Duplicate fd 3 to fd 4
    fd_table.duplicate_fd(3, 4).unwrap();
    assert!(fd_table.is_open(4));

    // Open another file on fd 5
    fd_table
        .open_fd(5, temp_file2, true, false, false, false, false)
        .unwrap();
    assert!(fd_table.is_open(5));

    // Close fd 4
    fd_table.close_fd(4).unwrap();
    assert!(fd_table.is_closed(4));
    assert!(!fd_table.is_open(4));

    // fd 3 and 5 should still be open
    assert!(fd_table.is_open(3));
    assert!(fd_table.is_open(5));

    // Clean up
    let _ = std::fs::remove_file(temp_file1);
    let _ = std::fs::remove_file(temp_file2);
}

#[test]
fn test_shell_state_has_fd_table() {
    use crate::state::ShellState;
    let state = ShellState::new();
    let fd_table = state.fd_table.borrow();
    assert!(!fd_table.is_open(3));
}

#[test]
fn test_shell_state_fd_table_operations() {
    use crate::state::ShellState;
    let state = ShellState::new();

    // Create a temporary file
    let temp_file = "/tmp/rush_test_state_fd.txt";
    std::fs::write(temp_file, "test content").unwrap();

    // Open file through shell state's fd table
    {
        let mut fd_table = state.fd_table.borrow_mut();
        fd_table
            .open_fd(3, temp_file, true, false, false, false, false)
            .unwrap();
    }

    // Verify it's open
    {
        let fd_table = state.fd_table.borrow();
        assert!(fd_table.is_open(3));
    }

    // Clean up
    let _ = std::fs::remove_file(temp_file);
}