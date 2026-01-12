//! Subshell and compound command execution for the Rush shell.
//!
//! This module handles the execution of subshells and compound commands (command groups),
//! which are critical for POSIX shell compliance and proper state isolation.
//!
//! # Subshell Execution
//!
//! Subshells provide complete state isolation from the parent shell:
//! - Variables, functions, and shell options are cloned
//! - Changes in the subshell don't affect the parent
//! - File descriptor table is deep-cloned for isolation
//! - Current directory changes are isolated
//! - Exit and return commands only affect the subshell
//!
//! # Trap Inheritance
//!
//! Subshells inherit trap handlers from the parent shell:
//! - Trap handlers are cloned when entering a subshell
//! - Changes to traps in the subshell don't affect the parent
//! - Signal queue is inherited but isolated
//!
//! # Exit Code Propagation
//!
//! Exit codes are properly propagated from subshells:
//! - Normal execution returns the last command's exit code
//! - `exit` command in subshell returns its specified code
//! - `return` command in subshell is treated as exit
//! - Parent's `last_exit_code` is updated with subshell result
//!
//! # Depth Limit Protection
//!
//! To prevent stack overflow from excessive nesting:
//! - Maximum subshell depth is 100 levels
//! - Depth is tracked in `ShellState.subshell_depth`
//! - Exceeding the limit returns error code 1
//!
//! # Compound Command Handling
//!
//! Command groups (`{ ... }`) execute in the current shell context:
//! - No state isolation (unlike subshells)
//! - File descriptors are saved and restored around redirections
//! - Can be used in pipelines with proper I/O handling
//!
//! # Pipeline Integration
//!
//! Both subshells and command groups can be used in pipelines:
//! - Input from previous stage is connected via stdin override
//! - Output capture is set up for non-final pipeline stages
//! - Redirections are applied with proper FD save/restore
//! - Stdout redirection detection prevents unnecessary capture

use std::cell::RefCell;
use std::fs::File;
use std::io::{Write, pipe};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::rc::Rc;

use crate::parser::{Ast, Redirection};
use crate::state::ShellState;

use super::expansion::expand_variables_in_string;
use super::redirection::{apply_redirections, write_file_with_noclobber_public};
use super::execute;

/// Maximum allowed subshell nesting depth to prevent stack overflow
const MAX_SUBSHELL_DEPTH: usize = 100;

