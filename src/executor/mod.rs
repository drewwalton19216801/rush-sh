//! Command execution engine for the Rush shell.
//!
//! This module handles the execution of parsed AST nodes, including pipelines,
//! control structures, redirections, and built-in commands.


use super::parser::Ast;
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

                // Save current line number and reset to 1 for function body
                shell_state.line_number_stack.push(shell_state.current_line_number);
                shell_state.current_line_number = 1;

                // Execute function body
                let exit_code = execute(function_body, shell_state);

                // Helper to restore line number from stack
                let restore_line_number = |state: &mut ShellState| {
                    if let Some(saved_line) = state.line_number_stack.pop() {
                        state.current_line_number = saved_line;
                    }
                };

                // Check if we got an early return from the function
                if shell_state.is_returning() {
                    let return_value = shell_state.get_return_value().unwrap_or(0);

                    // Restore old positional parameters
                    shell_state.set_positional_params(old_positional);

                    // Restore line number
                    restore_line_number(shell_state);

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

                // Restore line number
                if let Some(saved_line) = shell_state.line_number_stack.pop() {
                    shell_state.current_line_number = saved_line;
                }

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
mod tests;