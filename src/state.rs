use super::parser::Ast;
use lazy_static::lazy_static;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::IsTerminal;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::process::Stdio;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

lazy_static! {
    /// Global queue for pending signal events
    /// Signals are enqueued by the signal handler thread and dequeued by the main thread
    pub static ref SIGNAL_QUEUE: Arc<Mutex<VecDeque<SignalEvent>>> =
        Arc::new(Mutex::new(VecDeque::new()));
}

/// Maximum number of signals to queue before dropping old ones
const MAX_SIGNAL_QUEUE_SIZE: usize = 100;

/// Represents a signal event that needs to be processed
#[derive(Debug, Clone)]
pub struct SignalEvent {
    /// Signal name (e.g., "INT", "TERM")
    pub signal_name: String,
    /// Signal number (e.g., 2, 15)
    pub signal_number: i32,
    /// When the signal was received
    pub timestamp: Instant,
}

impl SignalEvent {
    pub fn new(signal_name: String, signal_number: i32) -> Self {
        Self {
            signal_name,
            signal_number,
            timestamp: Instant::now(),
        }
    }
}

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
    ) -> Result<(), String> {
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

/// Shell option flags that control shell behavior
#[derive(Debug, Clone)]
pub struct ShellOptions {
    /// -e: Exit on command failure
    pub errexit: bool,
    
    /// -u: Treat unset variables as error
    pub nounset: bool,
    
    /// -x: Print commands before execution
    pub xtrace: bool,
    
    /// -v: Print input lines as read
    pub verbose: bool,
    
    /// -n: Read but don't execute commands
    pub noexec: bool,
    
    /// -f: Disable pathname expansion
    pub noglob: bool,
    
    /// -C: Prevent overwriting files with redirection
    pub noclobber: bool,
    
    /// -a: Auto-export all variables
    pub allexport: bool,
    
    /// -b: Notify of job completion immediately
    pub notify: bool,
    
    /// Ignore EOF (Ctrl+D) - not a standard POSIX option but commonly supported
    pub ignoreeof: bool,
    
    /// -m: Enable job control (monitor)
    pub monitor: bool,
}

impl Default for ShellOptions {
    /// Create a ShellOptions with all option flags set to false.
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::ShellOptions;
    /// let opts = ShellOptions::default();
    /// assert!(!opts.errexit && !opts.nounset && !opts.xtrace);
    /// ```
    fn default() -> Self {
        Self {
            errexit: false,
            nounset: false,
            xtrace: false,
            verbose: false,
            noexec: false,
            noglob: false,
            noclobber: false,
            allexport: false,
            notify: false,
            ignoreeof: false,
            monitor: false,
        }
    }
}

impl ShellOptions {
    /// Retrieve the value of a shell option by its short-name flag.
    ///
    /// Returns `Some(bool)` with the option's current value for recognized short names; `None` if the short name is not recognized.
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::ShellOptions;
    /// let opts = ShellOptions::default();
    /// assert_eq!(opts.get_by_short_name('e'), Some(false)); // errexit is false by default
    /// assert_eq!(opts.get_by_short_name('?'), None); // unknown short name
    /// ```
    #[allow(dead_code)]
    pub fn get_by_short_name(&self, name: char) -> Option<bool> {
        match name {
            'e' => Some(self.errexit),
            'u' => Some(self.nounset),
            'x' => Some(self.xtrace),
            'v' => Some(self.verbose),
            'n' => Some(self.noexec),
            'f' => Some(self.noglob),
            'C' => Some(self.noclobber),
            'a' => Some(self.allexport),
            'b' => Some(self.notify),
            'm' => Some(self.monitor),
            _ => None,
        }
    }
    
    /// Set a shell option identified by its single-character short name.
    ///
    /// Sets the option corresponding to `name` to `value`. Recognized short names:
    /// 'e' (errexit), 'u' (nounset), 'x' (xtrace), 'v' (verbose), 'n' (noexec),
    /// 'f' (noglob), 'C' (noclobber), 'a' (allexport), 'b' (notify), 'm' (monitor).
    ///
    /// # Arguments
    ///
    /// * `name` - single-character short option name.
    /// * `value` - true to enable the option, false to disable it.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or `Err(String)` if `name` is not a recognized option.
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::ShellOptions;
    /// let mut opts = ShellOptions::default();
    /// opts.set_by_short_name('e', true).unwrap();
    /// assert!(opts.errexit);
    /// ```
    pub fn set_by_short_name(&mut self, name: char, value: bool) -> Result<(), String> {
        match name {
            'e' => { self.errexit = value; Ok(()) },
            'u' => { self.nounset = value; Ok(()) },
            'x' => { self.xtrace = value; Ok(()) },
            'v' => { self.verbose = value; Ok(()) },
            'n' => { self.noexec = value; Ok(()) },
            'f' => { self.noglob = value; Ok(()) },
            'C' => { self.noclobber = value; Ok(()) },
            'a' => { self.allexport = value; Ok(()) },
            'b' => { self.notify = value; Ok(()) },
            'm' => { self.monitor = value; Ok(()) },
            _ => Err(format!("Invalid option: -{}", name)),
        }
    }
    
    /// Retrieve the value of a shell option by its long name.
    ///
    /// `name` is the option's full identifier (for example: "errexit", "nounset", "xtrace").
    ///
    /// # Returns
    ///
    /// `Some(true)` if the option is enabled, `Some(false)` if the option is disabled, or `None` if the name is not recognized.
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::ShellOptions;
    /// let mut opts = ShellOptions::default();
    /// opts.errexit = true;
    /// assert_eq!(opts.get_by_long_name("errexit"), Some(true));
    /// assert_eq!(opts.get_by_long_name("noglob"), Some(false));
    /// assert_eq!(opts.get_by_long_name("unknown"), None);
    /// ```
    #[allow(dead_code)]
    pub fn get_by_long_name(&self, name: &str) -> Option<bool> {
        match name {
            "errexit" => Some(self.errexit),
            "nounset" => Some(self.nounset),
            "xtrace" => Some(self.xtrace),
            "verbose" => Some(self.verbose),
            "noexec" => Some(self.noexec),
            "noglob" => Some(self.noglob),
            "noclobber" => Some(self.noclobber),
            "allexport" => Some(self.allexport),
            "notify" => Some(self.notify),
            "ignoreeof" => Some(self.ignoreeof),
            "monitor" => Some(self.monitor),
            _ => None,
        }
    }
    
    /// Set a shell option by its long name.
    ///
    /// Sets the specified long-form option (for example `"errexit"` or `"nounset"`) to the provided boolean value.
    /// Returns `Ok(())` if the option was recognized and set, or `Err(String)` if the name is not recognized.
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::ShellOptions;
    /// let mut opts = ShellOptions::default();
    /// opts.set_by_long_name("errexit", true).unwrap();
    /// assert!(opts.errexit);
    ///
    /// assert!(opts.set_by_long_name("nonexistent", true).is_err());
    /// ```
    pub fn set_by_long_name(&mut self, name: &str, value: bool) -> Result<(), String> {
        match name {
            "errexit" => { self.errexit = value; Ok(()) },
            "nounset" => { self.nounset = value; Ok(()) },
            "xtrace" => { self.xtrace = value; Ok(()) },
            "verbose" => { self.verbose = value; Ok(()) },
            "noexec" => { self.noexec = value; Ok(()) },
            "noglob" => { self.noglob = value; Ok(()) },
            "noclobber" => { self.noclobber = value; Ok(()) },
            "allexport" => { self.allexport = value; Ok(()) },
            "notify" => { self.notify = value; Ok(()) },
            "ignoreeof" => { self.ignoreeof = value; Ok(()) },
            "monitor" => { self.monitor = value; Ok(()) },
            _ => Err(format!("Invalid option: {}", name)),
        }
    }
    
    /// Lists all shell option names with their short-letter aliases and current values.
    ///
    /// Returns a vector of tuples `(long_name, short_name, value)` for every supported option.
    /// The `short_name` is `'\0'` when no short alias exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::ShellOptions;
    /// let opts = ShellOptions::default();
    /// let all = opts.get_all_options();
    /// assert!(all.iter().any(|(name, _, _)| *name == "errexit"));
    /// assert!(all.iter().any(|(name, short, _)| *name == "ignoreeof" && *short == '\0'));
    /// ```
    pub fn get_all_options(&self) -> Vec<(&'static str, char, bool)> {
        vec![
            ("allexport", 'a', self.allexport),
            ("notify", 'b', self.notify),
            ("noclobber", 'C', self.noclobber),
            ("errexit", 'e', self.errexit),
            ("noglob", 'f', self.noglob),
            ("monitor", 'm', self.monitor),
            ("noexec", 'n', self.noexec),
            ("nounset", 'u', self.nounset),
            ("verbose", 'v', self.verbose),
            ("xtrace", 'x', self.xtrace),
            ("ignoreeof", '\0', self.ignoreeof), // No short option
        ]
    }
}

#[derive(Debug, Clone)]
pub struct ColorScheme {
    /// ANSI color code for prompt
    pub prompt: String,
    /// ANSI color code for error messages
    pub error: String,
    /// ANSI color code for success messages
    pub success: String,
    /// ANSI color code for builtin command output
    pub builtin: String,
    /// ANSI color code for directory listings
    pub directory: String,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            prompt: "\x1b[32m".to_string(),    // Green
            error: "\x1b[31m".to_string(),     // Red
            success: "\x1b[32m".to_string(),   // Green
            builtin: "\x1b[36m".to_string(),   // Cyan
            directory: "\x1b[34m".to_string(), // Blue
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShellState {
    /// Shell variables (local to the shell session)
    pub variables: HashMap<String, String>,
    /// Which variables are exported to child processes
    pub exported: HashSet<String>,
    /// Last exit code ($?)
    pub last_exit_code: i32,
    /// Shell process ID ($$)
    pub shell_pid: u32,
    /// Script name or command ($0)
    pub script_name: String,
    /// Directory stack for pushd/popd
    pub dir_stack: Vec<String>,
    /// Command aliases
    pub aliases: HashMap<String, String>,
    /// Whether colors are enabled
    pub colors_enabled: bool,
    /// Current color scheme
    pub color_scheme: ColorScheme,
    /// Positional parameters ($1, $2, $3, ...)
    pub positional_params: Vec<String>,
    /// Function definitions
    pub functions: HashMap<String, Ast>,
    /// Local variable stack for function scoping
    pub local_vars: Vec<HashMap<String, String>>,
    /// Function call depth for local scope management
    pub function_depth: usize,
    /// Maximum allowed recursion depth
    pub max_recursion_depth: usize,
    /// Flag to indicate if we're currently returning from a function
    pub returning: bool,
    /// Return value when returning from a function
    pub return_value: Option<i32>,
    /// Loop nesting depth for break/continue
    pub loop_depth: usize,
    /// Flag to indicate if we're breaking out of a loop
    pub breaking: bool,
    /// Number of loop levels to break out of
    pub break_level: usize,
    /// Flag to indicate if we're continuing to next loop iteration
    pub continuing: bool,
    /// Number of loop levels to continue from
    pub continue_level: usize,
    /// Output capture buffer for command substitution
    pub capture_output: Option<Rc<RefCell<Vec<u8>>>>,
    /// Whether to use condensed cwd display in prompt
    pub condensed_cwd: bool,
    /// Signal trap handlers: maps signal name to command string
    pub trap_handlers: Arc<Mutex<HashMap<String, String>>>,
    /// Flag to track if EXIT trap has been executed
    pub exit_trap_executed: bool,
    /// Flag to indicate that the shell should exit
    pub exit_requested: bool,
    /// Exit code to use when exiting
    pub exit_code: i32,
    /// Flag to indicate pending signals need processing
    /// Set by signal handler, checked by executor
    #[allow(dead_code)]
    pub pending_signals: bool,
    /// Pending here-document content from script execution
    pub pending_heredoc_content: Option<String>,
    /// Interactive mode heredoc collection state
    pub collecting_heredoc: Option<(String, String, String)>, // (command_line, delimiter, collected_content)
    /// File descriptor table for managing open file descriptors
    pub fd_table: Rc<RefCell<FileDescriptorTable>>,
    /// Current subshell nesting depth (for recursion limit)
    pub subshell_depth: usize,
    /// Override for stdin (used for pipeline subshells to avoid process-global fd manipulation)
    pub stdin_override: Option<RawFd>,
    /// Shell option flags (set builtin)
    pub options: ShellOptions,
    /// Context tracking for errexit option - true when executing commands in if/while/until conditions
    pub in_condition: bool,
    /// Context tracking for errexit option - true when executing commands in && or || chains
    pub in_logical_chain: bool,
    /// Context tracking for errexit option - true when executing negated commands (!)
    pub in_negation: bool,
    /// Track if the last command executed was a negation (to skip errexit check on inverted code)
    pub last_was_negation: bool,
}

impl ShellState {
    /// Creates a new ShellState initialized with sensible defaults and environment-derived settings.
    ///
    /// The returned state initializes runtime fields (variables, exported, aliases, positional params, function/local scopes, FD table, traps, and control flags) and derives display preferences from environment:
    /// - `colors_enabled` is determined by `NO_COLOR`, `RUSH_COLORS`, and whether stdout is a terminal.
    /// - `condensed_cwd` is determined by `RUSH_CONDENSED` (defaults to `true`).
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::ShellState;
    /// let state = ShellState::new();
    /// // basic invariants
    /// assert_eq!(state.last_exit_code, 0);
    /// assert!(state.shell_pid != 0);
    /// ```
    pub fn new() -> Self {
        let shell_pid = std::process::id();

        // Check NO_COLOR environment variable (respects standard)
        let no_color = env::var("NO_COLOR").is_ok();

        // Check RUSH_COLORS environment variable for explicit control
        let rush_colors = env::var("RUSH_COLORS")
            .map(|v| v.to_lowercase())
            .unwrap_or_else(|_| "auto".to_string());

        let colors_enabled = match rush_colors.as_str() {
            "1" | "true" | "on" | "enable" => !no_color && std::io::stdout().is_terminal(),
            "0" | "false" | "off" | "disable" => false,
            "auto" => !no_color && std::io::stdout().is_terminal(),
            _ => !no_color && std::io::stdout().is_terminal(),
        };

        // Check RUSH_CONDENSED environment variable for cwd display preference
        let rush_condensed = env::var("RUSH_CONDENSED")
            .map(|v| v.to_lowercase())
            .unwrap_or_else(|_| "true".to_string());

        let condensed_cwd = match rush_condensed.as_str() {
            "1" | "true" | "on" | "enable" => true,
            "0" | "false" | "off" | "disable" => false,
            _ => true, // Default to condensed for backward compatibility
        };

        Self {
            variables: HashMap::new(),
            exported: HashSet::new(),
            last_exit_code: 0,
            shell_pid,
            script_name: "rush".to_string(),
            dir_stack: Vec::new(),
            aliases: HashMap::new(),
            colors_enabled,
            color_scheme: ColorScheme::default(),
            positional_params: Vec::new(),
            functions: HashMap::new(),
            local_vars: Vec::new(),
            function_depth: 0,
            max_recursion_depth: 500, // Default recursion limit (reduced to avoid Rust stack overflow)
            returning: false,
            return_value: None,
            loop_depth: 0,
            breaking: false,
            break_level: 0,
            continuing: false,
            continue_level: 0,
            capture_output: None,
            condensed_cwd,
            trap_handlers: Arc::new(Mutex::new(HashMap::new())),
            exit_trap_executed: false,
            exit_requested: false,
            exit_code: 0,
            pending_signals: false,
            pending_heredoc_content: None,
            collecting_heredoc: None,
            fd_table: Rc::new(RefCell::new(FileDescriptorTable::new())),
            subshell_depth: 0,
            stdin_override: None,
            options: ShellOptions::default(),
            in_condition: false,
            in_logical_chain: false,
            in_negation: false,
            last_was_negation: false,
        }
    }

    /// Get a variable value, checking local scopes first, then shell variables, then environment
    pub fn get_var(&self, name: &str) -> Option<String> {
        // Handle special variables (these are never local)
        match name {
            "?" => Some(self.last_exit_code.to_string()),
            "$" => Some(self.shell_pid.to_string()),
            "0" => Some(self.script_name.clone()),
            "*" => {
                // $* - all positional parameters as single string (space-separated)
                if self.positional_params.is_empty() {
                    Some("".to_string())
                } else {
                    Some(self.positional_params.join(" "))
                }
            }
            "@" => {
                // $@ - all positional parameters as separate words (but returns as single string for compatibility)
                if self.positional_params.is_empty() {
                    Some("".to_string())
                } else {
                    Some(self.positional_params.join(" "))
                }
            }
            "#" => Some(self.positional_params.len().to_string()),
            _ => {
                // Handle positional parameters $1, $2, $3, etc. (these are never local)
                if let Ok(index) = name.parse::<usize>()
                    && index > 0
                    && index <= self.positional_params.len()
                {
                    return Some(self.positional_params[index - 1].clone());
                }

                // Check local scopes first, then shell variables, then environment
                // Search local scopes from innermost to outermost
                for scope in self.local_vars.iter().rev() {
                    if let Some(value) = scope.get(name) {
                        return Some(value.clone());
                    }
                }

                // Check shell variables
                if let Some(value) = self.variables.get(name) {
                    Some(value.clone())
                } else {
                    // Fall back to environment variables
                    env::var(name).ok()
                }
            }
        }
    }

    /// Set a shell variable (updates local scope if variable exists there, otherwise sets globally)
    pub fn set_var(&mut self, name: &str, value: String) {
        // Check if this variable exists in any local scope
        // If it does, update it there instead of setting globally
        for scope in self.local_vars.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return;
            }
        }

        // Variable doesn't exist in local scopes, set it globally
        self.variables.insert(name.to_string(), value);
    }

    /// Remove a shell variable
    pub fn unset_var(&mut self, name: &str) {
        self.variables.remove(name);
        self.exported.remove(name);
    }

    /// Mark a variable as exported
    pub fn export_var(&mut self, name: &str) {
        if self.variables.contains_key(name) {
            self.exported.insert(name.to_string());
        }
    }

    /// Set and export a variable
    pub fn set_exported_var(&mut self, name: &str, value: String) {
        self.set_var(name, value);
        self.export_var(name);
    }

    /// Get all environment variables for child processes (exported + inherited)
    pub fn get_env_for_child(&self) -> HashMap<String, String> {
        let mut child_env = HashMap::new();

        // Add all current environment variables
        for (key, value) in env::vars() {
            child_env.insert(key, value);
        }

        // Override with exported shell variables
        for var_name in &self.exported {
            if let Some(value) = self.variables.get(var_name) {
                child_env.insert(var_name.clone(), value.clone());
            }
        }

        child_env
    }

    /// Update the last exit code
    pub fn set_last_exit_code(&mut self, code: i32) {
        self.last_exit_code = code;
    }

    /// Set the script name ($0)
    pub fn set_script_name(&mut self, name: &str) {
        self.script_name = name.to_string();
    }

    /// Get the condensed current working directory for the prompt
    pub fn get_condensed_cwd(&self) -> String {
        match env::current_dir() {
            Ok(path) => {
                let path_str = path.to_string_lossy();
                let components: Vec<&str> = path_str.split('/').collect();
                if components.is_empty() || (components.len() == 1 && components[0].is_empty()) {
                    return "/".to_string();
                }
                let mut result = String::new();
                for (i, comp) in components.iter().enumerate() {
                    if comp.is_empty() {
                        continue; // skip leading empty component
                    }
                    if i == components.len() - 1 {
                        result.push('/');
                        result.push_str(comp);
                    } else {
                        result.push('/');
                        if let Some(first) = comp.chars().next() {
                            result.push(first);
                        }
                    }
                }
                if result.is_empty() {
                    "/".to_string()
                } else {
                    result
                }
            }
            Err(_) => "/?".to_string(), // fallback if can't get cwd
        }
    }

    /// Get the full current working directory for the prompt
    pub fn get_full_cwd(&self) -> String {
        match env::current_dir() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => "/?".to_string(), // fallback if can't get cwd
        }
    }

    /// Get the user@hostname string for the prompt
    pub fn get_user_hostname(&self) -> String {
        let user = env::var("USER").unwrap_or_else(|_| "user".to_string());

        // First try to get hostname from HOSTNAME environment variable
        if let Ok(hostname) = env::var("HOSTNAME")
            && !hostname.trim().is_empty()
        {
            return format!("{}@{}", user, hostname);
        }

        // If HOSTNAME is not set or empty, try the hostname command
        let hostname = match std::process::Command::new("hostname").output() {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            }
            _ => "hostname".to_string(), // Last resort fallback
        };

        // Set the HOSTNAME environment variable for future use
        if hostname != "hostname" {
            unsafe {
                env::set_var("HOSTNAME", &hostname);
            }
        }

        format!("{}@{}", user, hostname)
    }

    /// Get the full prompt string
    pub fn get_prompt(&self) -> String {
        let user = env::var("USER").unwrap_or_else(|_| "user".to_string());
        let prompt_char = if user == "root" { "#" } else { "$" };
        let cwd = if self.condensed_cwd {
            self.get_condensed_cwd()
        } else {
            self.get_full_cwd()
        };
        format!("{}:{} {} ", self.get_user_hostname(), cwd, prompt_char)
    }

    /// Set an alias
    pub fn set_alias(&mut self, name: &str, value: String) {
        self.aliases.insert(name.to_string(), value);
    }

    /// Get an alias value
    pub fn get_alias(&self, name: &str) -> Option<&String> {
        self.aliases.get(name)
    }

    /// Remove an alias
    pub fn remove_alias(&mut self, name: &str) {
        self.aliases.remove(name);
    }

    /// Get all aliases
    pub fn get_all_aliases(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    /// Set positional parameters
    pub fn set_positional_params(&mut self, params: Vec<String>) {
        self.positional_params = params;
    }

    /// Get positional parameters
    #[allow(dead_code)]
    pub fn get_positional_params(&self) -> &[String] {
        &self.positional_params
    }

    /// Shift positional parameters (remove first n parameters)
    pub fn shift_positional_params(&mut self, count: usize) {
        if count > 0 {
            for _ in 0..count {
                if !self.positional_params.is_empty() {
                    self.positional_params.remove(0);
                }
            }
        }
    }

    /// Add a positional parameter at the end
    #[allow(dead_code)]
    pub fn push_positional_param(&mut self, param: String) {
        self.positional_params.push(param);
    }

    /// Define a function
    pub fn define_function(&mut self, name: String, body: Ast) {
        self.functions.insert(name, body);
    }

    /// Get a function definition
    pub fn get_function(&self, name: &str) -> Option<&Ast> {
        self.functions.get(name)
    }

    /// Remove a function definition
    #[allow(dead_code)]
    pub fn remove_function(&mut self, name: &str) {
        self.functions.remove(name);
    }

    /// Get all function names
    #[allow(dead_code)]
    pub fn get_function_names(&self) -> Vec<&String> {
        self.functions.keys().collect()
    }

    /// Push a new local variable scope
    pub fn push_local_scope(&mut self) {
        self.local_vars.push(HashMap::new());
    }

    /// Pop the current local variable scope
    pub fn pop_local_scope(&mut self) {
        if !self.local_vars.is_empty() {
            self.local_vars.pop();
        }
    }

    /// Set a local variable in the current scope
    pub fn set_local_var(&mut self, name: &str, value: String) {
        if let Some(current_scope) = self.local_vars.last_mut() {
            current_scope.insert(name.to_string(), value);
        } else {
            // If no local scope exists, set as global variable
            self.set_var(name, value);
        }
    }

    /// Enter a function context (push local scope if needed)
    pub fn enter_function(&mut self) {
        self.function_depth += 1;
        if self.function_depth > self.local_vars.len() {
            self.push_local_scope();
        }
    }

    /// Exit a function context (pop local scope if needed)
    pub fn exit_function(&mut self) {
        if self.function_depth > 0 {
            self.function_depth -= 1;
            if self.function_depth == self.local_vars.len() - 1 {
                self.pop_local_scope();
            }
        }
    }

    /// Set return state for function returns
    pub fn set_return(&mut self, value: i32) {
        self.returning = true;
        self.return_value = Some(value);
    }

    /// Clear return state
    pub fn clear_return(&mut self) {
        self.returning = false;
        self.return_value = None;
    }

    /// Check if currently returning
    pub fn is_returning(&self) -> bool {
        self.returning
    }

    /// Get return value if returning
    pub fn get_return_value(&self) -> Option<i32> {
        self.return_value
    }

    /// Enter a loop context (increment loop depth)
    pub fn enter_loop(&mut self) {
        self.loop_depth += 1;
    }

    /// Exit a loop context (decrement loop depth)
    pub fn exit_loop(&mut self) {
        if self.loop_depth > 0 {
            self.loop_depth -= 1;
        }
    }

    /// Set break state for loop control
    pub fn set_break(&mut self, level: usize) {
        self.breaking = true;
        self.break_level = level;
    }

    /// Clear break state
    pub fn clear_break(&mut self) {
        self.breaking = false;
        self.break_level = 0;
    }

    /// Check if currently breaking
    pub fn is_breaking(&self) -> bool {
        self.breaking
    }

    /// Get break level
    pub fn get_break_level(&self) -> usize {
        self.break_level
    }

    /// Decrement break level (when exiting a loop level)
    pub fn decrement_break_level(&mut self) {
        if self.break_level > 0 {
            self.break_level -= 1;
        }
        if self.break_level == 0 {
            self.breaking = false;
        }
    }

    /// Set continue state for loop control
    pub fn set_continue(&mut self, level: usize) {
        self.continuing = true;
        self.continue_level = level;
    }

    /// Clear continue state
    pub fn clear_continue(&mut self) {
        self.continuing = false;
        self.continue_level = 0;
    }

    /// Check if currently continuing
    pub fn is_continuing(&self) -> bool {
        self.continuing
    }

    /// Get continue level
    pub fn get_continue_level(&self) -> usize {
        self.continue_level
    }

    /// Decrement continue level (when exiting a loop level)
    pub fn decrement_continue_level(&mut self) {
        if self.continue_level > 0 {
            self.continue_level -= 1;
        }
        if self.continue_level == 0 {
            self.continuing = false;
        }
    }

    /// Set a trap handler for a signal
    pub fn set_trap(&mut self, signal: &str, command: String) {
        if let Ok(mut handlers) = self.trap_handlers.lock() {
            handlers.insert(signal.to_uppercase(), command);
        }
    }

    /// Get a trap handler for a signal
    pub fn get_trap(&self, signal: &str) -> Option<String> {
        if let Ok(handlers) = self.trap_handlers.lock() {
            handlers.get(&signal.to_uppercase()).cloned()
        } else {
            None
        }
    }

    /// Remove a trap handler for a signal
    pub fn remove_trap(&mut self, signal: &str) {
        if let Ok(mut handlers) = self.trap_handlers.lock() {
            handlers.remove(&signal.to_uppercase());
        }
    }

    /// Get all trap handlers
    pub fn get_all_traps(&self) -> HashMap<String, String> {
        if let Ok(handlers) = self.trap_handlers.lock() {
            handlers.clone()
        } else {
            HashMap::new()
        }
    }

    /// Clear all trap handlers
    #[allow(dead_code)]
    pub fn clear_traps(&mut self) {
        if let Ok(mut handlers) = self.trap_handlers.lock() {
            handlers.clear();
        }
    }
}

