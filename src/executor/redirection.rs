//! I/O redirection handling for the Rush shell.
//!
//! This module provides comprehensive support for POSIX-compliant I/O redirections,
//! including file descriptor operations, here-documents, and here-strings.
//!
//! # Redirection Types
//!
//! The module handles the following redirection types:
//!
//! ## Basic Redirections
//! - **Input redirection** (`<`): Redirects stdin from a file
//! - **Output redirection** (`>`): Redirects stdout to a file (with noclobber support)
//! - **Output clobber** (`>|`): Forces overwrite even with noclobber set
//! - **Append redirection** (`>>`): Appends stdout to a file
//!
//! ## File Descriptor Operations
//! - **FD input** (`N<file`): Opens file for reading on FD N
//! - **FD output** (`N>file`): Opens file for writing on FD N
//! - **FD append** (`N>>file`): Opens file for appending on FD N
//! - **FD duplication** (`N>&M`, `N<&M`): Duplicates FD M to FD N
//! - **FD close** (`N>&-`, `N<&-`): Closes FD N
//! - **FD read/write** (`N<>file`): Opens file for both reading and writing on FD N
//!
//! ## Here-documents and Here-strings
//! - **Here-document** (`<<DELIMITER`): Provides multi-line input from the script
//! - **Here-string** (`<<<word`): Provides a single line of input
//!
//! # File Descriptor Table Integration
//!
//! All redirection operations interact with the shell's file descriptor table
//! ([`FileDescriptorTable`](crate::state::FileDescriptorTable)) to maintain
//! proper state across command executions, subshells, and command groups.
//!
//! # Noclobber Support
//!
//! When the `noclobber` shell option is enabled (`set -C`), output redirections
//! that would overwrite existing files are rejected unless the clobber operator
//! (`>|`) is used. This prevents accidental file overwrites.
//!
//! # Examples
//!
//! ```no_run
//! use rush_sh::ShellState;
//! use rush_sh::parser::Redirection;
//! use std::process::Command;
//!
//! let mut shell_state = ShellState::new();
//! let redirections = vec![
//!     Redirection::Output("output.txt".into()),
//!     Redirection::FdOutput(2, "errors.txt".into()),
//! ];
//!
//! // For external commands:
//! let mut cmd = Command::new("echo");
//! // apply_redirections(&redirections, &mut shell_state, Some(&mut cmd))?;
//!
//! // For builtins (command is None):
//! // apply_redirections(&redirections, &mut shell_state, None)?;
//! ```

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write, pipe};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use super::expansion::expand_variables_in_string;
use crate::parser::Redirection;
use crate::state::ShellState;

/// Atomically write data to a file, respecting noclobber settings
///
/// When noclobber is enabled and force_clobber is false, uses create_new()
/// to atomically fail if file exists. Otherwise allows overwriting.
fn write_file_with_noclobber(
    path: &str,
    data: &[u8],
    noclobber: bool,
    force_clobber: bool,
    shell_state: &ShellState,
) -> Result<(), String> {
    use std::fs::OpenOptions;

    if noclobber && !force_clobber {
        // Atomic check-and-create: fails if file exists
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    if shell_state.colors_enabled {
                        format!(
                            "{}cannot overwrite existing file '{}' (noclobber is set)\x1b[0m",
                            shell_state.color_scheme.error, path
                        )
                    } else {
                        format!(
                            "cannot overwrite existing file '{}' (noclobber is set)",
                            path
                        )
                    }
                } else if shell_state.colors_enabled {
                    format!(
                        "{}Cannot create {}: {}\x1b[0m",
                        shell_state.color_scheme.error, path, e
                    )
                } else {
                    format!("Cannot create {}: {}", path, e)
                }
            })?;

        file.write_all(data).map_err(|e| {
            if shell_state.colors_enabled {
                format!(
                    "{}Failed to write to {}: {}\x1b[0m",
                    shell_state.color_scheme.error, path, e
                )
            } else {
                format!("Failed to write to {}: {}", path, e)
            }
        })?;
    } else {
        // Allow overwriting (normal behavior or force_clobber)
        std::fs::write(path, data).map_err(|e| {
            if shell_state.colors_enabled {
                format!(
                    "{}Cannot write to {}: {}\x1b[0m",
                    shell_state.color_scheme.error, path, e
                )
            } else {
                format!("Cannot write to {}: {}", path, e)
            }
        })?;
    }

    Ok(())
}

