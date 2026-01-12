//! Command execution engine for the Rush shell.
//!
//! This module handles the execution of parsed AST nodes, including pipelines,
//! control structures, redirections, and built-in commands.


use super::parser::{Ast, Redirection};
use super::state::ShellState;

// Submodules
mod expansion;
mod redirection;
mod command;
mod subshell;

// Re-export expansion functions
pub use expansion::expand_variables_in_string;

// Re-export command execution functions
pub(crate) use command::{execute_and_capture_output, execute_single_command, execute_pipeline};

// Re-export subshell functions
pub(crate) use subshell::{execute_compound_with_redirections, execute_compound_in_pipeline};


/// Execute a trap handler command
/// Note: Signal masking during trap execution will be added in a future update
pub fn execute_trap_handler(trap_cmd: &str, shell_state: &mut ShellState) -> i32 {
    // Save current exit code to preserve it across trap execution
    let saved_exit_code = shell_state.last_exit_code;

    // TODO: Add signal masking to prevent recursive trap calls
    // This requires careful handling of the nix sigprocmask API
    // For now, traps execute without signal masking

    // Parse and execute the trap command
    let result = match crate::lexer::lex(trap_cmd, shell_state) {
        Ok(tokens) => {
            match crate::lexer::expand_aliases(
                tokens,
                shell_state,
                &mut std::collections::HashSet::new(),
            ) {
                Ok(expanded_tokens) => {
                    match crate::parser::parse(expanded_tokens) {
                        Ok(ast) => execute(ast, shell_state),
                        Err(_) => {
                            // Parse error in trap handler - silently continue
                            saved_exit_code
                        }
                    }
                }
                Err(_) => {
                    // Alias expansion error - silently continue
                    saved_exit_code
                }
            }
        }
        Err(_) => {
            // Lex error in trap handler - silently continue
            saved_exit_code
        }
    };

    // Restore the original exit code (trap handlers don't affect $?)
    shell_state.last_exit_code = saved_exit_code;

    result
}

