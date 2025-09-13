use std::fs;
use std::io::Write;

use crate::executor;
use crate::lexer;
use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct DotBuiltin;

impl super::Builtin for DotBuiltin {
    fn name(&self) -> &'static str {
        "."
    }

    fn description(&self) -> &'static str {
        "Execute a script file (same as source)"
    }

    fn run(&self, cmd: &ShellCommand, shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        if cmd.args.len() < 2 {
            let _ = writeln!(output_writer, ".: missing script file operand");
            return 1;
        }
        let script_file = &cmd.args[1];

        match fs::read_to_string(script_file) {
            Ok(content) => {
                let mut exit_code = 0;
                for line in content.lines() {
                    let line = line.trim();
                    // Skip shebang lines and empty lines
                    if line.is_empty() || line.starts_with("#!") {
                        continue;
                    }
                    // Skip comment lines
                    if line.starts_with("#") {
                        continue;
                    }
                    // Execute the line using the same logic as main.rs
                    // Now using the parent shell state to share variables with sourced scripts
                    match lexer::lex(line, &*shell_state) {
                        Ok(tokens) => match crate::parser::parse(tokens) {
                            Ok(ast) => {
                                exit_code = executor::execute(ast, shell_state);
                            }
                            Err(e) => {
                                let _ = writeln!(output_writer, "Parse error: {}", e);
                                return 1;
                            }
                        },
                        Err(e) => {
                            let _ = writeln!(output_writer, "Lex error: {}", e);
                            return 1;
                        }
                    }
                }
                exit_code
            }
            Err(e) => {
                let _ = writeln!(output_writer, ".: {}: {}", script_file, e);
                1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;
    use std::fs;

    #[test]
    fn test_execute_builtin_dot_variable_sharing() {
        // Create a temporary script file
        let temp_script = "/tmp/test_dot_vars.sh";
        let script_content = "DOT_TEST_VAR=dot_shared\nDOT_VAR2=dot_value";
        fs::write(temp_script, script_content).unwrap();

        let cmd = ShellCommand {
            args: vec![".".to_string(), temp_script.to_string()],
            input: None,
            output: None,
            append: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let builtin = DotBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);

        // Verify that variables are now available in the shell state
        assert_eq!(shell_state.get_var("DOT_TEST_VAR"), Some("dot_shared".to_string()));
        assert_eq!(shell_state.get_var("DOT_VAR2"), Some("dot_value".to_string()));

        // Clean up
        fs::remove_file(temp_script).unwrap();
    }
}