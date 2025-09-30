use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;
use super::parser::Ast;
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
    /// Output capture buffer for command substitution
    pub capture_output: Option<Rc<RefCell<Vec<u8>>>>,
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
            positional_params: Vec::new(),
            functions: HashMap::new(),
            local_vars: Vec::new(),
            function_depth: 0,
            max_recursion_depth: 500, // Default recursion limit (reduced to avoid Rust stack overflow)
            returning: false,
            return_value: None,
            capture_output: None,
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

    /// Get a local variable from any scope (local first, then global)
    pub fn get_local_var(&self, name: &str) -> Option<String> {
        // Search local scopes from innermost to outermost
        for scope in self.local_vars.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        // Fall back to global variables (but avoid recursion by not calling get_var)
        self.variables.get(name).map(|v| v.clone())
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

    /// Serialize a function to string format for export
    pub fn serialize_function(&self, name: &str) -> Option<String> {
        if let Some(ast) = self.get_function(name) {
            Some(format_function_definition(name, ast))
        } else {
            None
        }
    }

    /// Deserialize a function from string format for import
    pub fn deserialize_function(&mut self, definition: &str) -> Result<(), String> {
        // Parse the function definition
        let tokens = match crate::lexer::lex(definition, self) {
            Ok(t) => t,
            Err(e) => return Err(format!("Failed to lex function definition: {}", e)),
        };

        let ast = match crate::parser::parse(tokens) {
            Ok(a) => a,
            Err(e) => return Err(format!("Failed to parse function definition: {}", e)),
        };

        if let crate::parser::Ast::FunctionDefinition { name, body } = ast {
            self.define_function(name, *body);
            Ok(())
        } else {
            Err("Provided definition is not a function".to_string())
        }
    }

    /// Export all functions to a string format
    pub fn export_functions(&self) -> String {
        let mut result = String::new();
        let function_names: Vec<&String> = self.get_function_names();

        for name in function_names {
            if let Some(definition) = self.serialize_function(name) {
                result.push_str(&definition);
                result.push('\n');
            }
        }

        result
    }

    /// Import functions from a string format
    pub fn import_functions(&mut self, functions_data: &str) -> Result<usize, String> {
        let mut imported_count = 0;
        let mut last_error = None;

        // Split by newlines and process each line as a potential function definition
        for line in functions_data.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue; // Skip empty lines and comments
            }

            match self.deserialize_function(trimmed) {
                Ok(()) => {
                    imported_count += 1;
                }
                Err(e) => {
                    last_error = Some(e);
                    // Continue trying to import other functions even if one fails
                }
            }
        }

        if imported_count == 0 && last_error.is_some() {
            return Err(last_error.unwrap());
        }

        Ok(imported_count)
    }

}

/// Helper function to format function definition
fn format_function_definition(name: &str, ast: &crate::parser::Ast) -> String {
    format!("{}() {{\n{}\n}}", name, format_ast_body(ast, 1))
}

