//! Variable and wildcard expansion functionality for the Rush shell.
//!
//! This module handles the expansion of shell variables, command substitutions,
//! arithmetic expressions, and wildcard patterns in command arguments.

use crate::parser::Ast;
use crate::state::ShellState;

/// Expand variables in a list of argument strings.
///
/// Processes each argument through [`expand_variables_in_string`] to perform
/// variable expansion, command substitution, and arithmetic evaluation.
///
/// # Arguments
/// * `args` - Slice of argument strings to expand
/// * `shell_state` - Mutable reference to shell state for variable lookups
///
/// # Returns
/// Vector of expanded argument strings
pub(crate) fn expand_variables_in_args(args: &[String], shell_state: &mut ShellState) -> Vec<String> {
    let mut expanded_args = Vec::new();

    for arg in args {
        // Expand variables within the argument string
        let expanded_arg = expand_variables_in_string(arg, shell_state);
        expanded_args.push(expanded_arg);
    }

    expanded_args
}

/// Expands shell-style variables, command substitutions, arithmetic expressions, and backtick substitutions inside a string.
///
/// This function processes `$VAR` and positional/special parameters (`$1`, `$?`, `$#`, `$*`, `$@`, `$$`, `$0`), command substitutions using `$(...)` and backticks, and arithmetic expansions using `$((...))`, producing the resulting string with substitutions applied. Undefined numeric positional parameters and the documented special parameters expand to an empty string; other undefined variable names are left as literal `$NAME`. Arithmetic evaluation errors are rendered as an error message (colorized when the shell state enables colors). Command substitutions are parsed and executed using the current shell state; on failure the original substitution text is preserved.
///
/// # Examples
///
/// ```no_run
/// use rush_sh::ShellState;
/// use rush_sh::executor::expand_variables_in_string;
/// // assume `shell_state` is a mutable ShellState with VAR=hello
/// let mut shell_state = ShellState::new();
/// shell_state.set_var("VAR", "hello".to_string());
/// let input = "Value:$VAR";
/// let out = expand_variables_in_string(input, &mut shell_state);
/// assert_eq!(out, "Value:hello");
/// ```
///
/// # Errors
///
/// Returns `Err` if input cannot be tokenized
pub fn expand_variables_in_string(input: &str, shell_state: &mut ShellState) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            // Check for command substitution $(...) or arithmetic expansion $((...))
            if let Some(&'(') = chars.peek() {
                chars.next(); // consume first (

                // Check if this is arithmetic expansion $((...))
                if let Some(&'(') = chars.peek() {
                    // Arithmetic expansion $((...))
                    chars.next(); // consume second (
                    let mut arithmetic_expr = String::new();
                    let mut paren_depth = 1;
                    let mut found_closing = false;

                    while let Some(c) = chars.next() {
                        if c == '(' {
                            paren_depth += 1;
                            arithmetic_expr.push(c);
                        } else if c == ')' {
                            paren_depth -= 1;
                            if paren_depth == 0 {
                                // Found the first closing ) - check for second )
                                if let Some(&')') = chars.peek() {
                                    chars.next(); // consume the second )
                                    found_closing = true;
                                    break;
                                } else {
                                    // Missing second closing paren, treat as error
                                    result.push_str("$((");
                                    result.push_str(&arithmetic_expr);
                                    result.push(')');
                                    break;
                                }
                            }
                            arithmetic_expr.push(c);
                        } else {
                            arithmetic_expr.push(c);
                        }
                    }

                    if found_closing {
                        // First expand variables in the arithmetic expression
                        // The arithmetic evaluator expects variable names without $ prefix
                        // So we need to expand $VAR to the value before evaluation
                        let mut expanded_expr = String::new();
                        let mut expr_chars = arithmetic_expr.chars().peekable();

                        while let Some(ch) = expr_chars.next() {
                            if ch == '$' {
                                // Expand variable
                                let mut var_name = String::new();
                                if let Some(&c) = expr_chars.peek() {
                                    if c == '?'
                                        || c == '$'
                                        || c == '0'
                                        || c == '#'
                                        || c == '*'
                                        || c == '@'
                                        || c == '!'
                                        || c.is_ascii_digit()
                                    {
                                        var_name.push(c);
                                        expr_chars.next();
                                    } else {
                                        while let Some(&c) = expr_chars.peek() {
                                            if c.is_alphanumeric() || c == '_' {
                                                var_name.push(c);
                                                expr_chars.next();
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                }

                                if !var_name.is_empty() {
                                    if let Some(value) = shell_state.get_var(&var_name) {
                                        expanded_expr.push_str(&value);
                                    } else {
                                        // Variable not found, use 0 for arithmetic
                                        expanded_expr.push('0');
                                    }
                                } else {
                                    expanded_expr.push('$');
                                }
                            } else {
                                expanded_expr.push(ch);
                            }
                        }

                        match crate::arithmetic::evaluate_arithmetic_expression(
                            &expanded_expr,
                            shell_state,
                        ) {
                            Ok(value) => {
                                result.push_str(&value.to_string());
                            }
                            Err(e) => {
                                // On arithmetic error, display a proper error message
                                if shell_state.colors_enabled {
                                    result.push_str(&format!(
                                        "{}arithmetic error: {}{}",
                                        shell_state.color_scheme.error, e, "\x1b[0m"
                                    ));
                                } else {
                                    result.push_str(&format!("arithmetic error: {}", e));
                                }
                            }
                        }
                    } else {
                        // Didn't find proper closing - keep as literal
                        result.push_str("$((");
                        result.push_str(&arithmetic_expr);
                        // Note: we don't add closing parens since they weren't in the input
                    }
                    continue;
                }

                // Regular command substitution $(...)
                let mut sub_command = String::new();
                let mut paren_depth = 1;

                for c in chars.by_ref() {
                    if c == '(' {
                        paren_depth += 1;
                        sub_command.push(c);
                    } else if c == ')' {
                        paren_depth -= 1;
                        if paren_depth == 0 {
                            break;
                        }
                        sub_command.push(c);
                    } else {
                        sub_command.push(c);
                    }
                }

                // Execute the command substitution within the current shell context
                // Parse and execute the command using our own lexer/parser/executor
                if let Ok(tokens) = crate::lexer::lex(&sub_command, shell_state) {
                    // Expand aliases before parsing
                    let expanded_tokens = match crate::lexer::expand_aliases(
                        tokens,
                        shell_state,
                        &mut std::collections::HashSet::new(),
                    ) {
                        Ok(t) => t,
                        Err(_) => {
                            // Alias expansion error, keep literal
                            result.push_str("$(");
                            result.push_str(&sub_command);
                            result.push(')');
                            continue;
                        }
                    };

                    match crate::parser::parse(expanded_tokens) {
                        Ok(ast) => {
                            // Execute within current shell context and capture output
                            match super::execute_and_capture_output(ast, shell_state) {
                                Ok(output) => {
                                    result.push_str(&output);
                                }
                                Err(_) => {
                                    // On failure, keep the literal
                                    result.push_str("$(");
                                    result.push_str(&sub_command);
                                    result.push(')');
                                }
                            }
                        }
                        Err(_parse_err) => {
                            // Parse error - try to handle as function call if it looks like one
                            let tokens_str = sub_command.trim();
                            if tokens_str.contains(' ') {
                                // Split by spaces and check if first token looks like a function call
                                let parts: Vec<&str> = tokens_str.split_whitespace().collect();
                                if let Some(first_token) = parts.first()
                                    && shell_state.get_function(first_token).is_some()
                                {
                                    // This is a function call, create AST manually
                                    let function_call = Ast::FunctionCall {
                                        name: first_token.to_string(),
                                        args: parts[1..].iter().map(|s| s.to_string()).collect(),
                                    };
                                    match super::execute_and_capture_output(function_call, shell_state) {
                                        Ok(output) => {
                                            result.push_str(&output);
                                            continue;
                                        }
                                        Err(_) => {
                                            // Fall back to literal
                                        }
                                    }
                                }
                            }
                            // Keep the literal
                            result.push_str("$(");
                            result.push_str(&sub_command);
                            result.push(')');
                        }
                    }
                } else {
                    // Lex error, keep literal
                    result.push_str("$(");
                    result.push_str(&sub_command);
                    result.push(')');
                }
            } else if let Some(&'{') = chars.peek() {
                // ${VAR} syntax
                chars.next(); // consume the {
                let mut var_name = String::new();
                let mut found_closing = false;

                // Read until we find the closing }
                for c in chars.by_ref() {
                    if c == '}' {
                        found_closing = true;
                        break;
                    }
                    var_name.push(c);
                }

                if found_closing && !var_name.is_empty() {
                    if let Some(value) = shell_state.get_var(&var_name) {
                        result.push_str(&value);
                    } else {
                        // Variable not found - for positional parameters and special variables, expand to empty string
                        // For other variables, keep the literal
                        if var_name.chars().next().unwrap().is_ascii_digit()
                            || var_name == "?"
                            || var_name == "$"
                            || var_name == "0"
                            || var_name == "#"
                            || var_name == "*"
                            || var_name == "@"
                            || var_name == "!"
                        {
                            // Expand to empty string for undefined positional parameters and special variables
                        } else {
                            // Keep the literal for regular variables
                            result.push_str("${");
                            result.push_str(&var_name);
                            result.push('}');
                        }
                    }
                } else {
                    // Malformed ${...} - keep as literal
                    result.push_str("${");
                    result.push_str(&var_name);
                    if !found_closing {
                        // No closing brace found
                    }
                }
            } else {
                // Regular variable
                let mut var_name = String::new();
                let mut next_ch = chars.peek();

                // Handle special single-character variables first
                if let Some(&c) = next_ch {
                    if c == '?' || c == '$' || c == '0' || c == '#' || c == '*' || c == '@' || c == '!' {
                        var_name.push(c);
                        chars.next(); // consume the character
                    } else if c.is_ascii_digit() {
                        // Positional parameter
                        var_name.push(c);
                        chars.next();
                    } else {
                        // Regular variable name (including multi-character special variables like LINENO)
                        while let Some(&c) = next_ch {
                            if c.is_alphanumeric() || c == '_' {
                                var_name.push(c);
                                chars.next(); // consume the character
                                next_ch = chars.peek();
                            } else {
                                break;
                            }
                        }
                    }
                }

                if !var_name.is_empty() {
                    if let Some(value) = shell_state.get_var(&var_name) {
                        result.push_str(&value);
                    } else {
                        // Variable not found - for positional parameters and special variables, expand to empty string
                        // For other variables, keep the literal
                        if var_name.chars().next().unwrap().is_ascii_digit()
                            || var_name == "?"
                            || var_name == "$"
                            || var_name == "0"
                            || var_name == "#"
                            || var_name == "*"
                            || var_name == "@"
                            || var_name == "!"
                        {
                            // Expand to empty string for undefined positional parameters and special variables
                        } else {
                            // Keep the literal for regular variables
                            result.push('$');
                            result.push_str(&var_name);
                        }
                    }
                } else {
                    result.push('$');
                }
            }
        } else if ch == '`' {
            // Backtick command substitution
            let mut sub_command = String::new();

            for c in chars.by_ref() {
                if c == '`' {
                    break;
                }
                sub_command.push(c);
            }

            // Execute the command substitution
            if let Ok(tokens) = crate::lexer::lex(&sub_command, shell_state) {
                // Expand aliases before parsing
                let expanded_tokens = match crate::lexer::expand_aliases(
                    tokens,
                    shell_state,
                    &mut std::collections::HashSet::new(),
                ) {
                    Ok(t) => t,
                    Err(_) => {
                        // Alias expansion error, keep literal
                        result.push('`');
                        result.push_str(&sub_command);
                        result.push('`');
                        continue;
                    }
                };

                if let Ok(ast) = crate::parser::parse(expanded_tokens) {
                    // Execute and capture output
                    match super::execute_and_capture_output(ast, shell_state) {
                        Ok(output) => {
                            result.push_str(&output);
                        }
                        Err(_) => {
                            // On failure, keep the literal
                            result.push('`');
                            result.push_str(&sub_command);
                            result.push('`');
                        }
                    }
                } else {
                    // Parse error, keep literal
                    result.push('`');
                    result.push_str(&sub_command);
                    result.push('`');
                }
            } else {
                // Lex error, keep literal
                result.push('`');
                result.push_str(&sub_command);
                result.push('`');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Expand shell-style wildcard patterns in a list of arguments unless the `noglob` option is set.
///
/// Patterns containing `*`, `?`, or `[` are replaced by the sorted list of matching filesystem paths. If a pattern has no matches or is an invalid pattern, the original literal argument is kept. If the shell state's `noglob` option is enabled, all arguments are returned unchanged.
///
/// # Examples
///
/// ```
/// // Note: expand_wildcards is a private function
/// // This example is for documentation only
/// ```
pub(crate) fn expand_wildcards(args: &[String], shell_state: &ShellState) -> Result<Vec<String>, String> {
    let mut expanded_args = Vec::new();

    for arg in args {
        // Skip wildcard expansion if noglob option (-f) is enabled
        if shell_state.options.noglob {
            expanded_args.push(arg.clone());
            continue;
        }
        
        if arg.contains('*') || arg.contains('?') || arg.contains('[') {
            // Try to expand wildcard
            match glob::glob(arg) {
                Ok(paths) => {
                    let mut matches: Vec<String> = paths
                        .filter_map(|p| p.ok())
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();
                    if matches.is_empty() {
                        // No matches, keep literal
                        expanded_args.push(arg.clone());
                    } else {
                        // Sort for consistent behavior
                        matches.sort();
                        expanded_args.extend(matches);
                    }
                }
                Err(_e) => {
                    // Invalid pattern, keep literal
                    expanded_args.push(arg.clone());
                }
            }
        } else {
            expanded_args.push(arg.clone());
        }
    }
    Ok(expanded_args)
}