/// Collect here-document content from stdin until the specified delimiter is found
/// This function reads from stdin line by line until it finds a line that exactly matches the delimiter
/// If shell_state has pending_heredoc_content, it uses that instead (for script execution)
pub(crate) fn collect_here_document_content(
    delimiter: &str,
    shell_state: &mut ShellState,
) -> String {
    // Check if we have pending here-document content from script execution
    if let Some(content) = shell_state.pending_heredoc_content.take() {
        return content;
    }

    // Otherwise, read from stdin (interactive mode)
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut content = String::new();
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // EOF reached
                break;
            }
            Ok(_) => {
                // Check if this line (without trailing newline) matches the delimiter
                let line_content = line.trim_end();
                if line_content == delimiter {
                    // Found the delimiter, stop collecting
                    break;
                } else {
                    // This is content, add it to our collection
                    content.push_str(&line);
                }
            }
            Err(e) => {
                if shell_state.colors_enabled {
                    eprintln!(
                        "{}Error reading here-document content: {}\x1b[0m",
                        shell_state.color_scheme.error, e
                    );
                } else {
                    eprintln!("Error reading here-document content: {}", e);
                }
                break;
            }
        }
    }

    content
}

/// Apply a sequence of redirections to a command or to the current process in left-to-right order.
///
/// Applies each redirection in the provided slice to the optional `Command` (when executing an external
/// command) or to the shell's file descriptor table for the current process. Redirections are processed
/// left-to-right to match POSIX semantics; on the first failure no further redirections are applied.
///
/// # Errors
///
/// Returns `Err(String)` with a diagnostic message if any redirection fails; returns `Ok(())` on success.
///
/// # Examples
///
/// ```no_run
/// use rush_sh::ShellState;
/// use rush_sh::parser::Redirection;
/// use std::process::Command;
/// // Example showing the function signature
/// let mut shell_state = ShellState::new();
/// let mut cmd = Command::new("cat");
/// let reds = vec![Redirection::Output("out.txt".into())];
/// // apply_redirections(&reds, &mut shell_state, Some(&mut cmd))?;
/// ```
pub fn apply_redirections(
    redirections: &[Redirection],
    shell_state: &mut ShellState,
    mut command: Option<&mut Command>,
) -> Result<(), String> {
    // Process redirections in left-to-right order per POSIX
    for redir in redirections {
        match redir {
            Redirection::Input(file) => {
                apply_input_redirection(0, file, shell_state, command.as_deref_mut())?;
            }
            Redirection::Output(file) => {
                apply_output_redirection(
                    1,
                    file,
                    false,
                    false,
                    shell_state,
                    command.as_deref_mut(),
                )?;
            }
            Redirection::OutputClobber(file) => {
                apply_output_redirection(
                    1,
                    file,
                    false,
                    true,
                    shell_state,
                    command.as_deref_mut(),
                )?;
            }
            Redirection::Append(file) => {
                apply_output_redirection(
                    1,
                    file,
                    true,
                    false,
                    shell_state,
                    command.as_deref_mut(),
                )?;
            }
            Redirection::FdInput(fd, file) => {
                apply_input_redirection(*fd, file, shell_state, command.as_deref_mut())?;
            }
            Redirection::FdOutput(fd, file) => {
                apply_output_redirection(
                    *fd,
                    file,
                    false,
                    false,
                    shell_state,
                    command.as_deref_mut(),
                )?;
            }
            Redirection::FdOutputClobber(fd, file) => {
                apply_output_redirection(
                    *fd,
                    file,
                    false,
                    true,
                    shell_state,
                    command.as_deref_mut(),
                )?;
            }
            Redirection::FdAppend(fd, file) => {
                apply_output_redirection(
                    *fd,
                    file,
                    true,
                    false,
                    shell_state,
                    command.as_deref_mut(),
                )?;
            }
            Redirection::FdDuplicate(target_fd, source_fd) => {
                apply_fd_duplication(*target_fd, *source_fd, shell_state, command.as_deref_mut())?;
            }
            Redirection::FdClose(fd) => {
                apply_fd_close(*fd, shell_state, command.as_deref_mut())?;
            }
            Redirection::FdInputOutput(fd, file) => {
                apply_fd_input_output(*fd, file, shell_state, command.as_deref_mut())?;
            }
            Redirection::HereDoc(delimiter, quoted_str) => {
                let quoted = quoted_str == "true";
                apply_heredoc_redirection(
                    0,
                    delimiter,
                    quoted,
                    shell_state,
                    command.as_deref_mut(),
                )?;
            }
            Redirection::HereString(content) => {
                apply_herestring_redirection(0, content, shell_state, command.as_deref_mut())?;
            }
        }
    }
    Ok(())
}

