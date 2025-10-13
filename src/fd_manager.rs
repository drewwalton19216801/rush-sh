use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};

use nix::fcntl::{fcntl, FcntlArg, FdFlag};
use nix::unistd::{close, dup, dup2};

use crate::parser::FdRedirection;

/// Manages file descriptor operations for commands
/// 
/// This module provides POSIX-compliant file descriptor management including:
/// - Arbitrary FD numbers (0-1023)
/// - FD duplication (dup2)
/// - FD redirection to files
/// - FD closing
/// - State save/restore for built-in commands
#[derive(Debug)]
pub struct FdManager {
    /// File descriptors opened by this manager (FD -> File)
    opened_fds: HashMap<RawFd, File>,
    /// Saved FDs for restoration (original FD -> OwnedFd)
    saved_fds: HashMap<RawFd, OwnedFd>,
    /// Redirections to apply
    redirections: Vec<FdRedirection>,
}

impl FdManager {
    /// Create a new FD manager
    pub fn new() -> Self {
        Self {
            opened_fds: HashMap::new(),
            saved_fds: HashMap::new(),
            redirections: Vec::new(),
        }
    }

    /// Prepare redirections for execution
    pub fn prepare_redirections(&mut self, redirections: &[FdRedirection]) {
        self.redirections = redirections.to_vec();
    }