/// Enqueue a signal event for later processing
/// If the queue is full, the oldest event is dropped
pub fn enqueue_signal(signal_name: &str, signal_number: i32) {
    if let Ok(mut queue) = SIGNAL_QUEUE.lock() {
        // If queue is full, remove oldest event
        if queue.len() >= MAX_SIGNAL_QUEUE_SIZE {
            queue.pop_front();
            eprintln!("Warning: Signal queue overflow, dropping oldest signal");
        }

        queue.push_back(SignalEvent::new(signal_name.to_string(), signal_number));
    }
}

/// Process all pending signals in the queue
/// This should be called at safe points during command execution
pub fn process_pending_signals(shell_state: &mut ShellState) {
    // Try to lock the queue with a timeout to avoid blocking
    if let Ok(mut queue) = SIGNAL_QUEUE.lock() {
        // Process all pending signals
        while let Some(signal_event) = queue.pop_front() {
            // Check if a trap is set for this signal
            if let Some(trap_cmd) = shell_state.get_trap(&signal_event.signal_name)
                && !trap_cmd.is_empty()
            {
                // Display signal information for debugging/tracking
                if shell_state.colors_enabled {
                    eprintln!(
                        "{}Signal {} (signal {}) received at {:?}\x1b[0m",
                        shell_state.color_scheme.builtin,
                        signal_event.signal_name,
                        signal_event.signal_number,
                        signal_event.timestamp
                    );
                } else {
                    eprintln!(
                        "Signal {} (signal {}) received at {:?}",
                        signal_event.signal_name,
                        signal_event.signal_number,
                        signal_event.timestamp
                    );
                }

                // Execute the trap handler
                // Note: This preserves the exit code as per POSIX requirements
                crate::executor::execute_trap_handler(&trap_cmd, shell_state);
            }
        }
    }
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize tests that create temporary files
    static FILE_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_shell_state_basic() {
        let mut state = ShellState::new();
        state.set_var("TEST_VAR", "test_value".to_string());
        assert_eq!(state.get_var("TEST_VAR"), Some("test_value".to_string()));
    }

    #[test]
    fn test_special_variables() {
        let mut state = ShellState::new();
        state.set_last_exit_code(42);
        state.set_script_name("test_script");

        assert_eq!(state.get_var("?"), Some("42".to_string()));
        assert_eq!(state.get_var("$"), Some(state.shell_pid.to_string()));
        assert_eq!(state.get_var("0"), Some("test_script".to_string()));
    }

    #[test]
    fn test_export_variable() {
        let mut state = ShellState::new();
        state.set_var("EXPORT_VAR", "export_value".to_string());
        state.export_var("EXPORT_VAR");

        let child_env = state.get_env_for_child();
        assert_eq!(
            child_env.get("EXPORT_VAR"),
            Some(&"export_value".to_string())
        );
    }

    #[test]
    fn test_unset_variable() {
        let mut state = ShellState::new();
        state.set_var("UNSET_VAR", "value".to_string());
        state.export_var("UNSET_VAR");

        assert!(state.variables.contains_key("UNSET_VAR"));
        assert!(state.exported.contains("UNSET_VAR"));

        state.unset_var("UNSET_VAR");

        assert!(!state.variables.contains_key("UNSET_VAR"));
        assert!(!state.exported.contains("UNSET_VAR"));
    }

    #[test]
    fn test_get_user_hostname() {
        let state = ShellState::new();
        let user_hostname = state.get_user_hostname();
        // Should contain @ since it's user@hostname format
        assert!(user_hostname.contains('@'));
    }

    #[test]
    fn test_get_prompt() {
        let state = ShellState::new();
        let prompt = state.get_prompt();
        // Should end with $ and contain @
        assert!(prompt.ends_with(" $ "));
        assert!(prompt.contains('@'));
    }

    #[test]
    fn test_positional_parameters() {
        let mut state = ShellState::new();
        state.set_positional_params(vec![
            "arg1".to_string(),
            "arg2".to_string(),
            "arg3".to_string(),
        ]);

        assert_eq!(state.get_var("1"), Some("arg1".to_string()));
        assert_eq!(state.get_var("2"), Some("arg2".to_string()));
        assert_eq!(state.get_var("3"), Some("arg3".to_string()));
        assert_eq!(state.get_var("4"), None);
        assert_eq!(state.get_var("#"), Some("3".to_string()));
        assert_eq!(state.get_var("*"), Some("arg1 arg2 arg3".to_string()));
        assert_eq!(state.get_var("@"), Some("arg1 arg2 arg3".to_string()));
    }

    #[test]
    fn test_positional_parameters_empty() {
        let mut state = ShellState::new();
        state.set_positional_params(vec![]);

        assert_eq!(state.get_var("1"), None);
        assert_eq!(state.get_var("#"), Some("0".to_string()));
        assert_eq!(state.get_var("*"), Some("".to_string()));
        assert_eq!(state.get_var("@"), Some("".to_string()));
    }

    #[test]
    fn test_shift_positional_params() {
        let mut state = ShellState::new();
        state.set_positional_params(vec![
            "arg1".to_string(),
            "arg2".to_string(),
            "arg3".to_string(),
        ]);

        assert_eq!(state.get_var("1"), Some("arg1".to_string()));
        assert_eq!(state.get_var("2"), Some("arg2".to_string()));
        assert_eq!(state.get_var("3"), Some("arg3".to_string()));

        state.shift_positional_params(1);

        assert_eq!(state.get_var("1"), Some("arg2".to_string()));
        assert_eq!(state.get_var("2"), Some("arg3".to_string()));
        assert_eq!(state.get_var("3"), None);
        assert_eq!(state.get_var("#"), Some("2".to_string()));

        state.shift_positional_params(2);

        assert_eq!(state.get_var("1"), None);
        assert_eq!(state.get_var("#"), Some("0".to_string()));
    }

    #[test]
    fn test_push_positional_param() {
        let mut state = ShellState::new();
        state.set_positional_params(vec!["arg1".to_string()]);

        assert_eq!(state.get_var("1"), Some("arg1".to_string()));
        assert_eq!(state.get_var("#"), Some("1".to_string()));

        state.push_positional_param("arg2".to_string());

        assert_eq!(state.get_var("1"), Some("arg1".to_string()));
        assert_eq!(state.get_var("2"), Some("arg2".to_string()));
        assert_eq!(state.get_var("#"), Some("2".to_string()));
    }

    #[test]
    fn test_local_variable_scoping() {
        let mut state = ShellState::new();

        // Set a global variable
        state.set_var("global_var", "global_value".to_string());
        assert_eq!(
            state.get_var("global_var"),
            Some("global_value".to_string())
        );

        // Push local scope
        state.push_local_scope();

        // Set a local variable with the same name
        state.set_local_var("global_var", "local_value".to_string());
        assert_eq!(state.get_var("global_var"), Some("local_value".to_string()));

        // Set another local variable
        state.set_local_var("local_var", "local_only".to_string());
        assert_eq!(state.get_var("local_var"), Some("local_only".to_string()));

        // Pop local scope
        state.pop_local_scope();

        // Should be back to global variable
        assert_eq!(
            state.get_var("global_var"),
            Some("global_value".to_string())
        );
        assert_eq!(state.get_var("local_var"), None);
    }

    #[test]
    fn test_nested_local_scopes() {
        let mut state = ShellState::new();

        // Set global variable
        state.set_var("test_var", "global".to_string());

        // Push first local scope
        state.push_local_scope();
        state.set_local_var("test_var", "level1".to_string());
        assert_eq!(state.get_var("test_var"), Some("level1".to_string()));

        // Push second local scope
        state.push_local_scope();
        state.set_local_var("test_var", "level2".to_string());
        assert_eq!(state.get_var("test_var"), Some("level2".to_string()));

        // Pop second scope
        state.pop_local_scope();
        assert_eq!(state.get_var("test_var"), Some("level1".to_string()));

        // Pop first scope
        state.pop_local_scope();
        assert_eq!(state.get_var("test_var"), Some("global".to_string()));
    }

    #[test]
    fn test_variable_set_in_local_scope() {
        let mut state = ShellState::new();

        // No local scope initially
        state.set_var("test_var", "global".to_string());
        assert_eq!(state.get_var("test_var"), Some("global".to_string()));

        // Push local scope and set local variable
        state.push_local_scope();
        state.set_local_var("test_var", "local".to_string());
        assert_eq!(state.get_var("test_var"), Some("local".to_string()));

        // Pop scope
        state.pop_local_scope();
        assert_eq!(state.get_var("test_var"), Some("global".to_string()));
    }

    #[test]
    fn test_condensed_cwd_environment_variable() {
        // Test default behavior (should be true for backward compatibility)
        let state = ShellState::new();
        assert!(state.condensed_cwd);

        // Test explicit true
        unsafe {
            env::set_var("RUSH_CONDENSED", "true");
        }
        let state = ShellState::new();
        assert!(state.condensed_cwd);

        // Test explicit false
        unsafe {
            env::set_var("RUSH_CONDENSED", "false");
        }
        let state = ShellState::new();
        assert!(!state.condensed_cwd);

        // Clean up
        unsafe {
            env::remove_var("RUSH_CONDENSED");
        }
    }

    #[test]
    fn test_get_full_cwd() {
        let state = ShellState::new();
        let full_cwd = state.get_full_cwd();
        assert!(!full_cwd.is_empty());
        // Should contain path separators (either / or \ depending on platform)
        assert!(full_cwd.contains('/') || full_cwd.contains('\\'));
    }

    #[test]
    fn test_prompt_with_condensed_setting() {
        let mut state = ShellState::new();

        // Test with condensed enabled (default)
        assert!(state.condensed_cwd);
        let prompt_condensed = state.get_prompt();
        assert!(prompt_condensed.contains('@'));

        // Test with condensed disabled
        state.condensed_cwd = false;
        let prompt_full = state.get_prompt();
        assert!(prompt_full.contains('@'));

        // Both should end with "$ " (or "# " for root)
        assert!(prompt_condensed.ends_with("$ ") || prompt_condensed.ends_with("# "));
        assert!(prompt_full.ends_with("$ ") || prompt_full.ends_with("# "));
    }

    // File Descriptor Table Tests

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
        let result = fd_table.open_fd(3, temp_file, true, false, false, false);
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
        let result = fd_table.open_fd(4, temp_file, false, true, false, true);
        assert!(result.is_ok());
        assert!(fd_table.is_open(4));

        // Clean up
        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_fd_table_invalid_fd_number() {
        let mut fd_table = FileDescriptorTable::new();

        // Test invalid fd numbers
        let result = fd_table.open_fd(-1, "/tmp/test.txt", true, false, false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid file descriptor"));

        let result = fd_table.open_fd(1025, "/tmp/test.txt", true, false, false, false);
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
            .open_fd(3, temp_file, true, false, false, false)
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
            .open_fd(3, temp_file, true, false, false, false)
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
            .open_fd(3, temp_file, true, false, false, false)
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
            .open_fd(50, &temp_file1, true, false, false, false)
            .unwrap();
        fd_table
            .open_fd(51, &temp_file2, true, false, false, false)
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
            .open_fd(50, temp_file, true, false, false, false)
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
            .open_fd(3, temp_file, true, false, false, false)
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
            .open_fd(3, temp_file1, true, false, false, false)
            .unwrap();
        assert!(fd_table.is_open(3));

        // Duplicate fd 3 to fd 4
        fd_table.duplicate_fd(3, 4).unwrap();
        assert!(fd_table.is_open(4));

        // Open another file on fd 5
        fd_table
            .open_fd(5, temp_file2, true, false, false, false)
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
        let state = ShellState::new();
        let fd_table = state.fd_table.borrow();
        assert!(!fd_table.is_open(3));
    }

    #[test]
    fn test_shell_state_fd_table_operations() {
        let state = ShellState::new();

        // Create a temporary file
        let temp_file = "/tmp/rush_test_state_fd.txt";
        std::fs::write(temp_file, "test content").unwrap();

        // Open file through shell state's fd table
        {
            let mut fd_table = state.fd_table.borrow_mut();
            fd_table
                .open_fd(3, temp_file, true, false, false, false)
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

    // ShellOptions Tests

    #[test]
    fn test_shell_options_default() {
        let options = ShellOptions::default();
        assert!(!options.errexit);
        assert!(!options.nounset);
        assert!(!options.xtrace);
        assert!(!options.verbose);
        assert!(!options.noexec);
        assert!(!options.noglob);
        assert!(!options.noclobber);
        assert!(!options.allexport);
        assert!(!options.notify);
        assert!(!options.ignoreeof);
        assert!(!options.monitor);
    }

    #[test]
    fn test_shell_options_get_by_short_name() {
        let mut options = ShellOptions::default();
        options.errexit = true;
        options.nounset = true;

        assert_eq!(options.get_by_short_name('e'), Some(true));
        assert_eq!(options.get_by_short_name('u'), Some(true));
        assert_eq!(options.get_by_short_name('x'), Some(false));
        assert_eq!(options.get_by_short_name('Z'), None);
    }

    #[test]
    fn test_shell_options_set_by_short_name() {
        let mut options = ShellOptions::default();

        assert!(options.set_by_short_name('e', true).is_ok());
        assert!(options.errexit);

        assert!(options.set_by_short_name('u', true).is_ok());
        assert!(options.nounset);

        assert!(options.set_by_short_name('x', true).is_ok());
        assert!(options.xtrace);

        assert!(options.set_by_short_name('e', false).is_ok());
        assert!(!options.errexit);

        // Invalid option
        assert!(options.set_by_short_name('Z', true).is_err());
    }

    #[test]
    fn test_shell_options_get_by_long_name() {
        let mut options = ShellOptions::default();
        options.errexit = true;
        options.nounset = true;

        assert_eq!(options.get_by_long_name("errexit"), Some(true));
        assert_eq!(options.get_by_long_name("nounset"), Some(true));
        assert_eq!(options.get_by_long_name("xtrace"), Some(false));
        assert_eq!(options.get_by_long_name("invalid"), None);
    }

    #[test]
    fn test_shell_options_set_by_long_name() {
        let mut options = ShellOptions::default();

        assert!(options.set_by_long_name("errexit", true).is_ok());
        assert!(options.errexit);

        assert!(options.set_by_long_name("nounset", true).is_ok());
        assert!(options.nounset);

        assert!(options.set_by_long_name("xtrace", true).is_ok());
        assert!(options.xtrace);

        assert!(options.set_by_long_name("errexit", false).is_ok());
        assert!(!options.errexit);

        // Invalid option
        assert!(options.set_by_long_name("invalid", true).is_err());
    }

    #[test]
    fn test_shell_options_all_short_options() {
        let mut options = ShellOptions::default();

        // Test all valid short options
        let short_opts = vec!['e', 'u', 'x', 'v', 'n', 'f', 'C', 'a', 'b', 'm'];
        for opt in short_opts {
            assert!(options.set_by_short_name(opt, true).is_ok());
            assert_eq!(options.get_by_short_name(opt), Some(true));
            assert!(options.set_by_short_name(opt, false).is_ok());
            assert_eq!(options.get_by_short_name(opt), Some(false));
        }
    }

    #[test]
    fn test_shell_options_all_long_options() {
        let mut options = ShellOptions::default();

        // Test all valid long options
        let long_opts = vec![
            "errexit", "nounset", "xtrace", "verbose", "noexec", "noglob", "noclobber",
            "allexport", "notify", "ignoreeof", "monitor",
        ];
        for opt in long_opts {
            assert!(options.set_by_long_name(opt, true).is_ok());
            assert_eq!(options.get_by_long_name(opt), Some(true));
            assert!(options.set_by_long_name(opt, false).is_ok());
            assert_eq!(options.get_by_long_name(opt), Some(false));
        }
    }

    #[test]
    fn test_shell_options_get_all_options() {
        let mut options = ShellOptions::default();
        options.errexit = true;
        options.xtrace = true;

        let all_options = options.get_all_options();
        
        // Should have 11 options
        assert_eq!(all_options.len(), 11);

        // Find errexit and verify it's on
        let errexit_opt = all_options.iter().find(|(name, _, _)| *name == "errexit");
        assert!(errexit_opt.is_some());
        assert_eq!(errexit_opt.unwrap().2, true);

        // Find xtrace and verify it's on
        let xtrace_opt = all_options.iter().find(|(name, _, _)| *name == "xtrace");
        assert!(xtrace_opt.is_some());
        assert_eq!(xtrace_opt.unwrap().2, true);

        // Find nounset and verify it's off
        let nounset_opt = all_options.iter().find(|(name, _, _)| *name == "nounset");
        assert!(nounset_opt.is_some());
        assert_eq!(nounset_opt.unwrap().2, false);
    }

    #[test]
    fn test_shell_state_has_options() {
        let state = ShellState::new();
        assert!(!state.options.errexit);
        assert!(!state.options.nounset);
        assert!(!state.options.xtrace);
    }

    #[test]
    fn test_shell_state_options_modification() {
        let mut state = ShellState::new();
        
        state.options.errexit = true;
        assert!(state.options.errexit);
        
        state.options.set_by_short_name('u', true).unwrap();
        assert!(state.options.nounset);
        
        state.options.set_by_long_name("xtrace", true).unwrap();
        assert!(state.options.xtrace);
    }

    #[test]
    fn test_shell_options_error_messages() {
        let mut options = ShellOptions::default();

        let result = options.set_by_short_name('Z', true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid option: -Z"));

        let result = options.set_by_long_name("invalid_option", true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid option: invalid_option"));
    }

    #[test]
    fn test_shell_options_case_sensitivity() {
        let mut options = ShellOptions::default();

        // 'C' is valid (noclobber), 'c' is not
        assert!(options.set_by_short_name('C', true).is_ok());
        assert!(options.noclobber);
        assert!(options.set_by_short_name('c', true).is_err());
    }
}