/// Apply input redirection for a specific file descriptor
fn apply_input_redirection(
    fd: i32,
    file: &str,
    shell_state: &mut ShellState,
    command: Option<&mut Command>,
) -> Result<(), String> {
    let expanded_file = expand_variables_in_string(file, shell_state);

    // Open file for reading
    let file_handle =
        File::open(&expanded_file).map_err(|e| format!("Cannot open {}: {}", expanded_file, e))?;

    if fd == 0 {
        // stdin redirection - apply to Command if present
        if let Some(cmd) = command {
            cmd.stdin(Stdio::from(file_handle));
        } else {
            // For builtins or command groups (command is None), redirect shell's stdin
            shell_state.fd_table.borrow_mut().open_fd(
                0,
                &expanded_file,
                true,  // read
                false, // write
                false, // append
                false, // truncate
                false, // clobber
            )?;

            // Also perform OS-level dup2
            let raw_fd = shell_state.fd_table.borrow().get_raw_fd(0);
            if let Some(rfd) = raw_fd
                && rfd != 0
            {
                unsafe {
                    if libc::dup2(rfd, 0) < 0 {
                        return Err(format!("Failed to dup2 fd {} to 0", rfd));
                    }
                }
            }
        }
    } else {
        // Custom fd - for external commands, we need to redirect the custom fd for reading
        // Open the file (we need to keep the handle alive for the command)
        let fd_file = File::open(&expanded_file)
            .map_err(|e| format!("Cannot open {}: {}", expanded_file, e))?;

        // For external commands, store both in fd table and prepare for stdin redirect
        shell_state.fd_table.borrow_mut().open_fd(
            fd,
            &expanded_file,
            true,  // read
            false, // write
            false, // append
            false, // truncate
            false, // clobber
        )?;

        // If we have an external command, set up the file descriptor in the child process
        if let Some(cmd) = command {
            // Keep fd_file alive by moving it into the closure
            // It will be dropped (and closed) when the closure is dropped in the parent
            let target_fd = fd;
            unsafe {
                cmd.pre_exec(move || {
                    let raw_fd = fd_file.as_raw_fd();

                    // The inherited file descriptor might not be at the target fd number
                    // Use dup2 to ensure it's at the correct fd number
                    if raw_fd != target_fd {
                        let result = libc::dup2(raw_fd, target_fd);
                        if result < 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                        // We don't need to close raw_fd manually because fd_file
                        // has CLOEXEC set by default and will be closed on exec
                    }
                    Ok(())
                });
            }
        }
    }

    Ok(())
}