    /// Create a pre_exec closure for external commands
    /// 
    /// This closure will be called in the child process after fork() but before exec()
    /// It applies all FD redirections using dup2() system calls
    pub fn create_pre_exec(redirections: Vec<FdRedirection>) -> impl FnMut() -> io::Result<()> + Send {
        move || {
            for redir in &redirections {
                match redir {
                    FdRedirection::ToFile { fd, filename } => {
                        // Open file for writing
                        let file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(filename)
                            .map_err(|e| io::Error::new(
                                e.kind(),
                                format!("Failed to open {} for FD {}: {}", filename, fd, e)
                            ))?;
                        
                        let file_fd = file.as_raw_fd();
                        if file_fd != *fd as RawFd {
                            unsafe {
                                let mut target = OwnedFd::from_raw_fd(*fd as RawFd);
                                dup2(&file, &mut target)
                                    .map_err(|e| io::Error::new(
                                        io::ErrorKind::Other,
                                        format!("Failed to dup2 for FD {}: {}", fd, e)
                                    ))?;
                                std::mem::forget(target); // Don't close target FD
                            }
                            // Close the original file descriptor
                            let _ = close(file_fd);
                        }
                        
                        // Clear close-on-exec flag so FD survives exec()
                        unsafe {
                            let target_fd = OwnedFd::from_raw_fd(*fd as RawFd);
                            let _ = fcntl(&target_fd, FcntlArg::F_SETFD(FdFlag::empty()));
                            std::mem::forget(target_fd);
                        }
                        
                        // Prevent file from being closed when it goes out of scope
                        std::mem::forget(file);
                    }
                    FdRedirection::AppendToFile { fd, filename } => {
                        // Open file for appending
                        let file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .append(true)
                            .open(filename)
                            .map_err(|e| io::Error::new(
                                e.kind(),
                                format!("Failed to open {} for FD {} append: {}", filename, fd, e)
                            ))?;
                        
                        let file_fd = file.as_raw_fd();
                        if file_fd != *fd as RawFd {
                            unsafe {
                                let mut target = OwnedFd::from_raw_fd(*fd as RawFd);
                                dup2(&file, &mut target)
                                    .map_err(|e| io::Error::new(
                                        io::ErrorKind::Other,
                                        format!("Failed to dup2 for FD {} append: {}", fd, e)
                                    ))?;
                                std::mem::forget(target);
                            }
                            let _ = close(file_fd);
                        }
                        
                        // Clear close-on-exec flag so FD survives exec()
                        unsafe {
                            let target_fd = OwnedFd::from_raw_fd(*fd as RawFd);
                            let _ = fcntl(&target_fd, FcntlArg::F_SETFD(FdFlag::empty()));
                            std::mem::forget(target_fd);
                        }
                        
                        std::mem::forget(file);
                    }
                    FdRedirection::FromFile { fd, filename } => {
                        // Open file for reading
                        let file = OpenOptions::new()
                            .read(true)
                            .open(filename)
                            .map_err(|e| io::Error::new(
                                e.kind(),
                                format!("Failed to open {} for FD {} input: {}", filename, fd, e)
                            ))?;
                        
                        let file_fd = file.as_raw_fd();
                        if file_fd != *fd as RawFd {
                            unsafe {
                                let mut target = OwnedFd::from_raw_fd(*fd as RawFd);
                                dup2(&file, &mut target)
                                    .map_err(|e| io::Error::new(
                                        io::ErrorKind::Other,
                                        format!("Failed to dup2 for FD {} input: {}", fd, e)
                                    ))?;
                                std::mem::forget(target);
                            }
                            let _ = close(file_fd);
                        }
                        
                        // Clear close-on-exec flag so FD survives exec()
                        unsafe {
                            let target_fd = OwnedFd::from_raw_fd(*fd as RawFd);
                            let _ = fcntl(&target_fd, FcntlArg::F_SETFD(FdFlag::empty()));
                            std::mem::forget(target_fd);
                        }
                        
                        std::mem::forget(file);
                    }
                    FdRedirection::DuplicateOutput { source_fd, target_fd } => {
                        // Shell syntax N>&M means "make FD N point to where FD M points for writing"
                        // In dup2 terms: dup2(M, N) - duplicate target_fd to source_fd
                        unsafe {
                            let source = OwnedFd::from_raw_fd(*target_fd as RawFd);
                            let mut target = OwnedFd::from_raw_fd(*source_fd as RawFd);
                            let result = dup2(&source, &mut target);
                            std::mem::forget(source); // Don't close the source FD
                            std::mem::forget(target); // Don't close the target FD
                            result
                        }
                            .map_err(|e| io::Error::new(
                                io::ErrorKind::Other,
                                format!("Failed to duplicate output FD {} to {}: {}", target_fd, source_fd, e)
                            ))?;
                    }
                    FdRedirection::DuplicateInput { source_fd, target_fd } => {
                        // Shell syntax N<&M means "make FD N point to where FD M points for reading"
                        // In dup2 terms: dup2(M, N) - duplicate target_fd to source_fd
                        // The semantics are the same as output duplication at the syscall level
                        unsafe {
                            let source = OwnedFd::from_raw_fd(*target_fd as RawFd);
                            let mut target = OwnedFd::from_raw_fd(*source_fd as RawFd);
                            let result = dup2(&source, &mut target);
                            std::mem::forget(source); // Don't close the source FD
                            std::mem::forget(target); // Don't close the target FD
                            result
                        }
                            .map_err(|e| io::Error::new(
                                io::ErrorKind::Other,
                                format!("Failed to duplicate input FD {} to {}: {}", target_fd, source_fd, e)
                            ))?;
                    }
                    FdRedirection::Close { fd } => {
                        // Close the file descriptor
                        close(*fd as RawFd)
                            .map_err(|e| io::Error::new(
                                io::ErrorKind::Other,
                                format!("Failed to close FD {}: {}", fd, e)
                            ))?;
                    }
                }
            }
            Ok(())
        }
    }

