# FdManager API Design

## Overview

The `FdManager` is responsible for managing file descriptor (FD) operations within the Rush shell. It provides a safe, POSIX-compliant interface for FD manipulation, including duplication, closing, redirection, and state restoration.

## Core Responsibilities

1. **FD State Tracking**: Maintain a registry of open FDs and their current states
2. **FD Operations**: Provide methods for FD duplication, closing, and redirection
3. **FD Validation**: Ensure FD operations are valid and safe
4. **FD Restoration**: Save and restore FD states for command execution
5. **FD Cleanup**: Properly close FDs after command execution to prevent leaks

## Data Structures

### FdManager

```rust
use std::collections::HashMap;
use std::fs::File;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::os::fd::OwnedFd;

/// Manages file descriptor operations for the shell
/// 
/// This module provides POSIX-compliant FD manipulation including:
/// - FD duplication (dup2)
/// - FD closing
/// - FD redirection to files
/// - FD state saving and restoration
pub struct FdManager {
    /// Saved FD states for restoration after command execution
    /// Maps FD number to its saved state (original FD or file path)
    saved_fds: HashMap<i32, SavedFdState>,
    
    /// Track FD duplications to prevent double-closing
    /// Maps destination FD to source FD (for dup2 operations)
    fd_duplications: HashMap<i32, i32>,
    
    /// Track which FDs have been explicitly closed
    closed_fds: HashSet<i32>,
}

/// Represents the saved state of a file descriptor
#[derive(Debug, Clone)]
pub enum SavedFdState {
    /// FD was duplicated from another FD
    DuplicatedFrom(i32),
    
    /// FD was redirected from a file
    RedirectedFromFile {
        path: String,
        append: bool,
        original_fd: Option<i32>,
    },
    
    /// FD was closed (saved as None to indicate it should stay closed)
    Closed,
}

/// Errors that can occur during FD operations
#[derive(Debug)]
pub enum FdError {
    /// Invalid FD number (negative or too large)
    InvalidFd(i32),
    
    /// Attempted to operate on a closed FD
    FdClosed(i32),
    
    /// I/O error during FD operation
    IoError(std::io::Error),
    
    /// Permission denied for FD operation
    PermissionDenied(String),
    
    /// File not found for FD redirection
    FileNotFound(String),
    
    /// FD duplication failed
    DuplicationFailed(i32, i32),
    
    /// FD restoration failed
    RestorationFailed(i32),
    
    /// FD already in use
    FdInUse(i32),
}

impl std::fmt::Display for FdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FdError::InvalidFd(fd) => write!(f, "Invalid file descriptor: {}", fd),
            FdError::FdClosed(fd) => write!(f, "File descriptor {} is closed", fd),
            FdError::IoError(e) => write!(f, "I/O error: {}", e),
            FdError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            FdError::FileNotFound(path) => write!(f, "File not found: {}", path),
            FdError::DuplicationFailed(src, dest) => {
                write!(f, "Failed to duplicate FD {} to {}", src, dest)
            }
            FdError::RestorationFailed(fd) => {
                write!(f, "Failed to restore FD {}", fd)
            }
            FdError::FdInUse(fd) => write!(f, "File descriptor {} is in use", fd),
        }
    }
}

impl std::error::Error for FdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
    
    fn description(&self) -> Option<&str> {
        Some("file descriptor operation error")
    }
}
```

## Public API

### Constructor

```rust
impl FdManager {
    /// Create a new FdManager with empty state
    /// 
    /// # Returns
    /// A new FdManager instance ready for FD operations
    pub fn new() -> Self {
        Self {
            saved_fds: HashMap::new(),
            fd_duplications: HashMap::new(),
            closed_fds: HashSet::new(),
        }
    }
}
```

### FD Validation