/// Helper function to recursively format AST
fn format_ast_body(ast: &crate::parser::Ast, indent_level: usize) -> String {
    let indent = "    ".repeat(indent_level);

    match ast {
        crate::parser::Ast::Pipeline(commands) => {
            if commands.len() == 1 {
                format!("{} {}", indent, format_command(&commands[0]))
            } else {
                let mut result = String::new();
                for (i, cmd) in commands.iter().enumerate() {
                    if i > 0 {
                        result.push_str(" |");
                    }
                    result.push('\n');
                    result.push_str(&format!("{} {}", indent, format_command(cmd)));
                }
                result
            }
        }
        crate::parser::Ast::Sequence(asts) => {
            let mut result = String::new();
            for (i, ast_node) in asts.iter().enumerate() {
                if i > 0 {
                    result.push_str(";\n");
                }
                result.push_str(&format_ast_body(ast_node, indent_level));
            }
            result
        }
        crate::parser::Ast::Assignment { var, value } => {
            format!("{} {}={}", indent, var, value)
        }
        crate::parser::Ast::LocalAssignment { var, value } => {
            format!("{} local {}={}", indent, var, value)
        }
        crate::parser::Ast::If { branches, else_branch } => {
            let mut result = String::new();

            for (i, (condition, then_branch)) in branches.iter().enumerate() {
                if i == 0 {
                    result.push_str(&format!("{}if ", indent));
                } else {
                    result.push_str(&format!("\n{}elif ", indent));
                }
                result.push_str(&format_ast_body(condition, 0));
                result.push_str("; then\n");
                result.push_str(&format_ast_body(then_branch, indent_level));
            }

            if let Some(else_b) = else_branch {
                result.push_str(&format!("\n{}else\n", indent));
                result.push_str(&format_ast_body(else_b, indent_level));
            }

            result.push_str(&format!("\n{}fi", indent));
            result
        }
        crate::parser::Ast::Case { word, cases, default } => {
            let mut result = String::new();
            result.push_str(&format!("{}case {} in\n", indent, word));

            for (patterns, branch) in cases {
                for pattern in patterns {
                    result.push_str(&format!("{}    {})\n", indent, pattern));
                }
                result.push_str(&format_ast_body(branch, indent_level + 1));
                result.push_str(&format!("\n{}    ;;\n", indent));
            }

            if let Some(def) = default {
                result.push_str(&format!("{}    *)\n", indent));
                result.push_str(&format_ast_body(def, indent_level + 1));
                result.push_str(&format!("\n{}    ;;\n", indent));
            }

            result.push_str(&format!("{}esac", indent));
            result
        }
        crate::parser::Ast::For { variable, items, body } => {
            let mut result = String::new();
            result.push_str(&format!("{}for {} in", indent, variable));
            for item in items {
                result.push_str(&format!(" {}", item));
            }
            result.push_str("; do\n");
            result.push_str(&format_ast_body(body, indent_level + 1));
            result.push_str(&format!("\n{}done", indent));
            result
        }
        crate::parser::Ast::While { condition, body } => {
            let mut result = String::new();
            result.push_str(&format!("{}while ", indent));
            result.push_str(&format_ast_body(condition, 0).trim());
            result.push_str("; do\n");
            result.push_str(&format_ast_body(body, indent_level + 1));
            result.push_str(&format!("\n{}done", indent));
            result
        }
        crate::parser::Ast::FunctionDefinition { name, body } => {
            format!("{}() {{\n{}    {}\n{}}}\n", name, indent, format_ast_body(body, indent_level), indent)
        }
        crate::parser::Ast::FunctionCall { name, args } => {
            if args.is_empty() {
                format!("{} {}", indent, name)
            } else {
                format!("{} {} {}", indent, name, args.join(" "))
            }
        }
        crate::parser::Ast::Return { value } => {
            if let Some(val) = value {
                format!("{} return {}", indent, val)
            } else {
                format!("{} return", indent)
            }
        }
        crate::parser::Ast::And { left, right } => {
            format!("{} && {}", format_ast_body(left, 0).trim(), format_ast_body(right, 0).trim())
        }
        crate::parser::Ast::Or { left, right } => {
            format!("{} || {}", format_ast_body(left, 0).trim(), format_ast_body(right, 0).trim())
        }
    }
}

/// Helper function to format shell command (duplicate of the one in builtin_declare.rs)
fn format_command(cmd: &crate::parser::ShellCommand) -> String {
    let mut result = cmd.args.join(" ");

    if let Some(ref input) = cmd.input {
        result.push_str(&format!(" < {}", input));
    }

    if let Some(ref output) = cmd.output {
        result.push_str(&format!(" > {}", output));
    }

    if let Some(ref append) = cmd.append {
        result.push_str(&format!(" >> {}", append));
    }

    result
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
        assert_eq!(state.get_var("global_var"), Some("global_value".to_string()));

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
        assert_eq!(state.get_var("global_var"), Some("global_value".to_string()));
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
}