    /// Apply redirections for built-in commands
    /// 
    /// This saves the current state of affected FDs and applies redirections
    /// Call restore() after the built-in completes to restore original state
    pub fn apply_for_builtin(&mut self) -> Result<(), String> {
        for redir in &self.redirections.clone() {
            match redir {
                FdRedirection::ToFile { fd, filename } => {
                    self.save_fd(*fd as RawFd)?;
                    
                    let file = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(filename)
                        .map_err(|e| format!("Failed to open {} for FD {}: {}", filename, fd, e))?;
                    
                    let file_fd = file.as_raw_fd();
                    if file_fd != *fd as RawFd {
                        unsafe {
                            let mut target = OwnedFd::from_raw_fd(*fd as RawFd);
                            dup2(&file, &mut target)
                                .map_err(|e| format!("Failed to dup2 for FD {}: {}", fd, e))?;
                            std::mem::forget(target);
                        }
                    }
                    
                    self.opened_fds.insert(*fd as RawFd, file);
                }
                FdRedirection::AppendToFile { fd, filename } => {
                    self.save_fd(*fd as RawFd)?;
                    
                    let file = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .append(true)
                        .open(filename)
                        .map_err(|e| format!("Failed to open {} for FD {} append: {}", filename, fd, e))?;
                    
                    let file_fd = file.as_raw_fd();
                    if file_fd != *fd as RawFd {
                        unsafe {
                            let mut target = OwnedFd::from_raw_fd(*fd as RawFd);
                            dup2(&file, &mut target)
                                .map_err(|e| format!("Failed to dup2 for FD {} append: {}", fd, e))?;
                            std::mem::forget(target);
                        }
                    }
                    
                    self.opened_fds.insert(*fd as RawFd, file);
                }
                FdRedirection::FromFile { fd, filename } => {
                    self.save_fd(*fd as RawFd)?;
                    
                    let file = OpenOptions::new()
                        .read(true)
                        .open(filename)
                        .map_err(|e| format!("Failed to open {} for FD {} input: {}", filename, fd, e))?;
                    
                    let file_fd = file.as_raw_fd();
                    if file_fd != *fd as RawFd {
                        unsafe {
                            let mut target = OwnedFd::from_raw_fd(*fd as RawFd);
                            dup2(&file, &mut target)
                                .map_err(|e| format!("Failed to dup2 for FD {} input: {}", fd, e))?;
                            std::mem::forget(target);
                        }
                    }
                    
                    self.opened_fds.insert(*fd as RawFd, file);
                }
                FdRedirection::DuplicateOutput { source_fd, target_fd } => {
                    self.save_fd(*source_fd as RawFd)?;
                    
                    // Shell syntax N>&M means "make FD N point to where FD M points for writing"
                    // In dup2 terms: dup2(M, N) - duplicate target_fd to source_fd
                    unsafe {
                        let source = OwnedFd::from_raw_fd(*target_fd as RawFd);
                        let mut target = OwnedFd::from_raw_fd(*source_fd as RawFd);
                        let result = dup2(&source, &mut target);
                        std::mem::forget(source); // Don't close the source FD
                        std::mem::forget(target); // Don't close the target FD
                        result
                    }
                        .map_err(|e| format!("Failed to duplicate output FD {} to {}: {}", target_fd, source_fd, e))?;
                }
                FdRedirection::DuplicateInput { source_fd, target_fd } => {
                    self.save_fd(*source_fd as RawFd)?;
                    
                    // Shell syntax N<&M means "make FD N point to where FD M points for reading"
                    // In dup2 terms: dup2(M, N) - duplicate target_fd to source_fd
                    // The semantics are the same as output duplication at the syscall level
                    unsafe {
                        let source = OwnedFd::from_raw_fd(*target_fd as RawFd);
                        let mut target = OwnedFd::from_raw_fd(*source_fd as RawFd);
                        let result = dup2(&source, &mut target);
                        std::mem::forget(source); // Don't close the source FD
                        std::mem::forget(target); // Don't close the target FD
                        result
                    }
                        .map_err(|e| format!("Failed to duplicate input FD {} to {}: {}", target_fd, source_fd, e))?;
                }
                FdRedirection::Close { fd } => {
                    self.save_fd(*fd as RawFd)?;
                    
                    close(*fd as RawFd)
                        .map_err(|e| format!("Failed to close FD {}: {}", fd, e))?;
                }
            }
        }
        Ok(())
    }

    /// Save the current state of an FD for later restoration
    fn save_fd(&mut self, fd: RawFd) -> Result<(), String> {
        // Only save if not already saved
        if self.saved_fds.contains_key(&fd) {
            return Ok(());
        }

        // Check if FD is valid before trying to save it
        if !is_fd_valid(fd) {
            // FD doesn't exist, no need to save
            return Ok(());
        }

        // Duplicate the FD to save its state
        unsafe {
            let owned_fd = OwnedFd::from_raw_fd(fd);
            match dup(&owned_fd) {
                Ok(saved_fd) => {
                    std::mem::forget(owned_fd); // Don't close the original FD
                    self.saved_fds.insert(fd, saved_fd);
                    Ok(())
                }
                Err(_e) => {
                    std::mem::forget(owned_fd); // Don't close the original FD
                    // If dup fails, the FD might not be open, which is okay
                    Ok(())
                }
            }
        }
    }

