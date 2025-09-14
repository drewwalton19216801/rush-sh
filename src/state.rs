use std::collections::{HashMap, HashSet};
use std::env;
use std::io::IsTerminal;

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
}

impl ShellState {
    pub fn new() -> Self {
        let shell_pid = std::process::id();

        // Check NO_COLOR environment variable (respects standard)
        let no_color = std::env::var("NO_COLOR").is_ok();

        // Check RUSH_COLORS environment variable for explicit control
        let rush_colors = std::env::var("RUSH_COLORS")
            .map(|v| v.to_lowercase())
            .unwrap_or_else(|_| "auto".to_string());

        let colors_enabled = match rush_colors.as_str() {
            "1" | "true" | "on" | "enable" => !no_color && std::io::stdout().is_terminal(),
            "0" | "false" | "off" | "disable" => false,
            "auto" => !no_color && std::io::stdout().is_terminal(),
            _ => !no_color && std::io::stdout().is_terminal(),
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
        }
    }

    /// Get a variable value, checking shell variables first, then environment
    pub fn get_var(&self, name: &str) -> Option<String> {
        // Handle special variables
        match name {
            "?" => Some(self.last_exit_code.to_string()),
            "$" => Some(self.shell_pid.to_string()),
            "0" => Some(self.script_name.clone()),
            _ => {
                // Check shell variables first
                if let Some(value) = self.variables.get(name) {
                    Some(value.clone())
                } else {
                    // Fall back to environment variables
                    env::var(name).ok()
                }
            }
        }
    }

    /// Set a shell variable
    pub fn set_var(&mut self, name: &str, value: String) {
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
        match std::env::current_dir() {
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

    /// Get the user@hostname string for the prompt
    pub fn get_user_hostname(&self) -> String {
        let user = env::var("USER").unwrap_or_else(|_| "user".to_string());
        let hostname = env::var("HOSTNAME").unwrap_or_else(|_| "hostname".to_string());
        format!("{}@{}", user, hostname)
    }

    /// Get the full prompt string
    pub fn get_prompt(&self) -> String {
        let user = env::var("USER").unwrap_or_else(|_| "user".to_string());
        let prompt_char = if user == "root" { "#" } else { "$" };
        format!(
            "{}:{} {} ",
            self.get_user_hostname(),
            self.get_condensed_cwd(),
            prompt_char
        )
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
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
