//! Shell Options Module
//!
//! This module provides the `ShellOptions` struct and related functionality for managing
//! POSIX shell options that control shell behavior. These options can be set using the
//! `set` builtin command with either short flags (e.g., `-e`, `-u`) or long names
//! (e.g., `-o errexit`, `-o nounset`).
//!
//! # POSIX Compliance
//!
//! The shell options implementation follows IEEE Std 1003.1-2008 (POSIX.1-2008) for the
//! `set` builtin command. Options can be:
//! - Enabled with `set -<flag>` or `set -o <option>`
//! - Disabled with `set +<flag>` or `set +o <option>`
//! - Listed with `set -o` (shows all options and their current state)
//!
//! # Available Options
//!
//! ## Standard POSIX Options
//!
//! - **errexit** (`-e`): Exit immediately if a command exits with a non-zero status.
//!   Does not apply to commands in conditions (if/while/until), logical chains (&&/||),
//!   or negated commands (!).
//!
//! - **nounset** (`-u`): Treat unset variables as an error when performing parameter expansion.
//!   Causes the shell to write a message to stderr and exit (in non-interactive mode).
//!
//! - **xtrace** (`-x`): Print commands and their arguments as they are executed.
//!   Useful for debugging shell scripts.
//!
//! - **verbose** (`-v`): Print shell input lines as they are read.
//!   Shows the raw input before any processing.
//!
//! - **noexec** (`-n`): Read commands but do not execute them.
//!   Useful for syntax checking scripts.
//!
//! - **noglob** (`-f`): Disable pathname expansion (globbing).
//!   Wildcards like `*` and `?` are treated as literal characters.
//!
//! - **noclobber** (`-C`): Prevent output redirection from overwriting existing files.
//!   The `>|` operator can be used to override this restriction.
//!
//! - **allexport** (`-a`): Automatically mark all variables for export to child processes.
//!   Variables become environment variables when set.
//!
//! - **notify** (`-b`): Report the status of background jobs immediately.
//!   Normally, job status is reported before the next prompt.
//!
//! - **monitor** (`-m`): Enable job control.
//!   Allows background jobs, job suspension, and job management commands.
//!
//! ## Extended Options
//!
//! - **ignoreeof**: Ignore EOF (Ctrl+D) to exit the shell.
//!   Requires explicit `exit` command to terminate the shell.
//!   Note: This option has no short flag equivalent.
//!
//! # Examples
//!
//! ```
//! use rush_sh::state::ShellOptions;
//!
//! let mut options = ShellOptions::default();
//!
//! // Enable errexit using short name
//! options.set_by_short_name('e', true).unwrap();
//! assert!(options.errexit);
//!
//! // Enable nounset using long name
//! options.set_by_long_name("nounset", true).unwrap();
//! assert!(options.nounset);
//!
//! // Check option value
//! assert_eq!(options.get_by_short_name('e'), Some(true));
//! assert_eq!(options.get_by_long_name("nounset"), Some(true));
//!
//! // List all options
//! let all_options = options.get_all_options();
//! for (name, short, value) in all_options {
//!     println!("{} ({}): {}", name, short, if value { "on" } else { "off" });
//! }
//! ```

/// Shell option flags that control shell behavior
///
/// This struct contains boolean flags for all supported shell options.
/// Each option can be accessed directly or through the getter/setter methods
/// that support both short and long option names.
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
            'e' => {
                self.errexit = value;
                Ok(())
            }
            'u' => {
                self.nounset = value;
                Ok(())
            }
            'x' => {
                self.xtrace = value;
                Ok(())
            }
            'v' => {
                self.verbose = value;
                Ok(())
            }
            'n' => {
                self.noexec = value;
                Ok(())
            }
            'f' => {
                self.noglob = value;
                Ok(())
            }
            'C' => {
                self.noclobber = value;
                Ok(())
            }
            'a' => {
                self.allexport = value;
                Ok(())
            }
            'b' => {
                self.notify = value;
                Ok(())
            }
            'm' => {
                self.monitor = value;
                Ok(())
            }
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
            "errexit" => {
                self.errexit = value;
                Ok(())
            }
            "nounset" => {
                self.nounset = value;
                Ok(())
            }
            "xtrace" => {
                self.xtrace = value;
                Ok(())
            }
            "verbose" => {
                self.verbose = value;
                Ok(())
            }
            "noexec" => {
                self.noexec = value;
                Ok(())
            }
            "noglob" => {
                self.noglob = value;
                Ok(())
            }
            "noclobber" => {
                self.noclobber = value;
                Ok(())
            }
            "allexport" => {
                self.allexport = value;
                Ok(())
            }
            "notify" => {
                self.notify = value;
                Ok(())
            }
            "ignoreeof" => {
                self.ignoreeof = value;
                Ok(())
            }
            "monitor" => {
                self.monitor = value;
                Ok(())
            }
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