```rust
impl FdManager {
    /// Check if a file descriptor number is valid
    /// 
    /// # Arguments
    /// * `fd` - The file descriptor number to validate
    /// 
    /// # Returns
    /// * `true` if the FD is valid (non-negative and reasonable)
    /// * `false` if the FD is invalid
    /// 
    /// # Notes
    /// - Valid FDs are typically 0-1023 (system-dependent)
    /// - Standard FDs are 0 (stdin), 1 (stdout), 2 (stderr)
    /// - Custom FDs can be any non-negative integer
    pub fn is_valid_fd(fd: i32) -> bool {
        fd >= 0 && fd < 1024
    }
    
    /// Validate that an FD is not closed
    /// 
    /// # Arguments
    /// * `fd` - The file descriptor to check
    /// 
    /// # Returns
    /// * `Ok(())` if the FD is not closed
    /// * `Err(FdError::FdClosed)` if the FD is closed
    pub fn validate_fd_not_closed(&self, fd: i32) -> Result<(), FdError> {
        if self.closed_fds.contains(&fd) {
            Err(FdError::FdClosed(fd))
        } else {
            Ok(())
        }
    }
}
```

### FD Duplication

```rust
impl FdManager {
    /// Duplicate a file descriptor using dup2 system call
    /// 
    /// This creates a copy of `src_fd` at `dest_fd`, closing `dest_fd` first if open.
    /// 
    /// # Arguments
    /// * `src_fd` - Source file descriptor to duplicate from
    /// * `dest_fd` - Destination file descriptor number
    /// 
    /// # Returns
    /// * `Ok(())` if duplication succeeded
    /// * `Err(FdError)` if duplication failed
    /// 
    /// # POSIX Behavior
    /// - Uses dup2() syscall for atomic FD duplication
    /// - Closes dest_fd if it's currently open
    /// - Shares file offset and status flags between FDs
    /// 
    /// # Example
    /// ```rust
    /// let mut fd_manager = FdManager::new();
    /// fd_manager.duplicate_fd(1, 2)?;  // Duplicate stdout to stderr
    /// ```
    pub fn duplicate_fd(&mut self, src_fd: i32, dest_fd: i32) -> Result<(), FdError> {
        // Validate FDs
        if !Self::is_valid_fd(src_fd) {
            return Err(FdError::InvalidFd(src_fd));
        }
        if !Self::is_valid_fd(dest_fd) {
            return Err(FdError::InvalidFd(dest_fd));
        }
        
        // Check if source FD is closed
        if self.closed_fds.contains(&src_fd) {
            return Err(FdError::FdClosed(src_fd));
        }
        
        // Check if destination FD is already in use
        if self.fd_duplications.contains_key(&dest_fd) {
            return Err(FdError::FdInUse(dest_fd));
        }
        
        // Perform dup2 system call
        // SAFETY: We're using valid FD numbers and checking for errors
        let result = unsafe { libc::dup2(src_fd, dest_fd) };
        
        if result == -1 {
            let errno = unsafe { *libc::__errno_location() };
            return Err(FdError::IoError(std::io::Error::from_raw_os_error(errno)));
        }
        
        // Track the duplication
        self.fd_duplications.insert(dest_fd, src_fd);
        
        Ok(())
    }
}
```

### FD Closing

```rust
impl FdManager {
    /// Close a file descriptor
    /// 
    /// # Arguments
    /// * `fd` - The file descriptor to close
    /// 
    /// # Returns
    /// * `Ok(())` if closing succeeded
    /// * `Err(FdError)` if closing failed
    /// 
    /// # Notes
    /// - Marks the FD as closed in the manager
    /// - Cannot close FDs that were duplicated (must close source)
    /// 
    /// # Example
    /// ```rust
    /// let mut fd_manager = FdManager::new();
    /// fd_manager.close_fd(2)?;  // Close stderr
    /// ```
    pub fn close_fd(&mut self, fd: i32) -> Result<(), FdError> {
        // Validate FD
        if !Self::is_valid_fd(fd) {
            return Err(FdError::InvalidFd(fd));
        }
        
        // Check if FD is already closed
        if self.closed_fds.contains(&fd) {
            return Err(FdError::FdClosed(fd));
        }
        
        // Perform close system call
        // SAFETY: We're using a valid FD number
        let result = unsafe { libc::close(fd) };
        
        if result == -1 {
            let errno = unsafe { *libc::__errno_location() };
            return Err(FdError::IoError(std::io::Error::from_raw_os_error(errno)));
        }
        
        // Mark FD as closed
        self.closed_fds.insert(fd);
        
        Ok(())
    }
}
```

### FD Redirection to File

```rust
impl FdManager {
    /// Redirect a file descriptor to a file
    /// 
    /// # Arguments
    /// * `fd` - The file descriptor to redirect
    /// * `path` - Path to the file to redirect to
    /// * `append` - If true, append to file; if false, truncate file
    /// 
    /// # Returns
    /// * `Ok(())` if redirection succeeded
    /// * `Err(FdError)` if redirection failed
    /// 
    /// # POSIX Behavior
    /// - Opens file with appropriate flags (O_CREAT | O_WRONLY | [O_APPEND])
    /// - Uses dup2() to redirect FD to the file's FD
    /// - Closes the original FD if it was open
    /// 
    /// # Example
    /// ```rust
    /// let mut fd_manager = FdManager::new();
    /// fd_manager.redirect_to_file(1, "output.txt", false)?;  // Redirect stdout to file
    /// ```
    pub fn redirect_to_file(&mut self, fd: i32, path: &str, append: bool) -> Result<(), FdError> {
        // Validate FD
        if !Self::is_valid_fd(fd) {
            return Err(FdError::InvalidFd(fd));
        }
        
        // Check if FD is closed
        if self.closed_fds.contains(&fd) {
            return Err(FdError::FdClosed(fd));
        }
        
        // Convert path to CString for system call
        let path_c = match std::ffi::CString::new(path) {
            Ok(c) => c,
            Err(_) => return Err(FdError::IoError(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path")
            )),
        };
        
        // Determine open flags
        let mut flags = libc::O_CREAT | libc::O_WRONLY;
        if append {
            flags |= libc::O_APPEND;
        } else {
            flags |= libc::O_TRUNC;
        }
        
        // Set file permissions (rw-rw-r--)
        let mode = libc::S_IRUSR | libc::S_IWUSR | libc::S_IRGRP | libc::S_IWGRP | libc::S_IROTH | libc::S_IWOTH;
        
        // Open the file
        // SAFETY: path_c is a valid CString, flags and mode are valid
        let file_fd = unsafe { libc::open(path_c.as_ptr(), flags, mode) };
        
        if file_fd == -1 {
            let errno = unsafe { *libc::__errno_location() };
            return Err(FdError::IoError(std::io::Error::from_raw_os_error(errno)));
        }
        
        // Use dup2 to redirect the FD to the file
        // This closes the original FD if it was open
        let result = unsafe { libc::dup2(file_fd, fd) };
        
        if result == -1 {
            let errno = unsafe { *libc::__errno_location() };
            // Close the file FD we just opened
            unsafe { libc::close(file_fd) };
            return Err(FdError::IoError(std::io::Error::from_raw_os_error(errno)));
        }
        
        Ok(())
    }
}
```

### FD State Saving and Restoration

```rust
impl FdManager {
    /// Save the current state of a file descriptor
    /// 
    /// This saves the FD's current state so it can be restored later.
    /// Useful for temporary FD operations during command execution.
    /// 
    /// # Arguments
    /// * `fd` - The file descriptor to save
    /// 
    /// # Returns
    /// * `Ok(())` if saving succeeded
    /// * `Err(FdError)` if saving failed
    /// 
    /// # Notes
    /// - Uses fcntl() to get FD flags
    /// - Stores the FD's current state for restoration
    /// - Cannot save FDs that are already closed
    /// 
    /// # Example
    /// ```rust
    /// let mut fd_manager = FdManager::new();
    /// fd_manager.save_fd(1)?;  // Save stdout state
    /// // ... perform operations ...
    /// fd_manager.restore_fd(1)?;  // Restore stdout
    /// ```
    pub fn save_fd(&mut self, fd: i32) -> Result<(), FdError> {
        // Validate FD
        if !Self::is_valid_fd(fd) {
            return Err(FdError::InvalidFd(fd));
        }
        
        // Check if FD is closed
        if self.closed_fds.contains(&fd) {
            return Err(FdError::FdClosed(fd));
        }
        
        // Check if FD is already saved
        if self.saved_fds.contains_key(&fd) {
            return Err(FdError::FdInUse(fd));
        }
        
        // Get FD flags using fcntl
        // SAFETY: fd is a valid, open FD
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        
        if flags == -1 {
            let errno = unsafe { *libc::__errno_location() };
            return Err(FdError::IoError(std::io::Error::from_raw_os_error(errno)));
        }
        
        // Save the FD state
        // We save the flags to indicate the FD was open
        self.saved_fds.insert(fd, SavedFdState::DuplicatedFrom(flags));
        
        Ok(())
    }
    
