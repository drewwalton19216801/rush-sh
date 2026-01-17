// File descriptor table module
pub mod fd_table;

// Shell options module
pub mod options;

// Signal management module
pub mod signals;

// Job management module
pub mod jobs;

// Re-export types for backward compatibility
pub use fd_table::FileDescriptorTable;
pub use options::ShellOptions;
pub use signals::{enqueue_signal, process_pending_signals, check_background_jobs};
pub use jobs::{Job, JobStatus, JobTable};

use super::parser::Ast;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env;
use std::io::IsTerminal;
use std::os::unix::io::RawFd;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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
    /// Current line number in script execution (for $LINENO)
    pub current_line_number: usize,
    /// Stack of line numbers for function calls (to restore after function returns)
    pub line_number_stack: Vec<usize>,
    /// Job table for managing background jobs
    pub job_table: Rc<RefCell<JobTable>>,
    /// Whether the shell is running in interactive mode
    pub interactive: bool,
    /// Terminal file descriptor for job control (None if not interactive)
    pub terminal_fd: Option<RawFd>,
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
            current_line_number: 1,
            line_number_stack: Vec::new(),
            job_table: Rc::new(RefCell::new(JobTable::new())),
            interactive: std::io::stdin().is_terminal(),
            terminal_fd: if std::io::stdin().is_terminal() {
                Some(libc::STDIN_FILENO)
            } else {
                None
            },
        }
    }

    /// Get a variable value, checking local scopes first, then shell variables, then environment
    pub fn get_var(&self, name: &str) -> Option<String> {
        // Handle special variables (these are never local)
        match name {
            "?" => Some(self.last_exit_code.to_string()),
            "$" => Some(self.shell_pid.to_string()),
            "0" => Some(self.script_name.clone()),
            "LINENO" => Some(self.current_line_number.to_string()),
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

impl Default for ShellState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;