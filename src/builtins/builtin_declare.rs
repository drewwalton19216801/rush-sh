use std::io::Write;

use crate::parser::Ast;
use crate::state::ShellState;

pub struct DeclareBuiltin;

impl super::Builtin for DeclareBuiltin {
    fn name(&self) -> &'static str {
        "declare"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Display function definitions or list function names"
    }

    fn run(
        &self,
        cmd: &crate::parser::ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        // Check if we have the -f flag (function introspection)
        if cmd.args.len() >= 2 && cmd.args[1] == "-f" {
            if cmd.args.len() == 2 {
                // declare -f: list all function names
                let function_names: Vec<&String> = shell_state.get_function_names();
                for name in function_names {
                    if shell_state.colors_enabled {
                        let _ = writeln!(
                            output_writer,
                            "{}{}\x1b[0m",
                            shell_state.color_scheme.builtin, name
                        );
                    } else {
                        let _ = writeln!(output_writer, "{}", name);
                    }
                }
                0
            } else {
                // declare -f function_name: show specific function definition
                let func_name = &cmd.args[2];

                if let Some(function_ast) = shell_state.get_function(func_name) {
                    // Convert function back to shell syntax for display
                    let definition = format_function_definition(func_name, function_ast);

                    if shell_state.colors_enabled {
                        let _ = writeln!(
                            output_writer,
                            "{}{}\x1b[0m",
                            shell_state.color_scheme.success, definition
                        );
                    } else {
                        let _ = writeln!(output_writer, "{}", definition);
                    }
                    0
                } else {
                    if shell_state.colors_enabled {
                        let _ = writeln!(
                            output_writer,
                            "{}declare: function 'declare: function '{}' not found\x1b[0m",
                            shell_state.color_scheme.error, func_name
                        );
                    } else {
                        let _ =
                            writeln!(output_writer, "declare: function '{}' not found", func_name);
                    }
                    1
                }
            }
        } else if cmd.args.len() >= 2 && cmd.args[1] == "-F" {
            // declare -F: list all function names (same as -f for now)
            let function_names: Vec<&String> = shell_state.get_function_names();
            for name in function_names {
                if shell_state.colors_enabled {
                    let _ = writeln!(
                        output_writer,
                        "{}{}\x1b[0m",
                        shell_state.color_scheme.builtin, name
                    );
                } else {
                    let _ = writeln!(output_writer, "{}", name);
                }
            }
            0
        } else {
            // declare without -f flag or with other arguments
            if shell_state.colors_enabled {
                let _ = writeln!(
                    output_writer,
                    "{}declare: use 'declare -f' to list functions or 'declare -f <name>' to show function definition\x1b[0m",
                    shell_state.color_scheme.error
                );
            } else {
                let _ = writeln!(
                    output_writer,
                    "declare: use 'declare -f' to list functions or 'declare -f <name>' to show function definition"
                );
            }
            1
        }
    }
}

/// Convert a function AST back to shell syntax for display
fn format_function_definition(name: &str, ast: &Ast) -> String {
    format!("{}() {{\n    {}\n}}", name, format_ast_body(ast, 1))
}