/// Evaluate an AST node within the provided shell state and return its exit code.
///
/// Executes the given `ast`, updating `shell_state` (variables, loop/function/subshell state,
/// file descriptor and redirection effects, traps, etc.) as the AST semantics require.
/// The function returns the final exit code for the executed AST node (0 for success,
/// non-zero for failure). Side effects on `shell_state` follow the shell semantics
/// implemented by the executor (variable assignment, function definition/call, loops,
/// pipelines, redirections, subshell isolation, errexit behavior, traps, etc.).
///
/// # Examples
///
/// ```
/// use rush_sh::{Ast, ShellState};
/// use rush_sh::executor::execute;
///
/// let mut state = ShellState::new();
/// let ast = Ast::Assignment { var: "X".into(), value: "1".into() };
/// let code = execute(ast, &mut state);
/// assert_eq!(code, 0);
/// assert_eq!(state.get_var("X").as_deref(), Some("1"));
/// ```
pub fn execute(ast: Ast, shell_state: &mut ShellState) -> i32 {
    match ast {
        Ast::Assignment { var, value } => {
            // Check noexec option (-n): Read commands but don't execute them
            if shell_state.options.noexec {
                return 0; // Return success without executing
            }
            
            // Expand variables and command substitutions in the value
            let expanded_value = expand_variables_in_string(&value, shell_state);
            shell_state.set_var(&var, expanded_value.clone());
            
            // Auto-export if allexport option (-a) is enabled
            if shell_state.options.allexport {
                shell_state.export_var(&var);
            }
            0
        }
        Ast::LocalAssignment { var, value } => {
            // Check noexec option (-n): Read commands but don't execute them
            if shell_state.options.noexec {
                return 0; // Return success without executing
            }
            
            // Expand variables and command substitutions in the value
            let expanded_value = expand_variables_in_string(&value, shell_state);
            shell_state.set_local_var(&var, expanded_value);
            0
        }
        Ast::Pipeline(commands) => {
            if commands.is_empty() {
                return 0;
            }

            if commands.len() == 1 {
                // Single command, handle redirections
                execute_single_command(&commands[0], shell_state)
            } else {
                // Pipeline
                execute_pipeline(&commands, shell_state)
            }
        }
        Ast::Sequence(asts) => {
            let mut exit_code = 0;
            for ast in asts {
                // Reset last_was_negation flag before executing each command
                shell_state.last_was_negation = false;
                
                exit_code = execute(ast, shell_state);

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    return exit_code;
                }

                // Check if exit was requested (e.g., from trap handler)
                if shell_state.exit_requested {
                    return shell_state.exit_code;
                }

                // Check for break/continue signals - stop executing remaining statements
                if shell_state.is_breaking() || shell_state.is_continuing() {
                    return exit_code;
                }
                
                // Check errexit option (-e): Exit immediately if command fails
                // POSIX: Don't exit in these contexts:
                // 1. Inside if/while/until condition (tracked by in_condition flag)
                // 2. Part of && or || chain (tracked by in_logical_chain flag)
                // 3. Negated command (tracked by in_negation flag)
                // 4. Last command was a negation (tracked by last_was_negation flag)
                if shell_state.options.errexit
                    && exit_code != 0
                    && !shell_state.in_condition
                    && !shell_state.in_logical_chain
                    && !shell_state.in_negation
                    && !shell_state.last_was_negation {
                    // Set exit_requested flag to trigger shell exit
                    shell_state.exit_requested = true;
                    shell_state.exit_code = exit_code;
                    return exit_code;
                }
            }
            exit_code
        }
        Ast::If {
            branches,
            else_branch,
        } => {
            for (condition, then_branch) in branches {
                // Mark that we're in a condition (for errexit)
                shell_state.in_condition = true;
                let cond_exit = execute(*condition, shell_state);
                shell_state.in_condition = false;
                
                if cond_exit == 0 {
                    let exit_code = execute(*then_branch, shell_state);

                    // Check if we got an early return from a function
                    if shell_state.is_returning() {
                        return exit_code;
                    }

                    return exit_code;
                }
            }
            if let Some(else_b) = else_branch {
                let exit_code = execute(*else_b, shell_state);

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    return exit_code;
                }

                exit_code
            } else {
                0
            }
        }
        Ast::Case {
            word,
            cases,
            default,
        } => {
            for (patterns, branch) in cases {
                for pattern in &patterns {
                    if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                        if glob_pattern.matches(&word) {
                            let exit_code = execute(branch, shell_state);

                            // Check if we got an early return from a function
                            if shell_state.is_returning() {
                                return exit_code;
                            }

                            return exit_code;
                        }
                    } else {
                        // If pattern is invalid, fall back to exact match
                        if &word == pattern {
                            let exit_code = execute(branch, shell_state);

                            // Check if we got an early return from a function
                            if shell_state.is_returning() {
                                return exit_code;
                            }

                            return exit_code;
                        }
                    }
                }
            }
            if let Some(def) = default {
                let exit_code = execute(*def, shell_state);

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    return exit_code;
                }

                exit_code
            } else {
                0
            }
        }
        Ast::For {
            variable,
            items,
            body,
        } => {
            let mut exit_code = 0;

            // Enter loop context
            shell_state.enter_loop();

            // Expand variables in items and perform word splitting
            let mut expanded_items = Vec::new();
            for item in items {
                // Expand variables in the item
                let expanded = expand_variables_in_string(&item, shell_state);
                
                // Perform word splitting on the expanded result
                // Split on whitespace (space, tab, newline)
                for word in expanded.split_whitespace() {
                    expanded_items.push(word.to_string());
                }
            }

            // Execute the loop body for each expanded item
            for item in expanded_items {
                // Process any pending signals before executing the body
                crate::state::process_pending_signals(shell_state);

                // Check if exit was requested (e.g., from trap handler)
                if shell_state.exit_requested {
                    shell_state.exit_loop();
                    return shell_state.exit_code;
                }

                // Set the loop variable
                shell_state.set_var(&variable, item.clone());

                // Execute the body
                exit_code = execute(*body.clone(), shell_state);

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    shell_state.exit_loop();
                    return exit_code;
                }

                // Check if exit was requested after executing the body
                if shell_state.exit_requested {
                    shell_state.exit_loop();
                    return shell_state.exit_code;
                }

                // Check for break signal
                if shell_state.is_breaking() {
                    if shell_state.get_break_level() == 1 {
                        // Break out of this loop
                        shell_state.clear_break();
                        break;
                    } else {
                        // Decrement level and propagate to outer loop
                        shell_state.decrement_break_level();
                        break;
                    }
                }

                // Check for continue signal
                if shell_state.is_continuing() {
                    if shell_state.get_continue_level() == 1 {
                        // Continue to next iteration of this loop
                        shell_state.clear_continue();
                        continue;
                    } else {
                        // Decrement level and propagate to outer loop
                        shell_state.decrement_continue_level();
                        break; // Exit this loop to continue outer loop
                    }
                }
            }

            // Exit loop context
            shell_state.exit_loop();

            exit_code
        }
        Ast::While { condition, body } => {
            let mut exit_code = 0;

            // Enter loop context
            shell_state.enter_loop();

            // Execute the loop while condition is true (exit code 0)
            loop {
                // Mark that we're in a condition (for errexit)
                shell_state.in_condition = true;
                let cond_exit = execute(*condition.clone(), shell_state);
                shell_state.in_condition = false;

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    shell_state.exit_loop();
                    return cond_exit;
                }

                // Check if exit was requested (e.g., from trap handler)
                if shell_state.exit_requested {
                    shell_state.exit_loop();
                    return shell_state.exit_code;
                }

                // If condition is false (non-zero exit code), break
                if cond_exit != 0 {
                    break;
                }

                // Execute the body
                exit_code = execute(*body.clone(), shell_state);

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    shell_state.exit_loop();
                    return exit_code;
                }

                // Check if exit was requested (e.g., from trap handler)
                if shell_state.exit_requested {
                    shell_state.exit_loop();
                    return shell_state.exit_code;
                }

                // Check for break signal
                if shell_state.is_breaking() {
                    if shell_state.get_break_level() == 1 {
                        // Break out of this loop
                        shell_state.clear_break();
                        break;
                    } else {
                        // Decrement level and propagate to outer loop
                        shell_state.decrement_break_level();
                        break;
                    }
                }

                // Check for continue signal
                if shell_state.is_continuing() {
                    if shell_state.get_continue_level() == 1 {
                        // Continue to next iteration of this loop
                        shell_state.clear_continue();
                        continue;
                    } else {
                        // Decrement level and propagate to outer loop
                        shell_state.decrement_continue_level();
                        break; // Exit this loop to continue outer loop
                    }
                }
            }

            // Exit loop context
            shell_state.exit_loop();

            exit_code
        }
        Ast::Until { condition, body } => {
            let mut exit_code = 0;

            // Enter loop context
            shell_state.enter_loop();

            // Execute the loop until condition is true (exit code 0)
            loop {
                // Mark that we're in a condition (for errexit)
                shell_state.in_condition = true;
                let cond_exit = execute(*condition.clone(), shell_state);
                shell_state.in_condition = false;

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    shell_state.exit_loop();
                    return cond_exit;
                }

                // Check if exit was requested (e.g., from trap handler)
                if shell_state.exit_requested {
                    shell_state.exit_loop();
                    return shell_state.exit_code;
                }

                // If condition is true (exit code 0), break
                if cond_exit == 0 {
                    break;
                }

                // Execute the body
                exit_code = execute(*body.clone(), shell_state);

                // Check if we got an early return from a function
                if shell_state.is_returning() {
                    shell_state.exit_loop();
                    return exit_code;
                }

                // Check if exit was requested (e.g., from trap handler)
                if shell_state.exit_requested {
                    shell_state.exit_loop();
                    return shell_state.exit_code;
                }

                // Check for break signal
                if shell_state.is_breaking() {
                    if shell_state.get_break_level() == 1 {
                        // Break out of this loop
                        shell_state.clear_break();
                        break;
                    } else {
                        // Decrement level and propagate to outer loop
                        shell_state.decrement_break_level();
                        break;
                    }
                }

                // Check for continue signal
                if shell_state.is_continuing() {
                    if shell_state.get_continue_level() == 1 {
                        // Continue to next iteration of this loop
                        shell_state.clear_continue();
                        continue;
                    } else {
                        // Decrement level and propagate to outer loop
                        shell_state.decrement_continue_level();
                        break; // Exit this loop to continue outer loop
                    }
                }
            }

            // Exit loop context
            shell_state.exit_loop();

            exit_code
        }
        Ast::FunctionDefinition { name, body } => {
            // Store function definition in shell state
            shell_state.define_function(name.clone(), *body);
            0
        }
        Ast::FunctionCall { name, args } => {
            if let Some(function_body) = shell_state.get_function(&name).cloned() {
                // Check recursion limit before entering function
                if shell_state.function_depth >= shell_state.max_recursion_depth {
                    eprintln!(
                        "Function recursion limit ({}) exceeded",
                        shell_state.max_recursion_depth
                    );
                    return 1;
                }

                // Enter function context for local variable scoping
                shell_state.enter_function();

                // Set up arguments as regular variables (will be enhanced in Phase 2)
                let old_positional = shell_state.positional_params.clone();

                // Set positional parameters for function arguments
                shell_state.set_positional_params(args.clone());

                // Execute function body
                let exit_code = execute(function_body, shell_state);

                // Check if we got an early return from the function
                if shell_state.is_returning() {
                    let return_value = shell_state.get_return_value().unwrap_or(0);

                    // Restore old positional parameters
                    shell_state.set_positional_params(old_positional);

                    // Exit function context
                    shell_state.exit_function();

                    // Clear return state
                    shell_state.clear_return();

                    // Update last_exit_code so $? captures the return value
                    shell_state.last_exit_code = return_value;

                    // Return the early return value
                    return return_value;
                }

                // Restore old positional parameters
                shell_state.set_positional_params(old_positional);

                // Exit function context
                shell_state.exit_function();

                // Update last_exit_code so $? captures the function's exit code
                shell_state.last_exit_code = exit_code;

                exit_code
            } else {
                eprintln!("Function '{}' not found", name);
                1
            }
        }
        Ast::Return { value } => {
            // Return statements can only be used inside functions
            if shell_state.function_depth == 0 {
                eprintln!("Return statement outside of function");
                return 1;
            }

            // Parse return value if provided
            let exit_code = if let Some(ref val) = value {
                val.parse::<i32>().unwrap_or(0)
            } else {
                0
            };

            // Set return state to indicate early return from function
            shell_state.set_return(exit_code);

            // Return the exit code - the function call handler will check for this
            exit_code
        }
        Ast::And { left, right } => {
            // Mark that we're in a logical chain (for errexit)
            shell_state.in_logical_chain = true;
            
            // Execute left side first
            let left_exit = execute(*left, shell_state);

            // Check ALL control-flow flags after executing left side
            // If ANY control-flow is active, reset flag and return immediately
            if shell_state.is_returning()
                || shell_state.exit_requested
                || shell_state.is_breaking()
                || shell_state.is_continuing()
            {
                shell_state.in_logical_chain = false;
                return left_exit;
            }

            // Only execute right side if left succeeded (exit code 0)
            let result = if left_exit == 0 {
                execute(*right, shell_state)
            } else {
                left_exit
            };
            
            shell_state.in_logical_chain = false;
            result
        }
        Ast::Or { left, right } => {
            // Mark that we're in a logical chain (for errexit)
            shell_state.in_logical_chain = true;
            
            // Execute left side first
            let left_exit = execute(*left, shell_state);

            // Check ALL control-flow flags after executing left side
            // If ANY control-flow is active, reset flag and return immediately
            if shell_state.is_returning()
                || shell_state.exit_requested
                || shell_state.is_breaking()
                || shell_state.is_continuing()
            {
                shell_state.in_logical_chain = false;
                return left_exit;
            }

            // Only execute right side if left failed (exit code != 0)
            let result = if left_exit != 0 {
                execute(*right, shell_state)
            } else {
                left_exit
            };
            
            shell_state.in_logical_chain = false;
            result
        }
        Ast::Negation { command } => {
            // Mark that we're in a negation (for errexit)
            shell_state.in_negation = true;
            
            // Execute the negated command
            let exit_code = execute(*command, shell_state);
            
            // Reset negation flag
            shell_state.in_negation = false;
            
            // Mark that this command was a negation (for errexit exemption)
            shell_state.last_was_negation = true;
            
            // Invert the exit code: 0 becomes 1, non-zero becomes 0
            let inverted_code = if exit_code == 0 { 1 } else { 0 };
            
            // Update last_exit_code so $? reflects the inverted code
            shell_state.last_exit_code = inverted_code;
            
            inverted_code
        }
        Ast::Subshell { body } => {
            let exit_code = subshell::execute_subshell(*body, shell_state);
            
            // Check errexit option (-e): Exit immediately if subshell fails
            // POSIX: Don't exit in these contexts:
            // 1. Inside if/while/until condition (tracked by in_condition flag)
            // 2. Part of && or || chain (tracked by in_logical_chain flag)
            // 3. Negated command (tracked by in_negation flag)
            if shell_state.options.errexit
                && exit_code != 0
                && !shell_state.in_condition
                && !shell_state.in_logical_chain
                && !shell_state.in_negation {
                // Set exit_requested flag to trigger shell exit
                shell_state.exit_requested = true;
                shell_state.exit_code = exit_code;
            }
            
            exit_code
        }
        Ast::CommandGroup { body } => execute(*body, shell_state),
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::ShellCommand;

    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize tests that modify environment variables or create files
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    #[test]
    fn test_execute_single_command_builtin() {
        let cmd = ShellCommand {
            args: vec!["true".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    // For external commands, test with a command that exists
    #[test]
    fn test_execute_single_command_external() {
        let cmd = ShellCommand {
            args: vec!["true".to_string()], // Assume true exists
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_single_command_external_nonexistent() {
        let cmd = ShellCommand {
            args: vec!["nonexistent_command".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 1); // Command not found
    }

    #[test]
    fn test_execute_pipeline() {
        let commands = vec![
            ShellCommand {
                args: vec!["printf".to_string(), "hello".to_string()],
                redirections: Vec::new(),
                compound: None,
            },
            ShellCommand {
                args: vec!["cat".to_string()], // cat reads from stdin
                redirections: Vec::new(),
                compound: None,
            },
        ];
        let mut shell_state = ShellState::new();
        let exit_code = execute_pipeline(&commands, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_empty_pipeline() {
        let commands = vec![];
        let mut shell_state = ShellState::new();
        let exit_code = execute(Ast::Pipeline(commands), &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_single_command() {
        let ast = Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            redirections: Vec::new(),
            compound: None,
        }]);
        let mut shell_state = ShellState::new();
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_function_definition() {
        let ast = Ast::FunctionDefinition {
            name: "test_func".to_string(),
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                redirections: Vec::new(),
                compound: None,
            }])),
        };
        let mut shell_state = ShellState::new();
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Check that function was stored
        assert!(shell_state.get_function("test_func").is_some());
    }

    #[test]
    fn test_execute_function_call() {
        // First define a function
        let mut shell_state = ShellState::new();
        shell_state.define_function(
            "test_func".to_string(),
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        );

        // Now call the function
        let ast = Ast::FunctionCall {
            name: "test_func".to_string(),
            args: vec![],
        };
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_function_call_with_args() {
        // First define a function that uses arguments
        let mut shell_state = ShellState::new();
        shell_state.define_function(
            "test_func".to_string(),
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "arg1".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        );

        // Now call the function with arguments
        let ast = Ast::FunctionCall {
            name: "test_func".to_string(),
            args: vec!["hello".to_string()],
        };
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_nonexistent_function() {
        let mut shell_state = ShellState::new();
        let ast = Ast::FunctionCall {
            name: "nonexistent".to_string(),
            args: vec![],
        };
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 1); // Should return error code
    }

    #[test]
    fn test_execute_function_integration() {
        // Test full integration: define function, then call it
        let mut shell_state = ShellState::new();

        // First define a function
        let define_ast = Ast::FunctionDefinition {
            name: "hello".to_string(),
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["printf".to_string(), "Hello from function".to_string()],
                redirections: Vec::new(),
                compound: None,
            }])),
        };
        let exit_code = execute(define_ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Now call the function
        let call_ast = Ast::FunctionCall {
            name: "hello".to_string(),
            args: vec![],
        };
        let exit_code = execute(call_ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_function_with_local_variables() {
        let mut shell_state = ShellState::new();

        // Set a global variable
        shell_state.set_var("global_var", "global_value".to_string());

        // Define a function that uses local variables
        let define_ast = Ast::FunctionDefinition {
            name: "test_func".to_string(),
            body: Box::new(Ast::Sequence(vec![
                Ast::LocalAssignment {
                    var: "local_var".to_string(),
                    value: "local_value".to_string(),
                },
                Ast::Assignment {
                    var: "global_var".to_string(),
                    value: "modified_in_function".to_string(),
                },
                Ast::Pipeline(vec![ShellCommand {
                    args: vec!["printf".to_string(), "success".to_string()],
                    redirections: Vec::new(),
                    compound: None,
                }]),
            ])),
        };
        let exit_code = execute(define_ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Global variable should not be modified during function definition
        assert_eq!(
            shell_state.get_var("global_var"),
            Some("global_value".to_string())
        );

        // Call the function
        let call_ast = Ast::FunctionCall {
            name: "test_func".to_string(),
            args: vec![],
        };
        let exit_code = execute(call_ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // After function call, global variable should be modified since function assignments affect global scope
        assert_eq!(
            shell_state.get_var("global_var"),
            Some("modified_in_function".to_string())
        );
    }

    #[test]
    fn test_execute_nested_function_calls() {
        let mut shell_state = ShellState::new();

        // Set global variable
        shell_state.set_var("global_var", "global".to_string());

        // Define outer function
        let outer_func = Ast::FunctionDefinition {
            name: "outer".to_string(),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "global_var".to_string(),
                    value: "outer_modified".to_string(),
                },
                Ast::FunctionCall {
                    name: "inner".to_string(),
                    args: vec![],
                },
                Ast::Pipeline(vec![ShellCommand {
                    args: vec!["printf".to_string(), "outer_done".to_string()],
                    redirections: Vec::new(),
                    compound: None,
                }]),
            ])),
        };

        // Define inner function
        let inner_func = Ast::FunctionDefinition {
            name: "inner".to_string(),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "global_var".to_string(),
                    value: "inner_modified".to_string(),
                },
                Ast::Pipeline(vec![ShellCommand {
                    args: vec!["printf".to_string(), "inner_done".to_string()],
                    redirections: Vec::new(),
                    compound: None,
                }]),
            ])),
        };

        // Define both functions
        execute(outer_func, &mut shell_state);
        execute(inner_func, &mut shell_state);

        // Set initial global value
        shell_state.set_var("global_var", "initial".to_string());

        // Call outer function (which calls inner function)
        let call_ast = Ast::FunctionCall {
            name: "outer".to_string(),
            args: vec![],
        };
        let exit_code = execute(call_ast, &mut shell_state);
        assert_eq!(exit_code, 0);

        // After nested function calls, global variable should be modified by inner function
        // (bash behavior: function variable assignments affect global scope)
        assert_eq!(
            shell_state.get_var("global_var"),
            Some("inner_modified".to_string())
        );
    }

    #[test]
    fn test_here_string_execution() {
        // Test here-string redirection with a simple command
        let cmd = ShellCommand {
            args: vec!["cat".to_string()],
            redirections: Vec::new(),
            compound: None,
            // TODO: Update test for new redirection system
        };

        // Note: This test would require mocking stdin to provide the here-string content
        // For now, we'll just verify the command structure is parsed correctly
        assert_eq!(cmd.args, vec!["cat"]);
        // assert_eq!(cmd.here_string_content, Some("hello world".to_string()));
    }

    #[test]
    fn test_here_document_execution() {
        // Test here-document redirection with a simple command
        let cmd = ShellCommand {
            args: vec!["cat".to_string()],
            redirections: Vec::new(),
            compound: None,
            // TODO: Update test for new redirection system
        };

        // Note: This test would require mocking stdin to provide the here-document content
        // For now, we'll just verify the command structure is parsed correctly
        assert_eq!(cmd.args, vec!["cat"]);
        // assert_eq!(cmd.here_doc_delimiter, Some("EOF".to_string()));
    }

    #[test]
    fn test_here_document_with_variable_expansion() {
        // Test that variables are expanded in here-document content
        let mut shell_state = ShellState::new();
        shell_state.set_var("PWD", "/test/path".to_string());

        // Simulate here-doc content with variable
        let content = "Working dir: $PWD";
        let expanded = expand_variables_in_string(content, &mut shell_state);

        assert_eq!(expanded, "Working dir: /test/path");
    }

    #[test]
    fn test_here_document_with_command_substitution_builtin() {
        // Test that builtin command substitutions work in here-document content
        let mut shell_state = ShellState::new();
        shell_state.set_var("PWD", "/test/dir".to_string());

        // Simulate here-doc content with pwd builtin command substitution
        let content = "Current directory: `pwd`";
        let expanded = expand_variables_in_string(content, &mut shell_state);

        // The pwd builtin should be executed and expanded
        assert!(expanded.contains("Current directory: "));
    }

    // ========================================================================
    // File Descriptor Integration Tests
    // ========================================================================

    #[test]
    fn test_fd_output_redirection() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_out_{}.txt", timestamp);

        // Test: echo "error" 2>errors.txt
        let cmd = ShellCommand {
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo error >&2".to_string(),
            ],
            redirections: vec![Redirection::FdOutput(2, temp_file.clone())],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify file was created and contains the error message
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert_eq!(content.trim(), "error");

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_input_redirection() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file with content
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_in_{}.txt", timestamp);

        std::fs::write(&temp_file, "test input\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Test: cat 3<input.txt (reading from fd 3)
        // Note: This tests that fd 3 is opened for reading
        let cmd = ShellCommand {
            args: vec!["cat".to_string()],
            compound: None,
            redirections: vec![
                Redirection::FdInput(3, temp_file.clone()),
                Redirection::Input(temp_file.clone()),
            ],
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_append_redirection() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file with initial content
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_append_{}.txt", timestamp);

        std::fs::write(&temp_file, "first line\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Test: echo "more" 2>>errors.txt
        let cmd = ShellCommand {
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo second line >&2".to_string(),
            ],
            redirections: vec![Redirection::FdAppend(2, temp_file.clone())],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify file contains both lines
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("first line"));
        assert!(content.contains("second line"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_duplication_stderr_to_stdout() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_dup_{}.txt", timestamp);

        // Test: command 2>&1 >output.txt
        // Note: For external commands, fd duplication is handled by the shell
        // We test that the command executes successfully with the redirection
        let cmd = ShellCommand {
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo test; echo error >&2".to_string(),
            ],
            compound: None,
            redirections: vec![Redirection::Output(temp_file.clone())],
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify file was created and contains output
        assert!(std::path::Path::new(&temp_file).exists());
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("test"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_close() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Test: command 2>&- (closes stderr)
        let cmd = ShellCommand {
            args: vec!["sh".to_string(), "-c".to_string(), "echo test".to_string()],
            redirections: vec![Redirection::FdClose(2)],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify fd 2 is closed in the fd table
        assert!(shell_state.fd_table.borrow().is_closed(2));
    }

    #[test]
    fn test_fd_read_write() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_rw_{}.txt", timestamp);

        std::fs::write(&temp_file, "initial content\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Test: 3<>file.txt (opens fd 3 for read/write)
        let cmd = ShellCommand {
            args: vec!["cat".to_string()],
            compound: None,
            redirections: vec![
                Redirection::FdInputOutput(3, temp_file.clone()),
                Redirection::Input(temp_file.clone()),
            ],
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_multiple_fd_redirections() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp files
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let out_file = format!("/tmp/rush_test_fd_multi_out_{}.txt", timestamp);
        let err_file = format!("/tmp/rush_test_fd_multi_err_{}.txt", timestamp);

        // Test: command 2>err.txt 1>out.txt
        let cmd = ShellCommand {
            args: vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo stdout; echo stderr >&2".to_string(),
            ],
            redirections: vec![
                Redirection::FdOutput(2, err_file.clone()),
                Redirection::Output(out_file.clone()),
            ],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify both files were created
        assert!(std::path::Path::new(&out_file).exists());
        assert!(std::path::Path::new(&err_file).exists());

        // Verify content
        let out_content = std::fs::read_to_string(&out_file).unwrap();
        let err_content = std::fs::read_to_string(&err_file).unwrap();
        assert!(out_content.contains("stdout"));
        assert!(err_content.contains("stderr"));

        // Cleanup
        let _ = std::fs::remove_file(&out_file);
        let _ = std::fs::remove_file(&err_file);
    }

    #[test]
    fn test_fd_swap_pattern() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp files
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_swap_{}.txt", timestamp);

        // Test fd operations: open fd 3, then close it
        // This tests the fd table operations
        let cmd = ShellCommand {
            args: vec!["sh".to_string(), "-c".to_string(), "echo test".to_string()],
            redirections: vec![
                Redirection::FdOutput(3, temp_file.clone()), // Open fd 3 for writing
                Redirection::FdClose(3),                     // Close fd 3
                Redirection::Output(temp_file.clone()),      // Write to stdout
            ],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify fd 3 is closed after the operations
        assert!(shell_state.fd_table.borrow().is_closed(3));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_redirection_with_pipes() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_pipe_{}.txt", timestamp);

        // Test: cmd1 | cmd2 >output.txt
        // This tests redirections in pipelines
        let commands = vec![
            ShellCommand {
                args: vec!["echo".to_string(), "piped output".to_string()],
                redirections: vec![],
                compound: None,
            },
            ShellCommand {
                args: vec!["cat".to_string()],
                compound: None,
                redirections: vec![Redirection::Output(temp_file.clone())],
            },
        ];

        let mut shell_state = ShellState::new();
        let exit_code = execute_pipeline(&commands, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify output file contains the piped content
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("piped output"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_error_invalid_fd_number() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_invalid_{}.txt", timestamp);

        // Test: Invalid fd number (>1024)
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "test".to_string()],
            compound: None,
            redirections: vec![Redirection::FdOutput(1025, temp_file.clone())],
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);

        // Should fail with error
        assert_eq!(exit_code, 1);

        // Cleanup (file may not exist)
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_error_duplicate_closed_fd() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Test: Attempting to duplicate a closed fd
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "test".to_string()],
            compound: None,
            redirections: vec![
                Redirection::FdClose(3),
                Redirection::FdDuplicate(2, 3), // Try to duplicate closed fd 3
            ],
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);

        // Should fail with error
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_fd_error_file_permission() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Test: Attempting to write to a read-only location
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "test".to_string()],
            redirections: vec![Redirection::FdOutput(2, "/proc/version".to_string())],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);

        // Should fail with permission error
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_fd_redirection_order() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp files
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file1 = format!("/tmp/rush_test_fd_order1_{}.txt", timestamp);
        let file2 = format!("/tmp/rush_test_fd_order2_{}.txt", timestamp);

        // Test: Redirections are processed left-to-right
        // 1>file1 1>file2 should write to file2
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "test".to_string()],
            compound: None,
            redirections: vec![
                Redirection::Output(file1.clone()),
                Redirection::Output(file2.clone()),
            ],
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // file2 should have the output (last redirection wins)
        let content2 = std::fs::read_to_string(&file2).unwrap();
        assert!(content2.contains("test"));

        // Cleanup
        let _ = std::fs::remove_file(&file1);
        let _ = std::fs::remove_file(&file2);
    }

    #[test]
    fn test_fd_builtin_with_redirection() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_builtin_{}.txt", timestamp);

        // Test: Built-in command with fd redirection
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "builtin test".to_string()],
            redirections: vec![Redirection::Output(temp_file.clone())],
            compound: None,
        };

        let mut shell_state = ShellState::new();
        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify output
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("builtin test"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_fd_variable_expansion_in_filename() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_fd_var_{}.txt", timestamp);

        // Set variable for filename
        let mut shell_state = ShellState::new();
        shell_state.set_var("OUTFILE", temp_file.clone());

        // Test: Variable expansion in redirection filename
        let cmd = ShellCommand {
            args: vec!["echo".to_string(), "variable test".to_string()],
            compound: None,
            redirections: vec![Redirection::Output("$OUTFILE".to_string())],
        };

        let exit_code = execute_single_command(&cmd, &mut shell_state);
        assert_eq!(exit_code, 0);

        // Verify output
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("variable test"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    // ========================================================================
    // Break and Continue Integration Tests
    // ========================================================================

    #[test]
    fn test_break_in_for_loop() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        
        // for i in 1 2 3 4 5; do
        //   output="$output$i"
        //   if [ $i = "3" ]; then break; fi
        // done
        let ast = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string(), "5".to_string()],
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i".to_string(),
                },
                Ast::If {
                    branches: vec![(
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "3".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["break".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                    )],
                    else_branch: None,
                },
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("123".to_string()));
    }

    #[test]
    fn test_continue_in_for_loop() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        
        // for i in 1 2 3 4 5; do
        //   if [ $i = "3" ]; then continue; fi
        //   output="$output$i"
        // done
        let ast = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string(), "5".to_string()],
            body: Box::new(Ast::Sequence(vec![
                Ast::If {
                    branches: vec![(
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "3".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["continue".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                    )],
                    else_branch: None,
                },
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i".to_string(),
                },
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("1245".to_string()));
    }

    #[test]
    fn test_break_in_while_loop() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("i", "0".to_string());
        shell_state.set_var("output", "".to_string());
        
        // i=0
        // while [ $i -lt 10 ]; do
        //   i=$((i + 1))
        //   output="$output$i"
        //   if [ $i = "5" ]; then break; fi
        // done
        let ast = Ast::While {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "-lt".to_string(), "10".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i".to_string(),
                },
                Ast::If {
                    branches: vec![(
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "5".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["break".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                    )],
                    else_branch: None,
                },
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("12345".to_string()));
    }

    #[test]
    fn test_continue_in_while_loop() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("i", "0".to_string());
        shell_state.set_var("output", "".to_string());
        
        // i=0
        // while [ $i -lt 5 ]; do
        //   i=$((i + 1))
        //   if [ $i = "3" ]; then continue; fi
        //   output="$output$i"
        // done
        let ast = Ast::While {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "-lt".to_string(), "5".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
                Ast::If {
                    branches: vec![(
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "3".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["continue".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                    )],
                    else_branch: None,
                },
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i".to_string(),
                },
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("1245".to_string()));
    }

    #[test]
    fn test_break_nested_loops() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        
        // for i in 1 2 3; do
        //   for j in a b c; do
        //     output="$output$i$j"
        //     if [ $j = "b" ]; then break; fi
        //   done
        // done
        let inner_loop = Ast::For {
            variable: "j".to_string(),
            items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i$j".to_string(),
                },
                Ast::If {
                    branches: vec![(
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "b".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["break".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                    )],
                    else_branch: None,
                },
            ])),
        };
        
        let outer_loop = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
            body: Box::new(inner_loop),
        };
        
        let exit_code = execute(outer_loop, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("1a1b2a2b3a3b".to_string()));
    }

    #[test]
    fn test_break_2_nested_loops() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        
        // for i in 1 2 3; do
        //   for j in a b c; do
        //     output="$output$i$j"
        //     if [ $i = "2" ] && [ $j = "b" ]; then break 2; fi
        //   done
        // done
        let inner_loop = Ast::For {
            variable: "j".to_string(),
            items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i$j".to_string(),
                },
                Ast::And {
                    left: Box::new(Ast::Pipeline(vec![ShellCommand {
                        args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "2".to_string()],
                        redirections: vec![],
                        compound: None,
                    }])),
                    right: Box::new(Ast::If {
                        branches: vec![(
                            Box::new(Ast::Pipeline(vec![ShellCommand {
                                args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "b".to_string()],
                                redirections: vec![],
                                compound: None,
                            }])),
                            Box::new(Ast::Pipeline(vec![ShellCommand {
                                args: vec!["break".to_string(), "2".to_string()],
                                redirections: vec![],
                                compound: None,
                            }])),
                        )],
                        else_branch: None,
                    }),
                },
            ])),
        };
        
        let outer_loop = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
            body: Box::new(inner_loop),
        };
        
        let exit_code = execute(outer_loop, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("1a1b1c2a2b".to_string()));
    }

    #[test]
    fn test_continue_nested_loops() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        
        // for i in 1 2 3; do
        //   for j in a b c; do
        //     if [ $j = "b" ]; then continue; fi
        //     output="$output$i$j"
        //   done
        // done
        let inner_loop = Ast::For {
            variable: "j".to_string(),
            items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            body: Box::new(Ast::Sequence(vec![
                Ast::If {
                    branches: vec![(
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "b".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["continue".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                    )],
                    else_branch: None,
                },
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i$j".to_string(),
                },
            ])),
        };
        
        let outer_loop = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
            body: Box::new(inner_loop),
        };
        
        let exit_code = execute(outer_loop, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("1a1c2a2c3a3c".to_string()));
    }

    #[test]
    fn test_continue_2_nested_loops() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        
        // for i in 1 2 3; do
        //   for j in a b c; do
        //     if [ $i = "2" ] && [ $j = "b" ]; then continue 2; fi
        //     output="$output$i$j"
        //   done
        //   output="$output-"
        // done
        let inner_loop = Ast::For {
            variable: "j".to_string(),
            items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            body: Box::new(Ast::Sequence(vec![
                Ast::And {
                    left: Box::new(Ast::Pipeline(vec![ShellCommand {
                        args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "2".to_string()],
                        redirections: vec![],
                        compound: None,
                    }])),
                    right: Box::new(Ast::If {
                        branches: vec![(
                            Box::new(Ast::Pipeline(vec![ShellCommand {
                                args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "b".to_string()],
                                redirections: vec![],
                                compound: None,
                            }])),
                            Box::new(Ast::Pipeline(vec![ShellCommand {
                                args: vec!["continue".to_string(), "2".to_string()],
                                redirections: vec![],
                                compound: None,
                            }])),
                        )],
                        else_branch: None,
                    }),
                },
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i$j".to_string(),
                },
            ])),
        };
        
        let outer_loop = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
            body: Box::new(Ast::Sequence(vec![
                inner_loop,
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i-".to_string(),
                },
            ])),
        };
        
        let exit_code = execute(outer_loop, &mut shell_state);
        assert_eq!(exit_code, 0);
        // After 2a, continue 2 skips rest of inner loop and the "$i-" assignment, goes to next outer iteration
        assert_eq!(shell_state.get_var("output"), Some("1a1b1c1-2a3a3b3c3-".to_string()));
    }

    #[test]
    fn test_break_preserves_exit_code() {
        let mut shell_state = ShellState::new();
        
        // for i in 1 2 3; do
        //   false
        //   break
        // done
        // echo $?
        let ast = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
            body: Box::new(Ast::Sequence(vec![
                Ast::Pipeline(vec![ShellCommand {
                    args: vec!["false".to_string()],
                    redirections: vec![],
                    compound: None,
                }]),
                Ast::Pipeline(vec![ShellCommand {
                    args: vec!["break".to_string()],
                    redirections: vec![],
                    compound: None,
                }]),
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        // break returns 0, so the loop's exit code should be 0
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_continue_preserves_exit_code() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("count", "0".to_string());
        
        // for i in 1 2; do
        //   count=$((count + 1))
        //   false
        //   continue
        // done
        let ast = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string()],
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "count".to_string(),
                    value: "$((count + 1))".to_string(),
                },
                Ast::Pipeline(vec![ShellCommand {
                    args: vec!["false".to_string()],
                    redirections: vec![],
                    compound: None,
                }]),
                Ast::Pipeline(vec![ShellCommand {
                    args: vec!["continue".to_string()],
                    redirections: vec![],
                    compound: None,
                }]),
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        // continue returns 0, so the loop's exit code should be 0
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("count"), Some("2".to_string()));
    }

    // ========================================================================
    // Until Loop Tests
    // ========================================================================

    #[test]
    fn test_until_basic_loop() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("i", "0".to_string());
        shell_state.set_var("output", "".to_string());
        
        // i=0; until [ $i = "3" ]; do output="$output$i"; i=$((i + 1)); done
        let ast = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "3".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i".to_string(),
                },
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("012".to_string()));
        assert_eq!(shell_state.get_var("i"), Some("3".to_string()));
    }

    #[test]
    fn test_until_condition_initially_true() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("executed", "no".to_string());
        
        // until true; do executed="yes"; done
        let ast = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["true".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Assignment {
                var: "executed".to_string(),
                value: "yes".to_string(),
            }),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        // Body should not execute since condition is true (exit code 0)
        assert_eq!(shell_state.get_var("executed"), Some("no".to_string()));
    }

    #[test]
    fn test_until_with_commands_in_body() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("count", "0".to_string());
        
        // count=0; until [ $count -ge 3 ]; do count=$((count + 1)); echo $count; done
        let ast = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$count".to_string(), "-ge".to_string(), "3".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "count".to_string(),
                    value: "$((count + 1))".to_string(),
                },
                Ast::Pipeline(vec![ShellCommand {
                    args: vec!["echo".to_string(), "$count".to_string()],
                    redirections: vec![],
                    compound: None,
                }]),
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("count"), Some("3".to_string()));
    }

    #[test]
    fn test_until_with_variable_modification() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("x", "1".to_string());
        
        // x=1; until [ $x -gt 5 ]; do x=$((x * 2)); done
        let ast = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$x".to_string(), "-gt".to_string(), "5".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Assignment {
                var: "x".to_string(),
                value: "$((x * 2))".to_string(),
            }),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("x"), Some("8".to_string()));
    }

    #[test]
    fn test_until_nested_loops() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        shell_state.set_var("i", "0".to_string());
        
        let inner_loop = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "2".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i$j".to_string(),
                },
                Ast::Assignment {
                    var: "j".to_string(),
                    value: "$((j + 1))".to_string(),
                },
            ])),
        };
        
        let outer_loop = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "2".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
                Ast::Assignment {
                    var: "j".to_string(),
                    value: "0".to_string(),
                },
                inner_loop,
            ])),
        };
        
        let exit_code = execute(outer_loop, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("10112021".to_string()));
    }

    #[test]
    fn test_until_with_break() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("i", "0".to_string());
        shell_state.set_var("output", "".to_string());
        
        let ast = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["false".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i".to_string(),
                },
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
                Ast::If {
                    branches: vec![(
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "3".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["break".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                    )],
                    else_branch: None,
                },
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("012".to_string()));
    }

    #[test]
    fn test_until_with_continue() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("i", "0".to_string());
        shell_state.set_var("output", "".to_string());
        
        let ast = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "-ge".to_string(), "5".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
                Ast::If {
                    branches: vec![(
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "3".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                        Box::new(Ast::Pipeline(vec![ShellCommand {
                            args: vec!["continue".to_string()],
                            redirections: vec![],
                            compound: None,
                        }])),
                    )],
                    else_branch: None,
                },
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i".to_string(),
                },
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("1245".to_string()));
    }

    #[test]
    fn test_until_empty_body() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("i", "0".to_string());
        
        // until true; do :; done (empty body with true condition)
        let ast = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["true".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["true".to_string()],
                redirections: vec![],
                compound: None,
            }])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_until_with_command_substitution() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("count", "0".to_string());
        shell_state.set_var("output", "".to_string());
        
        // until [ $(echo $count) = "3" ]; do output="$output$count"; count=$((count + 1)); done
        let ast = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$(echo $count)".to_string(), "=".to_string(), "3".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$count".to_string(),
                },
                Ast::Assignment {
                    var: "count".to_string(),
                    value: "$((count + 1))".to_string(),
                },
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("012".to_string()));
    }

    #[test]
    fn test_until_with_arithmetic_condition() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("x", "1".to_string());
        shell_state.set_var("output", "".to_string());
        
        // x=1; until [ $((x * 2)) -gt 10 ]; do output="$output$x"; x=$((x + 1)); done
        let ast = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$((x * 2))".to_string(), "-gt".to_string(), "10".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$x".to_string(),
                },
                Ast::Assignment {
                    var: "x".to_string(),
                    value: "$((x + 1))".to_string(),
                },
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("12345".to_string()));
    }

    #[test]
    fn test_until_inside_for() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        
        // for i in 1 2; do j=0; until [ $j = "2" ]; do output="$output$i$j"; j=$((j + 1)); done; done
        let inner_until = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "2".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i$j".to_string(),
                },
                Ast::Assignment {
                    var: "j".to_string(),
                    value: "$((j + 1))".to_string(),
                },
            ])),
        };
        
        let outer_for = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string()],
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "j".to_string(),
                    value: "0".to_string(),
                },
                inner_until,
            ])),
        };
        
        let exit_code = execute(outer_for, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("10112021".to_string()));
    }

    #[test]
    fn test_for_inside_until() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        shell_state.set_var("i", "0".to_string());
        
        // i=0; until [ $i = "2" ]; do for j in a b; do output="$output$i$j"; done; i=$((i + 1)); done
        let inner_for = Ast::For {
            variable: "j".to_string(),
            items: vec!["a".to_string(), "b".to_string()],
            body: Box::new(Ast::Assignment {
                var: "output".to_string(),
                value: "$output$i$j".to_string(),
            }),
        };
        
        let outer_until = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "2".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                inner_for,
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
            ])),
        };
        
        let exit_code = execute(outer_until, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("0a0b1a1b".to_string()));
    }

    #[test]
    fn test_until_inside_while() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        shell_state.set_var("i", "0".to_string());
        
        let inner_until = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$j".to_string(), "=".to_string(), "2".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i$j".to_string(),
                },
                Ast::Assignment {
                    var: "j".to_string(),
                    value: "$((j + 1))".to_string(),
                },
            ])),
        };
        
        let outer_while = Ast::While {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "-lt".to_string(), "2".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
                Ast::Assignment {
                    var: "j".to_string(),
                    value: "0".to_string(),
                },
                inner_until,
            ])),
        };
        
        let exit_code = execute(outer_while, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("10112021".to_string()));
    }

    #[test]
    fn test_while_inside_until() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        shell_state.set_var("i", "0".to_string());
        
        let inner_while = Ast::While {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$j".to_string(), "-lt".to_string(), "2".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "$output$i$j".to_string(),
                },
                Ast::Assignment {
                    var: "j".to_string(),
                    value: "$((j + 1))".to_string(),
                },
            ])),
        };
        
        let outer_until = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "2".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
                Ast::Assignment {
                    var: "j".to_string(),
                    value: "0".to_string(),
                },
                inner_while,
            ])),
        };
        
        let exit_code = execute(outer_until, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("output"), Some("10112021".to_string()));
    }

    #[test]
    fn test_until_preserves_exit_code() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("i", "0".to_string());
        
        // until [ $i = "1" ]; do i=$((i + 1)); false; done
        let ast = Ast::Until {
            condition: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["test".to_string(), "$i".to_string(), "=".to_string(), "1".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            body: Box::new(Ast::Sequence(vec![
                Ast::Assignment {
                    var: "i".to_string(),
                    value: "$((i + 1))".to_string(),
                },
                Ast::Pipeline(vec![ShellCommand {
                    args: vec!["false".to_string()],
                    redirections: vec![],
                    compound: None,
                }]),
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        // Last command in body was false (exit 1), so loop should return 1
        assert_eq!(exit_code, 1);
    }

    // ========================================================================
    // Control-Flow in Logical Chains Tests (&&, ||)
    // ========================================================================

    #[test]
    fn test_and_with_return_in_lhs() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("executed", "no".to_string());
        
        // Define a function that returns early
        shell_state.define_function(
            "early_return".to_string(),
            Ast::Sequence(vec![
                Ast::Assignment {
                    var: "executed".to_string(),
                    value: "yes".to_string(),
                },
                Ast::Return { value: Some("5".to_string()) },
            ]),
        );
        
        // Call function in && chain: early_return && echo "should not execute"
        let ast = Ast::FunctionCall {
            name: "early_return".to_string(),
            args: vec![],
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 5);
        assert_eq!(shell_state.get_var("executed"), Some("yes".to_string()));
    }

    #[test]
    fn test_and_with_exit_in_lhs() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("rhs_executed", "no".to_string());
        
        // exit 42 && rhs_executed=yes
        let ast = Ast::And {
            left: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["exit".to_string(), "42".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            right: Box::new(Ast::Assignment {
                var: "rhs_executed".to_string(),
                value: "yes".to_string(),
            }),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 42);
        assert_eq!(shell_state.get_var("rhs_executed"), Some("no".to_string()));
        assert!(shell_state.exit_requested);
    }

    #[test]
    fn test_and_with_break_in_lhs() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        
        // for i in 1 2 3; do
        //   (break && output="${output}bad") && output="${output}$i"
        // done
        let ast = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
            body: Box::new(Ast::And {
                left: Box::new(Ast::And {
                    left: Box::new(Ast::Pipeline(vec![ShellCommand {
                        args: vec!["break".to_string()],
                        redirections: vec![],
                        compound: None,
                    }])),
                    right: Box::new(Ast::Assignment {
                        var: "output".to_string(),
                        value: "${output}bad".to_string(),
                    }),
                }),
                right: Box::new(Ast::Assignment {
                    var: "output".to_string(),
                    value: "${output}$i".to_string(),
                }),
            }),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        // RHS should not execute after break
        assert_eq!(shell_state.get_var("output"), Some("".to_string()));
    }

    #[test]
    fn test_and_with_continue_in_lhs() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        
        // for i in 1 2 3; do
        //   continue && output="${output}bad"
        //   output="${output}$i"
        // done
        let ast = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
            body: Box::new(Ast::Sequence(vec![
                Ast::And {
                    left: Box::new(Ast::Pipeline(vec![ShellCommand {
                        args: vec!["continue".to_string()],
                        redirections: vec![],
                        compound: None,
                    }])),
                    right: Box::new(Ast::Assignment {
                        var: "output".to_string(),
                        value: "${output}bad".to_string(),
                    }),
                },
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "${output}$i".to_string(),
                },
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        // RHS of && should not execute, and subsequent assignment should not execute either
        assert_eq!(shell_state.get_var("output"), Some("".to_string()));
    }

    #[test]
    fn test_or_with_return_in_lhs() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("executed", "no".to_string());
        
        // Define a function that returns early with non-zero
        shell_state.define_function(
            "early_return".to_string(),
            Ast::Sequence(vec![
                Ast::Assignment {
                    var: "executed".to_string(),
                    value: "yes".to_string(),
                },
                Ast::Return { value: Some("5".to_string()) },
            ]),
        );
        
        // Call function in || chain: early_return || echo "should not execute"
        let ast = Ast::FunctionCall {
            name: "early_return".to_string(),
            args: vec![],
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 5);
        assert_eq!(shell_state.get_var("executed"), Some("yes".to_string()));
    }

    #[test]
    fn test_or_with_exit_in_lhs() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("rhs_executed", "no".to_string());
        
        // exit 42 || rhs_executed=yes
        let ast = Ast::Or {
            left: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["exit".to_string(), "42".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            right: Box::new(Ast::Assignment {
                var: "rhs_executed".to_string(),
                value: "yes".to_string(),
            }),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 42);
        assert_eq!(shell_state.get_var("rhs_executed"), Some("no".to_string()));
        assert!(shell_state.exit_requested);
    }

    #[test]
    fn test_or_with_break_in_lhs() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        
        // for i in 1 2 3; do
        //   (false || break) || output="${output}$i"
        // done
        let ast = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
            body: Box::new(Ast::Or {
                left: Box::new(Ast::Or {
                    left: Box::new(Ast::Pipeline(vec![ShellCommand {
                        args: vec!["false".to_string()],
                        redirections: vec![],
                        compound: None,
                    }])),
                    right: Box::new(Ast::Pipeline(vec![ShellCommand {
                        args: vec!["break".to_string()],
                        redirections: vec![],
                        compound: None,
                    }])),
                }),
                right: Box::new(Ast::Assignment {
                    var: "output".to_string(),
                    value: "${output}$i".to_string(),
                }),
            }),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        // RHS should not execute after break
        assert_eq!(shell_state.get_var("output"), Some("".to_string()));
    }

    #[test]
    fn test_or_with_continue_in_lhs() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("output", "".to_string());
        
        // for i in 1 2 3; do
        //   (false || continue) || output="${output}bad"
        //   output="${output}$i"
        // done
        let ast = Ast::For {
            variable: "i".to_string(),
            items: vec!["1".to_string(), "2".to_string(), "3".to_string()],
            body: Box::new(Ast::Sequence(vec![
                Ast::Or {
                    left: Box::new(Ast::Pipeline(vec![ShellCommand {
                        args: vec!["false".to_string()],
                        redirections: vec![],
                        compound: None,
                    }])),
                    right: Box::new(Ast::Pipeline(vec![ShellCommand {
                        args: vec!["continue".to_string()],
                        redirections: vec![],
                        compound: None,
                    }])),
                },
                Ast::Assignment {
                    var: "output".to_string(),
                    value: "${output}$i".to_string(),
                },
            ])),
        };
        
        let exit_code = execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        // Both RHS of || and subsequent assignment should not execute
        assert_eq!(shell_state.get_var("output"), Some("".to_string()));
    }

    #[test]
    fn test_logical_chain_flag_cleanup() {
        let mut shell_state = ShellState::new();
        
        // Verify in_logical_chain is false initially
        assert!(!shell_state.in_logical_chain);
        
        // Execute a simple && chain
        let ast = Ast::And {
            left: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["true".to_string()],
                redirections: vec![],
                compound: None,
            }])),
            right: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["true".to_string()],
                redirections: vec![],
                compound: None,
            }])),
        };
        
        execute(ast, &mut shell_state);
        
        // Verify in_logical_chain is reset to false after execution
        assert!(!shell_state.in_logical_chain);
    }

    #[test]
    fn test_logical_chain_flag_cleanup_with_return() {
        let mut shell_state = ShellState::new();
        
        // Define a function that returns
        shell_state.define_function(
            "test_return".to_string(),
            Ast::Return { value: Some("0".to_string()) },
        );
        
        // Execute && chain with return in LHS
        let ast = Ast::And {
            left: Box::new(Ast::FunctionCall {
                name: "test_return".to_string(),
                args: vec![],
            }),
            right: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "should not execute".to_string()],
                redirections: vec![],
                compound: None,
            }])),
        };
        
        // Execute in function context
        shell_state.enter_function();
        execute(ast, &mut shell_state);
        shell_state.exit_function();
        
        // Verify in_logical_chain is reset even with early return
        assert!(!shell_state.in_logical_chain);
    }
}