/// Apply output redirection for a specific file descriptor
fn apply_output_redirection(
    fd: i32,
    file: &str,
    append: bool,
    force_clobber: bool,
    shell_state: &mut ShellState,
    command: Option<&mut Command>,
) -> Result<(), String> {
    let expanded_file = expand_variables_in_string(file, shell_state);

    // Open file for writing or appending
    // For noclobber with > (not append, not force_clobber), use atomic create_new()
    let file_handle = if append {
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(&expanded_file)
            .map_err(|e| {
                if shell_state.colors_enabled {
                    format!(
                        "{}Cannot open {}: {}\x1b[0m",
                        shell_state.color_scheme.error, expanded_file, e
                    )
                } else {
                    format!("Cannot open {}: {}", expanded_file, e)
                }
            })?
    } else if shell_state.options.noclobber && !force_clobber {
        // Atomic check-and-create: fails if file exists
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&expanded_file)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    if shell_state.colors_enabled {
                        format!(
                            "{}cannot overwrite existing file '{}' (noclobber is set)\x1b[0m",
                            shell_state.color_scheme.error, expanded_file
                        )
                    } else {
                        format!(
                            "cannot overwrite existing file '{}' (noclobber is set)",
                            expanded_file
                        )
                    }
                } else if shell_state.colors_enabled {
                    format!(
                        "{}Cannot create {}: {}\x1b[0m",
                        shell_state.color_scheme.error, expanded_file, e
                    )
                } else {
                    format!("Cannot create {}: {}", expanded_file, e)
                }
            })?
    } else {
        // Normal create (truncate) or force_clobber
        File::create(&expanded_file).map_err(|e| {
            if shell_state.colors_enabled {
                format!(
                    "{}Cannot create {}: {}\x1b[0m",
                    shell_state.color_scheme.error, expanded_file, e
                )
            } else {
                format!("Cannot create {}: {}", expanded_file, e)
            }
        })?
    };

    if let Some(cmd) = command {
        if fd == 1 {
            // stdout redirection - apply to Command if present
            cmd.stdout(Stdio::from(file_handle));
        } else if fd == 2 {
            // stderr redirection - apply to Command if present
            cmd.stderr(Stdio::from(file_handle));
        } else {
            // Custom fd - store in fd table (and pre_exec will handle it?)
            // Actually, for external commands, custom FDs need to be inherited/set up.
            // But we can update the shell's FD table temporarily if we want?
            // Existing logic for custom FD WAS to update fd_table.
            shell_state.fd_table.borrow_mut().open_fd(
                fd,
                &expanded_file,
                false, // read
                true,  // write
                append,
                !append, // truncate if not appending
                false,   // clobber
            )?;
        }
    } else {
        // Current process redirection (builtins, command groups)
        // Check noclobber before opening in fd_table
        if shell_state.options.noclobber && !force_clobber && !append {
            // Check if file exists before opening
            if std::path::Path::new(&expanded_file).exists() {
                let error_msg = if shell_state.colors_enabled {
                    format!(
                        "{}cannot overwrite existing file '{}' (noclobber is set)\x1b[0m",
                        shell_state.color_scheme.error, expanded_file
                    )
                } else {
                    format!(
                        "cannot overwrite existing file '{}' (noclobber is set)",
                        expanded_file
                    )
                };
                return Err(error_msg);
            }
        }

        // Now safe to open - we MUST update the file descriptor table for ALL FDs including 1 and 2
        shell_state.fd_table.borrow_mut().open_fd(
            fd,
            &expanded_file,
            false, // read
            true,  // write
            append,
            !append, // truncate if not appending
            shell_state.options.noclobber && !force_clobber && !append, // create_new
        )?;

        // Also perform OS-level dup2 to ensure child processes inherit the redirection
        // (This is critical for external commands running inside command groups)
        let raw_fd = shell_state.fd_table.borrow().get_raw_fd(fd);
        if let Some(rfd) = raw_fd {
            // Avoid dup2-ing to itself if raw_fd happens to equal fd (unlikely but possible if we closed 1 then opened)
            if rfd != fd {
                unsafe {
                    if libc::dup2(rfd, fd) < 0 {
                        return Err(format!("Failed to dup2 fd {} to {}", rfd, fd));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Apply file descriptor duplication
fn apply_fd_duplication(
    target_fd: i32,
    source_fd: i32,
    shell_state: &mut ShellState,
    _command: Option<&mut Command>,
) -> Result<(), String> {
    // Check if source_fd is explicitly closed before attempting duplication
    if shell_state.fd_table.borrow().is_closed(source_fd) {
        let error_msg = format!("File descriptor {} is closed", source_fd);
        if shell_state.colors_enabled {
            eprintln!(
                "{}Redirection error: {}\x1b[0m",
                shell_state.color_scheme.error, error_msg
            );
        } else {
            eprintln!("Redirection error: {}", error_msg);
        }
        return Err(error_msg);
    }

    // Duplicate source_fd to target_fd
    shell_state
        .fd_table
        .borrow_mut()
        .duplicate_fd(source_fd, target_fd)?;
    Ok(())
}

/// Apply file descriptor closing
fn apply_fd_close(
    fd: i32,
    shell_state: &mut ShellState,
    command: Option<&mut Command>,
) -> Result<(), String> {
    // Close the specified fd in the fd table
    shell_state.fd_table.borrow_mut().close_fd(fd)?;

    // For external commands, we need to redirect the fd to /dev/null
    // This ensures that writes to the closed fd don't produce errors
    if let Some(cmd) = command {
        match fd {
            0 => {
                // Close stdin - redirect to /dev/null for reading
                cmd.stdin(Stdio::null());
            }
            1 => {
                // Close stdout - redirect to /dev/null for writing
                cmd.stdout(Stdio::null());
            }
            2 => {
                // Close stderr - redirect to /dev/null for writing
                cmd.stderr(Stdio::null());
            }
            _ => {
                // For custom fds (3+), we use pre_exec to close them
                // This is handled via the fd_table and dup2 operations
            }
        }
    }

    Ok(())
}

/// Apply read/write file descriptor opening
fn apply_fd_input_output(
    fd: i32,
    file: &str,
    shell_state: &mut ShellState,
    _command: Option<&mut Command>,
) -> Result<(), String> {
    let expanded_file = expand_variables_in_string(file, shell_state);

    // Open file for both reading and writing
    shell_state.fd_table.borrow_mut().open_fd(
        fd,
        &expanded_file,
        true,  // read
        true,  // write
        false, // append
        false, // truncate
        false, // create_new
    )?;

    Ok(())
}

/// Apply here-document redirection
fn apply_heredoc_redirection(
    fd: i32,
    delimiter: &str,
    quoted: bool,
    shell_state: &mut ShellState,
    command: Option<&mut Command>,
) -> Result<(), String> {
    let here_doc_content = collect_here_document_content(delimiter, shell_state);

    // Expand variables and command substitutions ONLY if delimiter was not quoted
    let expanded_content = if quoted {
        here_doc_content
    } else {
        expand_variables_in_string(&here_doc_content, shell_state)
    };

    // Create a pipe and write the content
    let (reader, mut writer) =
        pipe().map_err(|e| format!("Failed to create pipe for here-document: {}", e))?;

    writeln!(writer, "{}", expanded_content)
        .map_err(|e| format!("Failed to write here-document content: {}", e))?;

    // Apply to stdin if fd is 0
    if fd == 0
        && let Some(cmd) = command
    {
        cmd.stdin(Stdio::from(reader));
    }

    Ok(())
}

/// Apply here-string redirection
fn apply_herestring_redirection(
    fd: i32,
    content: &str,
    shell_state: &mut ShellState,
    command: Option<&mut Command>,
) -> Result<(), String> {
    let expanded_content = expand_variables_in_string(content, shell_state);

    // Create a pipe and write the content
    let (reader, mut writer) =
        pipe().map_err(|e| format!("Failed to create pipe for here-string: {}", e))?;

    write!(writer, "{}", expanded_content)
        .map_err(|e| format!("Failed to write here-string content: {}", e))?;

    // Apply to stdin if fd is 0
    if fd == 0
        && let Some(cmd) = command
    {
        cmd.stdin(Stdio::from(reader));
    }

    Ok(())
}

/// Helper function used by compound command execution
/// Atomically write data to a file with noclobber support (public for use in executor)
pub(crate) fn write_file_with_noclobber_public(
    path: &str,
    data: &[u8],
    noclobber: bool,
    force_clobber: bool,
    shell_state: &ShellState,
) -> Result<(), String> {
    write_file_with_noclobber(path, data, noclobber, force_clobber, shell_state)
}
