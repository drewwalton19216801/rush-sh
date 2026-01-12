//! File Descriptor Table Management
//!
//! This module provides the file descriptor table implementation for managing
//! open file descriptors in the Rush shell. The FD table is a critical component
//! for I/O redirection operations, allowing the shell to:
//!
//! - Open files and assign them to specific file descriptor numbers
//! - Duplicate file descriptors (e.g., `N>&M`, `N<&M`)
//! - Close file descriptors (e.g., `N>&-`, `N<&-`)
//! - Save and restore file descriptors for subshells and command groups
//!
//! ## File Descriptor Operations
//!
//! The [`FileDescriptorTable`] supports the following operations:
//!
//! - **Opening**: Open a file with specific read/write/append/truncate modes
//! - **Duplication**: Duplicate one FD to another (POSIX dup2 semantics)
//! - **Closing**: Mark an FD as explicitly closed
//! - **Save/Restore**: Save current FD state and restore it later (for subshells)
//!
//! ## Subshell Support
//!
//! The FD table provides save/restore functionality that is essential for proper
//! subshell execution. When entering a subshell:
//!
//! 1. Call [`FileDescriptorTable::save_all_fds`] to save the current state
//! 2. Execute subshell commands (which may modify FDs)
//! 3. Call [`FileDescriptorTable::restore_all_fds`] to restore the original state
//!
//! This ensures that FD modifications in subshells don't affect the parent shell.
//!
//! ## Example
//!
//! ```rust
//! use rush_sh::state::FileDescriptorTable;
//! use std::fs;
//!
//! let mut fd_table = FileDescriptorTable::new();
//!
//! // Create a temporary file for the example
//! let temp_file = "/tmp/rush_fd_example.txt";
//! fs::write(temp_file, "test content").unwrap();
//!
//! // Open a file for reading on FD 3
//! fd_table.open_fd(3, temp_file, true, false, false, false, false).unwrap();
//!
//! // Duplicate FD 3 to FD 4
//! fd_table.duplicate_fd(3, 4).unwrap();
//!
//! // Close FD 3
//! fd_table.close_fd(3).unwrap();
//!
//! // FD 4 still has access to the file
//! assert!(fd_table.is_open(4));
//!
//! // Clean up
//! let _ = fs::remove_file(temp_file);
//! ```

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::process::Stdio;

/// Represents an open file descriptor
#[derive(Debug)]
pub enum FileDescriptor {
    /// Standard file opened for reading, writing, or both
    File(File),
    /// Duplicate of another file descriptor
    Duplicate(RawFd),
    /// Closed file descriptor
    Closed,
}

impl FileDescriptor {
    pub fn try_clone(&self) -> Result<Self, String> {
        match self {
            FileDescriptor::File(f) => {
                let new_file = f
                    .try_clone()
                    .map_err(|e| format!("Failed to clone file: {}", e))?;
                Ok(FileDescriptor::File(new_file))
            }
            FileDescriptor::Duplicate(fd) => Ok(FileDescriptor::Duplicate(*fd)),
            FileDescriptor::Closed => Ok(FileDescriptor::Closed),
        }
    }
}

/// File descriptor table for managing open file descriptors
#[derive(Debug)]
pub struct FileDescriptorTable {
    /// Map of fd number to file descriptor
    fds: HashMap<i32, FileDescriptor>,
    /// Saved file descriptors for restoration after command execution
    saved_fds: HashMap<i32, RawFd>,
}

impl FileDescriptorTable {
    /// Create a new empty file descriptor table
    pub fn new() -> Self {
        Self {
            fds: HashMap::new(),
            saved_fds: HashMap::new(),
        }
    }