    /// Restore saved FDs to their original state
    pub fn restore(&mut self) -> Result<(), String> {
        // Restore all saved FDs
        for (original_fd, saved_fd) in self.saved_fds.drain() {
            unsafe {
                let mut target = OwnedFd::from_raw_fd(original_fd);
                dup2(&saved_fd, &mut target)
                    .map_err(|e| format!("Failed to restore FD {}: {}", original_fd, e))?;
                std::mem::forget(target);
            }
            // saved_fd will be closed when dropped
        }

        // Close all opened FDs
        for (_fd, _file) in self.opened_fds.drain() {
            // Files will be closed when dropped
        }

        Ok(())
    }

    /// Check if any redirections are present
    #[allow(dead_code)]
    pub fn has_redirections(&self) -> bool {
        !self.redirections.is_empty()
    }
}

impl Drop for FdManager {
    fn drop(&mut self) {
        // Ensure cleanup happens even if restore() wasn't called
        let _ = self.restore();
    }
}

/// Check if a file descriptor is valid (open)
fn is_fd_valid(fd: RawFd) -> bool {
    // Try to get FD flags - if this succeeds, the FD is valid
    unsafe {
        let owned_fd = OwnedFd::from_raw_fd(fd);
        let result = fcntl(&owned_fd, FcntlArg::F_GETFD).is_ok();
        std::mem::forget(owned_fd); // Don't close the FD
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_fd_manager_creation() {
        let manager = FdManager::new();
        assert!(!manager.has_redirections());
    }

    #[test]
    fn test_prepare_redirections() {
        let mut manager = FdManager::new();
        let redirections = vec![
            FdRedirection::ToFile {
                fd: 2,
                filename: "test.txt".to_string(),
            },
        ];
        
        manager.prepare_redirections(&redirections);
        assert!(manager.has_redirections());
    }

    #[test]
    fn test_fd_to_file() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_fd_test_{}.txt", timestamp);
        
        let mut manager = FdManager::new();
        let redirections = vec![
            FdRedirection::ToFile {
                fd: 3,
                filename: temp_file.clone(),
            },
        ];
        
        manager.prepare_redirections(&redirections);
        
        // Apply redirections
        manager.apply_for_builtin().unwrap();
        
        // Write to FD 3
        unsafe {
            let mut file = File::from_raw_fd(3);
            writeln!(file, "test data").unwrap();
            file.flush().unwrap();
            std::mem::forget(file); // Don't close FD 3 yet
        }
        
        // Restore
        manager.restore().unwrap();
        
        // Verify file contents
        let contents = std::fs::read_to_string(&temp_file).unwrap();
        assert!(contents.contains("test data"));
        
        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_duplication_output() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_fd_dup_test_{}.txt", timestamp);
        
        // First open FD 3 to a file
        let mut manager = FdManager::new();
        let redirections = vec![
            FdRedirection::ToFile {
                fd: 3,
                filename: temp_file.clone(),
            },
            // Shell syntax 4>&3 means "make FD 4 point to where FD 3 points for writing"
            FdRedirection::DuplicateOutput {
                source_fd: 4,
                target_fd: 3,
            },
        ];
        
        manager.prepare_redirections(&redirections);
        manager.apply_for_builtin().unwrap();
        
        // Write to FD 4 (which now points to the same file as FD 3)
        unsafe {
            let mut file = File::from_raw_fd(4);
            writeln!(file, "duplicated write").unwrap();
            file.flush().unwrap();
            std::mem::forget(file);
        }
        
        manager.restore().unwrap();
        
        // Verify file contents
        let contents = std::fs::read_to_string(&temp_file).unwrap();
        assert!(contents.contains("duplicated write"));
        
        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_is_fd_valid() {
        // Standard FDs should be valid
        assert!(is_fd_valid(0)); // stdin
        assert!(is_fd_valid(1)); // stdout
        assert!(is_fd_valid(2)); // stderr
        
        // High FD numbers should be invalid (unless opened)
        assert!(!is_fd_valid(999));
    }
}