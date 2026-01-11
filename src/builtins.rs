use std::io::{self, Write};
use std::os::unix::io::FromRawFd;

use crate::parser::ShellCommand;
use crate::state::ShellState;

/// A writer wrapper for output handling
pub struct ColoredWriter<W: Write> {
    inner: W,
}

impl<W: Write> ColoredWriter<W> {
    pub fn new(inner: W) -> Self {
        Self { inner }
    }
}

impl<W: Write> Write for ColoredWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// A writer that always returns EBADF
pub struct BadFdWriter;

impl Write for BadFdWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::from_raw_os_error(libc::EBADF))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from_raw_os_error(libc::EBADF))
    }
}

mod builtin_alias;
mod builtin_break;
mod builtin_cd;
mod builtin_continue;
mod builtin_declare;
mod builtin_dirs;
mod builtin_env;
mod builtin_exit;
mod builtin_export;
mod builtin_help;
mod builtin_popd;
mod builtin_pushd;
mod builtin_pwd;
mod builtin_return;
mod builtin_set;
mod builtin_set_color_scheme;
mod builtin_set_colors;
mod builtin_set_condensed;
mod builtin_shift;
mod builtin_source;
mod builtin_test;
mod builtin_trap;
mod builtin_type;
mod builtin_unalias;
mod builtin_unset;

pub trait Builtin {
    fn name(&self) -> &'static str;
    fn names(&self) -> Vec<&'static str>;
    fn description(&self) -> &'static str;
    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32;
}

/// Provides a vector of all builtin command implementations in registration order.
///
/// Each element is a boxed implementation of `Builtin` representing one builtin command
/// available to the shell.
///
/// # Examples
///
/// ```
/// let builtins = crate::builtins::get_builtins();
/// let names: Vec<_> = builtins.iter().map(|b| b.name()).collect();
/// assert!(names.contains(&"cd"));
/// assert!(names.contains(&"pwd"));
/// ```
fn get_builtins() -> Vec<Box<dyn Builtin>> {
    vec![
        Box::new(builtin_cd::CdBuiltin),
        Box::new(builtin_pwd::PwdBuiltin),
        Box::new(builtin_env::EnvBuiltin),
        Box::new(builtin_exit::ExitBuiltin),
        Box::new(builtin_help::HelpBuiltin),
        Box::new(builtin_source::SourceBuiltin),
        Box::new(builtin_export::ExportBuiltin),
        Box::new(builtin_unset::UnsetBuiltin),
        Box::new(builtin_pushd::PushdBuiltin),
        Box::new(builtin_popd::PopdBuiltin),
        Box::new(builtin_dirs::DirsBuiltin),
        Box::new(builtin_alias::AliasBuiltin),
        Box::new(builtin_unalias::UnaliasBuiltin),
        Box::new(builtin_test::TestBuiltin),
        Box::new(builtin_set::SetBuiltin),
        Box::new(builtin_set_colors::SetColorsBuiltin),
        Box::new(builtin_set_color_scheme::SetColorSchemeBuiltin),
        Box::new(builtin_set_condensed::SetCondensedBuiltin),
        Box::new(builtin_shift::ShiftBuiltin),
        Box::new(builtin_declare::DeclareBuiltin),
        Box::new(builtin_trap::TrapBuiltin),
        Box::new(builtin_type::TypeBuiltin),
        Box::new(builtin_return::ReturnBuiltin),
        Box::new(builtin_break::BreakBuiltin),
        Box::new(builtin_continue::ContinueBuiltin),
    ]
}

pub fn is_builtin(cmd: &str) -> bool {
    get_builtins().iter().any(|b| b.names().contains(&cmd))
}

pub fn get_builtin_commands() -> Vec<String> {
    let builtins = get_builtins();
    let mut commands = Vec::new();
    for b in builtins {
        for &name in &b.names() {
            commands.push(name.to_string());
        }
    }
    commands
}