    /// Open a file and assign it to a file descriptor number
    ///
    /// # Arguments
    /// * `fd_num` - The file descriptor number (0-9)
    /// * `path` - Path to the file to open
    /// * `read` - Whether to open for reading
    /// * `write` - Whether to open for writing
    /// * `append` - Whether to open in append mode
    /// * `truncate` - Whether to truncate the file
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(String)` with error message on failure
    pub fn open_fd(
        &mut self,
        fd_num: i32,
        path: &str,
        read: bool,
        write: bool,
        append: bool,
        truncate: bool,
        create_new: bool,
    ) -> Result<(), String> {
        let mut opts = OpenOptions::new();
        if create_new {
            opts.create_new(true); // Atomic check-and-create
        } else if truncate {
            opts.create(true).truncate(true);
        }

        // Validate fd number
        if !(0..=1024).contains(&fd_num) {
            return Err(format!("Invalid file descriptor number: {}", fd_num));
        }

        // Open the file with the specified options
        let file = OpenOptions::new()
            .read(read)
            .write(write)
            .append(append)
            .truncate(truncate)
            .create(write || append)
            .open(path)
            .map_err(|e| format!("Cannot open {}: {}", path, e))?;

        // Store the file descriptor
        self.fds.insert(fd_num, FileDescriptor::File(file));
        Ok(())
    }

    /// Duplicate a file descriptor
    ///
    /// # Arguments
    /// * `source_fd` - The source file descriptor to duplicate
    /// * `target_fd` - The target file descriptor number
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(String)` with error message on failure
    pub fn duplicate_fd(&mut self, source_fd: i32, target_fd: i32) -> Result<(), String> {
        // Validate fd numbers
        if !(0..=1024).contains(&source_fd) {
            return Err(format!("Invalid source file descriptor: {}", source_fd));
        }
        if !(0..=1024).contains(&target_fd) {
            return Err(format!("Invalid target file descriptor: {}", target_fd));
        }

        // POSIX: Duplicating to self is a no-op
        if source_fd == target_fd {
            return Ok(());
        }

        // Get the raw fd to duplicate
        let raw_fd = match self.get_raw_fd(source_fd) {
            Some(fd) => fd,
            None => {
                return Err(format!(
                    "File descriptor {} is not open or is closed",
                    source_fd
                ));
            }
        };

        // Store the duplication
        self.fds
            .insert(target_fd, FileDescriptor::Duplicate(raw_fd));
        Ok(())
    }

    /// Close a file descriptor
    ///
    /// # Arguments
    /// * `fd_num` - The file descriptor number to close
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(String)` with error message on failure
    pub fn close_fd(&mut self, fd_num: i32) -> Result<(), String> {
        // Validate fd number
        if !(0..=1024).contains(&fd_num) {
            return Err(format!("Invalid file descriptor number: {}", fd_num));
        }

        // Mark the fd as closed
        self.fds.insert(fd_num, FileDescriptor::Closed);
        Ok(())
    }

    /// Save the current state of a file descriptor for later restoration
    ///
    /// # Arguments
    /// * `fd_num` - The file descriptor number to save
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(String)` with error message on failure
    pub fn save_fd(&mut self, fd_num: i32) -> Result<(), String> {
        // Validate fd number
        if !(0..=1024).contains(&fd_num) {
            return Err(format!("Invalid file descriptor number: {}", fd_num));
        }

        // Duplicate the fd using dup() syscall to save it
        let saved_fd = unsafe {
            let raw_fd = fd_num as RawFd;
            libc::dup(raw_fd)
        };

        if saved_fd < 0 {
            return Err(format!("Failed to save file descriptor {}", fd_num));
        }

        self.saved_fds.insert(fd_num, saved_fd);
        Ok(())
    }

    /// Restore a previously saved file descriptor
    ///
    /// # Arguments
    /// * `fd_num` - The file descriptor number to restore
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(String)` with error message on failure
    pub fn restore_fd(&mut self, fd_num: i32) -> Result<(), String> {
        // Validate fd number
        if !(0..=1024).contains(&fd_num) {
            return Err(format!("Invalid file descriptor number: {}", fd_num));
        }

        // Get the saved fd
        if let Some(saved_fd) = self.saved_fds.remove(&fd_num) {
            // Restore using dup2() syscall
            unsafe {
                let result = libc::dup2(saved_fd, fd_num as RawFd);
                libc::close(saved_fd); // Close the saved fd

                if result < 0 {
                    return Err(format!("Failed to restore file descriptor {}", fd_num));
                }
            }

            // Remove from our tracking
            self.fds.remove(&fd_num);
        }

        Ok(())
    }

    /// Create a deep copy of the file descriptor table
    /// This duplicates all open file descriptors so they are independent of the original table
    pub fn deep_clone(&self) -> Result<Self, String> {
        let mut new_fds = HashMap::new();
        for (fd, descriptor) in &self.fds {
            new_fds.insert(*fd, descriptor.try_clone()?);
        }

        Ok(Self {
            fds: new_fds,
            saved_fds: self.saved_fds.clone(),
        })
    }

    /// Save all currently open file descriptors
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(String)` with error message on failure
    pub fn save_all_fds(&mut self) -> Result<(), String> {
        // Save all fds that we're tracking
        let fd_nums: Vec<i32> = self.fds.keys().copied().collect();
        for fd_num in fd_nums {
            self.save_fd(fd_num)?;
        }

        // Also explicitly save standard FDs (0, 1, 2) if they aren't already tracked
        // This ensures changes to standard streams (via CommandGroup etc.) can be restored
        for fd in 0..=2 {
            if !self.fds.contains_key(&fd) {
                // Try to save, ignore error if fd is closed/invalid
                let _ = self.save_fd(fd);
            }
        }
        Ok(())
    }