/// Execute a subshell with isolated state
///
/// # Arguments
/// * `body` - The AST to execute in the subshell
/// * `shell_state` - The parent shell state (will be cloned)
///
/// # Returns
/// * Exit code from the subshell execution
///
/// # Behavior
/// - Clones the shell state for isolation
/// - Executes the body in the cloned state
/// - Returns the exit code without modifying parent state
/// - Preserves parent state completely (variables, functions, etc.)
/// - Tracks subshell depth to prevent stack overflow
/// - Handles exit and return commands properly (isolated from parent)
/// - Cleans up file descriptors to prevent resource leaks
pub(crate) fn execute_subshell(body: Ast, shell_state: &mut ShellState) -> i32 {
    // Check depth limit to prevent stack overflow
    if shell_state.subshell_depth >= MAX_SUBSHELL_DEPTH {
        if shell_state.colors_enabled {
            eprintln!(
                "{}Subshell nesting limit ({}) exceeded\x1b[0m",
                shell_state.color_scheme.error, MAX_SUBSHELL_DEPTH
            );
        } else {
            eprintln!("Subshell nesting limit ({}) exceeded", MAX_SUBSHELL_DEPTH);
        }
        shell_state.last_exit_code = 1;
        return 1;
    }

    // Save current directory for restoration
    let original_dir = std::env::current_dir().ok();

    // Clone the shell state for isolation
    let mut subshell_state = shell_state.clone();

    // Deep clone the file descriptor table for isolation
    // shell_state.clone() only clones the Rc, so we need to manually deep clone the table
    // and put it in a new Rc<RefCell<_>>
    match shell_state.fd_table.borrow().deep_clone() {
        Ok(new_fd_table) => {
            subshell_state.fd_table = Rc::new(RefCell::new(new_fd_table));
        }
        Err(e) => {
            if shell_state.colors_enabled {
                eprintln!(
                    "{}Failed to clone file descriptor table: {}\x1b[0m",
                    shell_state.color_scheme.error, e
                );
            } else {
                eprintln!("Failed to clone file descriptor table: {}", e);
            }
            return 1;
        }
    }

    // Increment subshell depth in the cloned state
    subshell_state.subshell_depth = shell_state.subshell_depth + 1;

    // Clone trap handlers for isolation (subshells inherit but don't affect parent)
    let parent_traps = shell_state.trap_handlers.lock().unwrap().clone();
    subshell_state.trap_handlers = std::sync::Arc::new(std::sync::Mutex::new(parent_traps));

    // Execute the body in the isolated state
    let exit_code = execute(body, &mut subshell_state);

    // Handle exit in subshell: exit should only exit the subshell, not the parent
    // The exit_requested flag is isolated to the subshell_state, so it won't affect parent
    let final_exit_code = if subshell_state.exit_requested {
        // Subshell called exit - use its exit code
        subshell_state.exit_code
    } else if subshell_state.is_returning() {
        // Subshell called return - treat as exit from subshell
        // Return in subshell should not propagate to parent function
        subshell_state.get_return_value().unwrap_or(exit_code)
    } else {
        exit_code
    };

    // Clean up the subshell's file descriptor table to prevent resource leaks
    // This ensures any file descriptors opened in the subshell are properly released
    subshell_state.fd_table.borrow_mut().clear();

    // Restore original directory (in case subshell changed it)
    if let Some(dir) = original_dir {
        let _ = std::env::set_current_dir(dir);
    }

    // Update parent's last_exit_code to reflect subshell result
    shell_state.last_exit_code = final_exit_code;

    // Return the exit code
    final_exit_code
}

/// Execute a compound command with redirections
///
/// # Arguments
/// * `compound_ast` - The compound command AST
/// * `shell_state` - The shell state
/// * `redirections` - Redirections to apply
///
/// # Returns
/// * Exit code from the compound command
pub(crate) fn execute_compound_with_redirections(
    compound_ast: &Ast,
    shell_state: &mut ShellState,
    redirections: &[Redirection],
) -> i32 {
    match compound_ast {
        Ast::CommandGroup { body } => {
            // Save FDs before applying redirections
            if let Err(e) = shell_state.fd_table.borrow_mut().save_all_fds() {
                eprintln!("Error saving FDs: {}", e);
                return 1;
            }

            // Apply redirections to current process
            if let Err(e) = apply_redirections(redirections, shell_state, None) {
                if shell_state.colors_enabled {
                    eprintln!("{}{}\u{001b}[0m", shell_state.color_scheme.error, e);
                } else {
                    eprintln!("{}", e);
                }
                shell_state.fd_table.borrow_mut().restore_all_fds().ok();
                return 1;
            }

            // Execute the group body
            let exit_code = execute(*body.clone(), shell_state);

            // Restore FDs
            if let Err(e) = shell_state.fd_table.borrow_mut().restore_all_fds() {
                eprintln!("Error restoring FDs: {}", e);
            }

            exit_code
        }
        Ast::Subshell { body } => {
            // For subshells with redirections, we need to:
            // 1. Set up output capture if there are output redirections
            // 2. Execute the subshell
            // 3. Apply the redirections to the captured output

            // Check if we have output redirections
            let has_output_redir = redirections.iter().any(|r| {
                matches!(
                    r,
                    Redirection::Output(_)
                        | Redirection::Append(_)
                        | Redirection::FdOutput(_, _)
                        | Redirection::FdAppend(_, _)
                )
            });

            if has_output_redir {
                // Clone state for subshell
                let mut subshell_state = shell_state.clone();

                // Set up output capture
                let capture_buffer = Rc::new(RefCell::new(Vec::new()));
                subshell_state.capture_output = Some(capture_buffer.clone());

                // Execute subshell
                let exit_code = execute(*body.clone(), &mut subshell_state);

                // Get captured output
                let output = capture_buffer.borrow().clone();

                // Apply redirections to output
                for redir in redirections {
                    match redir {
                        Redirection::Output(file) => {
                            let expanded_file = expand_variables_in_string(file, shell_state);
                            
                            // Use atomic write helper to prevent TOCTOU race condition
                            if let Err(e) = write_file_with_noclobber_public(
                                &expanded_file,
                                &output,
                                shell_state.options.noclobber,
                                false, // not force_clobber
                                shell_state,
                            ) {
                                eprintln!("Redirection error: {}", e);
                                return 1;
                            }
                        }
                        Redirection::OutputClobber(file) => {
                            let expanded_file = expand_variables_in_string(file, shell_state);
                            // >| always overwrites, even with noclobber set
                            if let Err(e) = write_file_with_noclobber_public(
                                &expanded_file,
                                &output,
                                false, // noclobber doesn't apply
                                true,  // force_clobber
                                shell_state,
                            ) {
                                eprintln!("Redirection error: {}", e);
                                return 1;
                            }
                        }
                        Redirection::Append(file) => {
                            let expanded_file = expand_variables_in_string(file, shell_state);
                            use std::fs::OpenOptions;
                            let mut file_handle = match OpenOptions::new()
                                .append(true)
                                .create(true)
                                .open(&expanded_file)
                            {
                                Ok(f) => f,
                                Err(e) => {
                                    if shell_state.colors_enabled {
                                        eprintln!(
                                            "{}Redirection error: {}\x1b[0m",
                                            shell_state.color_scheme.error, e
                                        );
                                    } else {
                                        eprintln!("Redirection error: {}", e);
                                    }
                                    return 1;
                                }
                            };
                            if let Err(e) = file_handle.write_all(&output) {
                                if shell_state.colors_enabled {
                                    eprintln!(
                                        "{}Redirection error: {}\x1b[0m",
                                        shell_state.color_scheme.error, e
                                    );
                                } else {
                                    eprintln!("Redirection error: {}", e);
                                }
                                return 1;
                            }
                        }
                        _ => {
                            // For Phase 2, only support basic output redirections
                            // Other redirections are silently ignored for subshells
                        }
                    }
                }

                shell_state.last_exit_code = exit_code;
                exit_code
            } else {
                // No output redirections, execute normally
                execute_subshell(*body.clone(), shell_state)
            }
        }
        _ => {
            eprintln!("Unsupported compound command type");
            1
        }
    }
}