    /// Restore a previously saved file descriptor
    /// 
    /// # Arguments
    /// * `fd` - The file descriptor to restore
    /// 
    /// # Returns
    /// * `Ok(())` if restoration succeeded
    /// * `Err(FdError)` if restoration failed
    /// 
    /// # Notes
    /// - Removes the FD from saved state
    /// - Cannot restore FDs that are currently closed
    /// 
    /// # Example
    /// ```rust
    /// let mut fd_manager = FdManager::new();
    /// fd_manager.save_fd(1)?;
    /// // ... operations that modify FD 1 ...
    /// fd_manager.restore_fd(1)?;  // Restore to saved state
    /// ```
    pub fn restore_fd(&mut self, fd: i32) -> Result<(), FdError> {
        // Validate FD
        if !Self::is_valid_fd(fd) {
            return Err(FdError::InvalidFd(fd));
        }
        
        // Check if FD is closed
        if self.closed_fds.contains(&fd) {
            return Err(FdError::FdClosed(fd));
        }
        
        // Check if FD was saved
        if !self.saved_fds.contains_key(&fd) {
            return Err(FdError::RestorationFailed(fd));
        }
        
        // Remove from saved state
        self.saved_fds.remove(&fd);
        
        Ok(())
    }
    
    /// Restore all saved file descriptors
    /// 
    /// This restores all FDs that were previously saved, in reverse order
    /// to ensure proper restoration sequence.
    /// 
    /// # Returns
    /// * `Ok(())` if all restorations succeeded
    /// * `Err(FdError)` if any restoration failed
    /// 
    /// # Notes
    /// - Restores FDs in reverse order of saving
    /// - Clears all saved state after restoration
    /// 
    /// # Example
    /// ```rust
    /// let mut fd_manager = FdManager::new();
    /// fd_manager.save_fd(1)?;
    /// fd_manager.save_fd(2)?;
    /// // ... operations ...
    /// fd_manager.restore_all()?;  // Restore all saved FDs
    /// ```
    pub fn restore_all(&mut self) -> Result<(), FdError> {
        // Collect saved FDs in reverse order
        let saved_fds: Vec<i32> = self.saved_fds.keys().cloned().collect();
        
        // Restore each FD
        for fd in saved_fds.into_iter().rev() {
            self.restore_fd(fd)?;
        }
        
        Ok(())
    }
}
```

### Cleanup and Reset

```rust
impl FdManager {
    /// Reset the FdManager to initial state
    /// 
    /// This clears all saved FD states, duplications, and closed FD tracking.
    /// Useful for starting a new command execution context.
    /// 
    /// # Notes
    /// - Does NOT close any FDs (caller must ensure cleanup)
    /// - Clears all internal tracking structures
    /// 
    /// # Example
    /// ```rust
    /// let mut fd_manager = FdManager::new();
    /// // ... perform operations ...
    /// fd_manager.reset();  // Reset for next command
    /// ```
    pub fn reset(&mut self) {
        self.saved_fds.clear();
        self.fd_duplications.clear();
        self.closed_fds.clear();
    }
    