    /// Restore all previously saved file descriptors
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(String)` with error message on failure
    pub fn restore_all_fds(&mut self) -> Result<(), String> {
        // Restore all saved fds
        let fd_nums: Vec<i32> = self.saved_fds.keys().copied().collect();
        for fd_num in fd_nums {
            self.restore_fd(fd_num)?;
        }
        Ok(())
    }

    /// Get a file handle for a given file descriptor number
    ///
    /// # Arguments
    /// * `fd_num` - The file descriptor number
    ///
    /// # Returns
    /// * `Some(Stdio)` if the fd is open and can be converted to Stdio
    /// * `None` if the fd is not open or is closed
    #[allow(dead_code)]
    pub fn get_stdio(&self, fd_num: i32) -> Option<Stdio> {
        match self.fds.get(&fd_num) {
            Some(FileDescriptor::File(file)) => {
                // Try to duplicate the file descriptor for Stdio
                let raw_fd = file.as_raw_fd();
                let dup_fd = unsafe { libc::dup(raw_fd) };
                if dup_fd >= 0 {
                    let file = unsafe { File::from_raw_fd(dup_fd) };
                    Some(Stdio::from(file))
                } else {
                    None
                }
            }
            Some(FileDescriptor::Duplicate(raw_fd)) => {
                // Duplicate the raw fd for Stdio
                let dup_fd = unsafe { libc::dup(*raw_fd) };
                if dup_fd >= 0 {
                    let file = unsafe { File::from_raw_fd(dup_fd) };
                    Some(Stdio::from(file))
                } else {
                    None
                }
            }
            Some(FileDescriptor::Closed) | None => None,
        }
    }

    /// Get the raw file descriptor number for a given fd
    ///
    /// # Arguments
    /// * `fd_num` - The file descriptor number
    ///
    /// # Returns
    /// * `Some(RawFd)` if the fd is open
    /// * `None` if the fd is not open or is closed
    pub fn get_raw_fd(&self, fd_num: i32) -> Option<RawFd> {
        match self.fds.get(&fd_num) {
            Some(FileDescriptor::File(file)) => Some(file.as_raw_fd()),
            Some(FileDescriptor::Duplicate(raw_fd)) => Some(*raw_fd),
            Some(FileDescriptor::Closed) => None,
            None => {
                // Standard file descriptors (0, 1, 2) are always open unless explicitly closed
                if fd_num >= 0 && fd_num <= 2 {
                    Some(fd_num as RawFd)
                } else {
                    None
                }
            }
        }
    }

    /// Check if a file descriptor is open
    ///
    /// # Arguments
    /// * `fd_num` - The file descriptor number
    ///
    /// # Returns
    /// * `true` if the fd is open
    /// * `false` if the fd is closed or not tracked
    pub fn is_open(&self, fd_num: i32) -> bool {
        matches!(
            self.fds.get(&fd_num),
            Some(FileDescriptor::File(_)) | Some(FileDescriptor::Duplicate(_))
        )
    }

    /// Check if a file descriptor is closed
    ///
    /// # Arguments
    /// * `fd_num` - The file descriptor number
    ///
    /// # Returns
    /// * `true` if the fd is explicitly closed
    /// * `false` otherwise
    pub fn is_closed(&self, fd_num: i32) -> bool {
        matches!(self.fds.get(&fd_num), Some(FileDescriptor::Closed))
    }

    /// Clear all file descriptors and saved state
    pub fn clear(&mut self) {
        self.fds.clear();
        self.saved_fds.clear();
    }
}

impl Default for FileDescriptorTable {
    /// Creates the default FileDescriptorTable.
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::FileDescriptorTable;
    /// let table = FileDescriptorTable::default();
    /// ```
    fn default() -> Self {
        Self::new()
    }
}