/// Check if redirections include stdout redirections
/// Returns true if any redirection affects stdout (FD 1)
fn has_stdout_redirection(redirections: &[Redirection]) -> bool {
    redirections.iter().any(|r| match r {
        // Default output redirections affect stdout (FD 1)
        Redirection::Output(_) | Redirection::OutputClobber(_) | Redirection::Append(_) => true,
        // Explicit FD 1 redirections
        Redirection::FdOutput(1, _) | Redirection::FdAppend(1, _) => true,
        // FD 1 duplication or closure
        Redirection::FdDuplicate(1, _) | Redirection::FdClose(1) => true,
        // All other redirections don't affect stdout
        _ => false,
    })
}

/// Execute a compound command (subshell) as part of a pipeline
///
/// # Arguments
/// * `compound_ast` - The compound command AST (typically Subshell)
/// * `shell_state` - The parent shell state
/// * `stdin` - Optional stdin file from previous pipeline stage
/// * `is_first` - Whether this is the first command in the pipeline
/// * `is_last` - Whether this is the last command in the pipeline
/// * `redirections` - Redirections to apply to the compound command
///
/// # Returns
/// * Tuple of (exit code, optional stdout file for next stage)
pub(crate) fn execute_compound_in_pipeline(
    compound_ast: &Ast,
    shell_state: &mut ShellState,
    stdin: Option<File>,
    is_first: bool,
    is_last: bool,
    redirections: &[Redirection],
) -> (i32, Option<File>) {
    match compound_ast {
        Ast::Subshell { body } | Ast::CommandGroup { body } => {
            // Clone state for subshell
            let mut subshell_state = shell_state.clone();

            // Setup stdin from provided file if available
            // We must keep the file alive for the duration of the subshell execution.
            let mut _stdin_file = stdin;

            if let Some(ref f) = _stdin_file {
                let fd = f.as_raw_fd();
                subshell_state.stdin_override = Some(fd);
            } else if !is_first && subshell_state.stdin_override.is_none() {
                // If we have no input from previous stage and no override, use /dev/null
                if let Ok(f) = File::open("/dev/null") {
                    subshell_state.stdin_override = Some(f.as_raw_fd());
                    _stdin_file = Some(f);
                }
            }

            // Setup output capture if not last or if parent is capturing
            // BUT skip capture if stdout is redirected (e.g., { pwd; } > out | wc -l)
            let capture_buffer = if (!is_last || shell_state.capture_output.is_some())
                && !has_stdout_redirection(redirections)
            {
                let buffer = Rc::new(RefCell::new(Vec::new()));
                subshell_state.capture_output = Some(buffer.clone());
                Some(buffer)
            } else {
                None
            };

            // Apply redirections (saving/restoring if it's a group)
            let exit_code = if matches!(compound_ast, Ast::CommandGroup { .. }) {
                // Save FDs before applying redirections
                if let Err(e) = subshell_state.fd_table.borrow_mut().save_all_fds() {
                    eprintln!("Error saving FDs: {}", e);
                    return (1, None);
                }

                // If we have a pipe from previous stage, hook it up to FD 0 for builtins
                if let Some(ref f) = _stdin_file {
                    unsafe {
                        libc::dup2(f.as_raw_fd(), 0);
                    }
                }

                // Apply redirections to current process
                if let Err(e) = apply_redirections(redirections, &mut subshell_state, None) {
                    if subshell_state.colors_enabled {
                        eprintln!("{}{}\u{001b}[0m", subshell_state.color_scheme.error, e);
                    } else {
                        eprintln!("{}", e);
                    }
                    subshell_state.fd_table.borrow_mut().restore_all_fds().ok();
                    return (1, None);
                }

                // Execute the body
                let code = execute(*body.clone(), &mut subshell_state);

                // Restore FDs
                if let Err(e) = subshell_state.fd_table.borrow_mut().restore_all_fds() {
                    eprintln!("Error restoring FDs: {}", e);
                }
                code
            } else {
                // Subshell handling (non-forking)
                if let Err(e) = subshell_state.fd_table.borrow_mut().save_all_fds() {
                    eprintln!("Error saving FDs: {}", e);
                    return (1, None);
                }

                // If we have a pipe from previous stage, hook it up to FD 0
                if let Some(ref f) = _stdin_file {
                    unsafe {
                        libc::dup2(f.as_raw_fd(), 0);
                    }
                }

                if let Err(e) = apply_redirections(redirections, &mut subshell_state, None) {
                    eprintln!("{}", e);
                    subshell_state.fd_table.borrow_mut().restore_all_fds().ok();
                    return (1, None);
                }
                let code = execute(*body.clone(), &mut subshell_state);
                subshell_state.fd_table.borrow_mut().restore_all_fds().ok();
                code
            };

            // Prepare stdout for next stage if captured
            let mut next_stdout = None;
            if let Some(buffer) = capture_buffer {
                let captured = buffer.borrow().clone();

                // If not last, create a pipe and write captured output to it
                if !is_last {
                    use std::io::Write;
                    let (reader, mut writer) = match pipe() {
                        Ok((r, w)) => (r, w),
                        Err(e) => {
                            eprintln!("Error creating pipe for compound command: {}", e);
                            return (exit_code, None);
                        }
                    };
                    if let Err(e) = writer.write_all(&captured) {
                        eprintln!("Error writing to pipe: {}", e);
                    }
                    drop(writer); // Close write end so reader sees EOF

                    next_stdout = Some(unsafe { File::from_raw_fd(reader.into_raw_fd()) });
                }

                // If parent is capturing, also pass data up
                if let Some(ref parent_capture) = shell_state.capture_output {
                    parent_capture.borrow_mut().extend_from_slice(&captured);
                }
            }

            shell_state.last_exit_code = exit_code;
            (exit_code, next_stdout)
        }
        _ => {
            eprintln!("Unsupported compound command in pipeline");
            (1, None)
        }
    }
}