/// Format an AST node into an indented shell-like string.
///
/// The `indent_level` controls indentation depth in multiples of four spaces.
///
/// # Examples
///
/// ```
/// // Note: format_ast_body is a private function
/// // This example is for documentation only
/// ```
fn format_ast_body(ast: &Ast, indent_level: usize) -> String {
    let indent = "    ".repeat(indent_level);

    match ast {
        Ast::Pipeline(commands) => {
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
        Ast::Sequence(asts) => {
            let mut result = String::new();
            for (i, ast_node) in asts.iter().enumerate() {
                if i > 0 {
                    result.push_str(";\n");
                }
                result.push_str(&format_ast_body(ast_node, indent_level));
            }
            result
        }
        Ast::Assignment { var, value } => {
            format!("{} {}={}", indent, var, value)
        }
        Ast::LocalAssignment { var, value } => {
            format!("{} local {}={}", indent, var, value)
        }
        Ast::If {
            branches,
            else_branch,
        } => {
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
        Ast::Case {
            word,
            cases,
            default,
        } => {
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
        Ast::For {
            variable,
            items,
            body,
        } => {
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
        Ast::While { condition, body } => {
            let mut result = String::new();
            result.push_str(&format!("{}while ", indent));
            result.push_str(&format_ast_body(condition, 0));
            result.push_str("; do\n");
            result.push_str(&format_ast_body(body, indent_level + 1));
            result.push_str(&format!("\n{}done", indent));
            result
        }
        Ast::Until { condition, body } => {
            let mut result = String::new();
            result.push_str(&format!("{}until ", indent));
            result.push_str(&format_ast_body(condition, 0));
            result.push_str("; do\n");
            result.push_str(&format_ast_body(body, indent_level + 1));
            result.push_str(&format!("\n{}done", indent));
            result
        }
        Ast::FunctionDefinition { name, body } => {
            format!(
                "{}() {{\n{}    {}\n{}}}\n",
                name,
                indent,
                format_ast_body(body, indent_level),
                indent
            )
        }
        Ast::FunctionCall { name, args } => {
            if args.is_empty() {
                format!("{} {}", indent, name)
            } else {
                format!("{} {} {}", indent, name, args.join(" "))
            }
        }
        Ast::Return { value } => {
            if let Some(val) = value {
                format!("{} return {}", indent, val)
            } else {
                format!("{} return", indent)
            }
        }
        Ast::And { left, right } => {
            format!(
                "{} && {}",
                format_ast_body(left, 0).trim(),
                format_ast_body(right, 0).trim()
            )
        }
        Ast::Or { left, right } => {
            format!(
                "{} || {}",
                format_ast_body(left, 0).trim(),
                format_ast_body(right, 0).trim()
            )
        }
        Ast::Subshell { body } => {
            format!("({})", format_ast_body(body, 0).trim())
        }
        Ast::CommandGroup { body } => {
            format!("{{ {}; }}", format_ast_body(body, 0).trim())
        }
        Ast::Negation { command } => {
            format!("! {}", format_ast_body(command, 0).trim())
        }
    }
}

/// Format a ShellCommand into a single-line shell syntax string.
///
/// Joins command arguments with spaces and appends any redirections using
/// their shell operators (e.g. `>`, `>>`, `<`, `<<`, `>|`, `<<<`, and fd-style forms).
///
/// # Examples
///
/// ```
/// // Note: format_command is a private function
/// // This example is for documentation only
/// ```
fn format_command(cmd: &crate::parser::ShellCommand) -> String {
    let mut result = cmd.args.join(" ");

    // Format redirections
    for redir in &cmd.redirections {
        use crate::parser::Redirection;
        match redir {
            Redirection::Input(file) => result.push_str(&format!(" < {}", file)),
            Redirection::Output(file) => result.push_str(&format!(" > {}", file)),
            Redirection::OutputClobber(file) => result.push_str(&format!(" >| {}", file)),
            Redirection::Append(file) => result.push_str(&format!(" >> {}", file)),
            Redirection::FdInput(fd, file) => result.push_str(&format!(" {}<{}", fd, file)),
            Redirection::FdOutput(fd, file) => result.push_str(&format!(" {}>{}", fd, file)),
            Redirection::FdOutputClobber(fd, file) => result.push_str(&format!(" {}>|{}", fd, file)),
            Redirection::FdAppend(fd, file) => result.push_str(&format!(" {}>>{}", fd, file)),
            Redirection::FdDuplicate(from, to) => result.push_str(&format!(" {}>&{}", from, to)),
            Redirection::FdClose(fd) => result.push_str(&format!(" {}>&-", fd)),
            Redirection::FdInputOutput(fd, file) => result.push_str(&format!(" {}<>{}", fd, file)),
            Redirection::HereDoc(delim, _) => result.push_str(&format!(" << {}", delim)),
            Redirection::HereString(content) => result.push_str(&format!(" <<< {}", content)),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;

    #[test]
    fn test_declare_builtin_run_list_functions() {
        let cmd = crate::parser::ShellCommand {
            args: vec!["declare".to_string(), "-f".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.colors_enabled = false;

        // Define a test function
        shell_state.define_function(
            "test_func".to_string(),
            Ast::Pipeline(vec![crate::parser::ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        );

        let builtin = DeclareBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("test_func"));
    }

    #[test]
    fn test_declare_builtin_run_show_function() {
        let cmd = crate::parser::ShellCommand {
            args: vec![
                "declare".to_string(),
                "-f".to_string(),
                "test_func".to_string(),
            ],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.colors_enabled = false;

        // Define a test function
        shell_state.define_function(
            "test_func".to_string(),
            Ast::Pipeline(vec![crate::parser::ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        );

        let builtin = DeclareBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("test_func"));
        assert!(output_str.contains("echo"));
        assert!(output_str.contains("hello"));
    }

    #[test]
    fn test_declare_builtin_run_nonexistent_function() {
        let cmd = crate::parser::ShellCommand {
            args: vec![
                "declare".to_string(),
                "-f".to_string(),
                "nonexistent".to_string(),
            ],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.colors_enabled = false;

        let builtin = DeclareBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("function 'nonexistent' not found"));
    }

    #[test]
    fn test_declare_builtin_run_invalid_option() {
        let cmd = crate::parser::ShellCommand {
            args: vec!["declare".to_string(), "-x".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.colors_enabled = false;

        let builtin = DeclareBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("use 'declare -f'"));
    }
}