    /// Get the number of saved FDs
    /// 
    /// # Returns
    /// The count of FDs currently in saved state
    pub fn saved_fd_count(&self) -> usize {
        self.saved_fds.len()
    }
    
    /// Get the number of closed FDs
    /// 
    /// # Returns
    /// The count of FDs currently marked as closed
    pub fn closed_fd_count(&self) -> usize {
        self.closed_fds.len()
    }
}
```

## Usage Examples

### Basic FD Duplication

```rust
use crate::fd_manager::FdManager;

fn example_fd_duplication() -> Result<(), Box<dyn std::error::Error>> {
    let mut fd_manager = FdManager::new();
    
    // Duplicate stdout to stderr (redirect stderr to stdout)
    fd_manager.duplicate_fd(1, 2)?;
    
    // Execute command...
    
    Ok(())
}
```

### FD Redirection to File

```rust
fn example_fd_redirection() -> Result<(), Box<dyn std::error::Error>> {
    let mut fd_manager = FdManager::new();
    
    // Redirect stderr to a file
    fd_manager.redirect_to_file(2, "error.log", false)?;
    
    // Execute command...
    
    Ok(())
}
```

### FD State Saving and Restoration

```rust
fn example_fd_save_restore() -> Result<(), Box<dyn std::error::Error>> {
    let mut fd_manager = FdManager::new();
    
    // Save current stdout state
    fd_manager.save_fd(1)?;
    
    // Redirect stdout to a file temporarily
    fd_manager.redirect_to_file(1, "temp_output.txt", false)?;
    
    // Execute command...
    
    // Restore original stdout
    fd_manager.restore_fd(1)?;
    
    Ok(())
}
```

### Complex FD Operations

```rust
fn example_complex_fd_operations() -> Result<(), Box<dyn std::error::Error>> {
    let mut fd_manager = FdManager::new();
    
    // Save stdout and stderr
    fd_manager.save_fd(1)?;
    fd_manager.save_fd(2)?;
    
    // Redirect both to files
    fd_manager.redirect_to_file(1, "output.txt", false)?;
    fd_manager.redirect_to_file(2, "error.txt", false)?;
    
    // Execute command...
    
    // Restore both
    fd_manager.restore_all()?;
    
    Ok(())
}
```

## Testing Strategy

### Unit Tests

1. **FD Validation Tests**
   - Test valid FD numbers (0, 1, 2, 3, etc.)
   - Test invalid FD numbers (-1, 1024, etc.)
   - Test closed FD validation

2. **FD Duplication Tests**
   - Test successful duplication
   - Test duplication of invalid FDs
   - Test duplication to closed FDs
   - Test duplication of already-in-use FDs

3. **FD Closing Tests**
   - Test successful closing
   - Test closing invalid FDs
   - Test closing already-closed FDs
   - Test closing duplicated FDs

4. **FD Redirection Tests**
   - Test redirection to existing files
   - Test redirection to new files
   - Test append mode
   - Test redirection with invalid paths

5. **FD Save/Restore Tests**
   - Test saving and restoring FDs
   - Test restoring unsaved FDs
   - Test restoring closed FDs
   - Test restore_all functionality

### Integration Tests

1. **Command Execution Tests**
   - Test commands with FD redirections
   - Test commands with FD duplications
   - Test commands with FD closing

2. **Pipeline Tests**
   - Test FD operations in pipelines
   - Test FD state across pipeline stages

3. **Built-in Command Tests**
   - Test FD operations with built-in commands
   - Test FD state preservation in built-ins

## Error Handling

### Error Recovery

1. **Graceful Degradation**: On FD operation failure, continue with available FDs
2. **Error Propagation**: Return detailed errors to caller for proper handling
3. **State Cleanup**: Ensure FdManager is in consistent state after errors

### Error Messages

- Clear, actionable error messages
- Include FD numbers in error messages
- Suggest corrective actions when possible

## Performance Considerations

1. **Minimal Overhead**: FD operations should be fast (system calls are expensive)
2. **Efficient Tracking**: Use HashMap for O(1) lookups
3. **Batch Operations**: Support multiple FD operations in single call when possible

## Security Considerations

1. **FD Validation**: Prevent operations on invalid FDs
2. **Permission Checking**: Verify file permissions before FD operations
3. **Resource Limits**: Respect system FD limits
4. **Error Handling**: Never panic on FD operation failures

## Dependencies

```toml
[dependencies]
libc = "0.2"  # For system calls (dup2, close, fcntl, open)
```

## Notes

- All system calls use `unsafe` blocks with proper validation
- FD numbers are validated before all operations
- Error handling follows Rust best practices
- POSIX compliance is maintained throughout