/// Execute a builtin command, applying redirections and selecting the appropriate output writer.
///
/// This function locates and runs the builtin named by `cmd.args[0]`, applying any redirections
/// from `cmd.redirections` in left-to-right order, expanding filenames using `shell_state`,
/// saving and restoring file descriptors around the builtin invocation, and selecting stdout
/// from the shell's file-descriptor table (or using a sink writer if stdout is closed).
/// If `output_override` is provided, it is used directly as the builtin's output writer and
/// redirections are not applied. Colored error messages are printed according to `shell_state`'s
/// color settings. On success it returns the builtin's exit code; on failure it returns `1`.
///
/// # Examples
///
/// ```no_run
/// // Construct a ShellCommand and ShellState appropriately in real code.
/// // Here we show the call site pattern only.
/// # use std::io::Write;
/// # struct ShellCommand { args: Vec<String>, redirections: Vec<()> }
/// # struct ShellState { colors_enabled: bool, color_scheme: (), fd_table: std::rc::Rc<std::cell::RefCell<()>> }
/// # fn example_call(cmd: &ShellCommand, state: &mut ShellState) {
/// let exit_code = crate::builtins::execute_builtin(cmd, state, None);
/// println!("exit code: {}", exit_code);
/// # }
/// ```
pub fn execute_builtin(
    cmd: &ShellCommand,
    shell_state: &mut ShellState,
    output_override: Option<Box<dyn Write>>,
) -> i32 {
    // Helper function for colored error messages
    let colors_enabled = shell_state.colors_enabled;
    let error_color = shell_state.color_scheme.error.clone();
    let print_error = move |msg: &str| {
        if colors_enabled {
            eprintln!("{}{}\x1b[0m", error_color, msg);
        } else {
            eprintln!("{}", msg);
        }
    };

    // If output_override is provided, use the old simple path for command substitution
    if output_override.is_some() {
        let mut output_writer = output_override.unwrap();
        let builtins = get_builtins();
        if let Some(builtin) = builtins
            .into_iter()
            .find(|b| b.names().contains(&cmd.args[0].as_str()))
        {
            return builtin.run(cmd, shell_state, &mut *output_writer);
        } else {
            return 1;
        }
    }

    // Handle redirections using FileDescriptorTable for proper POSIX compliance
    use crate::parser::Redirection;

    // Clone redirections to avoid borrow checker issues
    let redirections = cmd.redirections.clone();

    // First, expand all filenames in redirections (needs mutable borrow of shell_state)
    // Collect all filenames that need expansion
    let mut files_to_expand: Vec<String> = Vec::new();
    for redir in &redirections {
        match redir {
            Redirection::Input(file)
            | Redirection::Output(file)
            | Redirection::OutputClobber(file)
            | Redirection::Append(file)
            | Redirection::FdInput(_, file)
            | Redirection::FdOutput(_, file)
            | Redirection::FdAppend(_, file)
            | Redirection::FdInputOutput(_, file) => {
                files_to_expand.push(file.clone());
            }
            _ => {
                files_to_expand.push(String::new()); // Placeholder for non-file redirections
            }
        }
    }

    // Now expand all filenames (single mutable borrow)
    let mut expanded_files: Vec<String> = Vec::new();
    for f in &files_to_expand {
        if f.is_empty() {
            expanded_files.push(String::new());
        } else {
            expanded_files.push(crate::executor::expand_variables_in_string(f, shell_state));
        }
    }

    // Pair redirections with their expanded filenames
    let mut expanded_redirections: Vec<(Redirection, Option<String>)> = Vec::new();
    for (i, redir) in redirections.iter().enumerate() {
        let expanded_file = if expanded_files[i].is_empty() {
            None
        } else {
            Some(expanded_files[i].clone())
        };
        expanded_redirections.push((redir.clone(), expanded_file));
    }

    // Save all current file descriptors before applying redirections
    if let Err(e) = shell_state.fd_table.borrow_mut().save_all_fds() {
        print_error(&format!("Failed to save file descriptors: {}", e));
        return 1;
    }

    // Apply all redirections in left-to-right order (POSIX requirement)
    for (redir, expanded_file) in &expanded_redirections {
        let result = match redir {
            Redirection::Input(_) => {
                let file = expanded_file.as_ref().unwrap();
                shell_state.fd_table.borrow_mut().open_fd(
                    0, file, true,  // read
                    false, // write
                    false, // append
                    false, // truncate
                )
            }
            Redirection::Output(_) | Redirection::OutputClobber(_) => {
                let file = expanded_file.as_ref().unwrap();
                shell_state.fd_table.borrow_mut().open_fd(
                    1, file, false, // read
                    true,  // write
                    false, // append
                    true,  // truncate
                )
            }
            Redirection::Append(_) => {
                let file = expanded_file.as_ref().unwrap();
                shell_state.fd_table.borrow_mut().open_fd(
                    1, file, false, // read
                    true,  // write
                    true,  // append
                    false, // truncate
                )
            }
            Redirection::FdInput(fd, _) => {
                let file = expanded_file.as_ref().unwrap();
                shell_state.fd_table.borrow_mut().open_fd(
                    *fd, file, true,  // read
                    false, // write
                    false, // append
                    false, // truncate
                )
            }
            Redirection::FdOutput(fd, _) => {
                let file = expanded_file.as_ref().unwrap();
                shell_state.fd_table.borrow_mut().open_fd(
                    *fd, file, false, // read
                    true,  // write
                    false, // append
                    true,  // truncate
                )
            }
            Redirection::FdAppend(fd, _) => {
                let file = expanded_file.as_ref().unwrap();
                shell_state.fd_table.borrow_mut().open_fd(
                    *fd, file, false, // read
                    true,  // write
                    true,  // append
                    false, // truncate
                )
            }
            Redirection::FdDuplicate(target_fd, source_fd) => shell_state
                .fd_table
                .borrow_mut()
                .duplicate_fd(*source_fd, *target_fd),
            Redirection::FdClose(fd) => shell_state.fd_table.borrow_mut().close_fd(*fd),
            Redirection::FdInputOutput(fd, _) => {
                let file = expanded_file.as_ref().unwrap();
                shell_state.fd_table.borrow_mut().open_fd(
                    *fd, file, true,  // read
                    true,  // write
                    false, // append
                    false, // truncate
                )
            }
            // Here-documents and here-strings are handled differently for builtins
            // They don't modify the fd table directly
            Redirection::HereDoc(_, _) | Redirection::HereString(_) => Ok(()),
        };

        if let Err(e) = result {
            print_error(&format!("Redirection error: {}", e));
            // Restore file descriptors before returning
            let _ = shell_state.fd_table.borrow_mut().restore_all_fds();
            return 1;
        }
    }

    // Get output writer - try to get FD 1 from fd_table to respect redirections
    let mut output_writer: Box<dyn Write> = {
        let raw_fd = shell_state.fd_table.borrow().get_raw_fd(1);
        match raw_fd {
            Some(fd) => {
                // Duplicate the fd so we can take ownership in a File
                // (using unsafe libc call similar to how state.rs handles it)
                let dup_fd = unsafe { libc::dup(fd) };
                if dup_fd >= 0 {
                    let file = unsafe { std::fs::File::from_raw_fd(dup_fd) };
                    Box::new(ColoredWriter::new(file))
                } else {
                    // Duplication failed
                    let err = io::Error::last_os_error();
                    if err.raw_os_error() == Some(libc::EBADF) {
                        // EBADF means the FD is closed/invalid (e.g. parent closed stdout).
                        // In this case, we just run without output.
                        Box::new(BadFdWriter)
                    } else {
                        // Other errors (e.g. EMFILE) are fatal
                        print_error(&format!("Failed to duplicate stdout: {}", err));
                        let _ = shell_state.fd_table.borrow_mut().restore_all_fds();
                        return 1;
                    }
                }
            }
            None => {
                // FD 1 is closed. Do NOT fall back to stdout.
                Box::new(BadFdWriter)
            }
        }
    };

    // Execute the builtin command
    let builtins = get_builtins();
    let exit_code = if let Some(builtin) = builtins
        .into_iter()
        .find(|b| b.names().contains(&cmd.args[0].as_str()))
    {
        builtin.run(cmd, shell_state, &mut *output_writer)
    } else {
        1
    };

    // Restore all file descriptors after builtin execution
    if let Err(e) = shell_state.fd_table.borrow_mut().restore_all_fds() {
        print_error(&format!("Failed to restore file descriptors: {}", e));
        return 1;
    }

    exit_code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin() {
        assert!(is_builtin("cd"));
        assert!(is_builtin("pwd"));
        assert!(is_builtin("env"));
        assert!(is_builtin("exit"));
        assert!(is_builtin("help"));
        assert!(is_builtin("alias"));
        assert!(is_builtin("unalias"));
        assert!(is_builtin("test"));
        assert!(is_builtin("["));
        assert!(is_builtin("."));
        assert!(!is_builtin("ls"));
        assert!(!is_builtin("grep"));
        assert!(!is_builtin("echo"));
    }

    #[test]
    fn test_execute_builtin_unknown() {
        let cmd = ShellCommand {
            args: vec!["unknown".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_get_builtin_commands() {
        let commands = get_builtin_commands();
        assert!(commands.contains(&"cd".to_string()));
        assert!(commands.contains(&"pwd".to_string()));
        assert!(commands.contains(&"env".to_string()));
        assert!(commands.contains(&"exit".to_string()));
        assert!(commands.contains(&"help".to_string()));
        assert!(commands.contains(&"source".to_string()));
        assert!(commands.contains(&"export".to_string()));
        assert!(commands.contains(&"unset".to_string()));
        assert!(commands.contains(&"pushd".to_string()));
        assert!(commands.contains(&"popd".to_string()));
        assert!(commands.contains(&"dirs".to_string()));
        assert!(commands.contains(&"alias".to_string()));
        assert!(commands.contains(&"unalias".to_string()));
        assert!(commands.contains(&"test".to_string()));
        assert!(commands.contains(&"[".to_string()));
        assert!(commands.contains(&".".to_string()));
        assert!(commands.contains(&"set_colors".to_string()));
        assert!(commands.contains(&"set_color_scheme".to_string()));
        assert!(commands.contains(&"set_condensed".to_string()));
        assert!(commands.contains(&"return".to_string()));
        assert!(commands.contains(&"break".to_string()));
        assert!(commands.contains(&"continue".to_string()));
        assert!(commands.contains(&"set".to_string()));
        assert_eq!(commands.len(), 27);
    }
}