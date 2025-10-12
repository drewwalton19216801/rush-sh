use std::fs;
use std::io::Write;

use crate::executor;
use crate::lexer;
use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct SourceBuiltin;

impl super::Builtin for SourceBuiltin {
    fn name(&self) -> &'static str {
        "source"
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["source", "."]
    }

    fn description(&self) -> &'static str {
        "Execute a script file with rush"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        if cmd.args.len() < 2 {
            let _ = writeln!(output_writer, "source: missing script file operand");
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
                let _ = writeln!(output_writer, "source: {}: {}", script_file, e);
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
    fn test_execute_builtin_source_variable_sharing() {
        // Create a temporary script file
        let temp_script = "/tmp/test_source_vars.sh";
        let script_content = "TEST_VAR_FROM_SOURCE=shared_value\nANOTHER_VAR=another_value";
        fs::write(temp_script, script_content).unwrap();

        let cmd = ShellCommand {
            args: vec!["source".to_string(), temp_script.to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_doc_quoted: false,
            here_string_content: None,
        };
        let mut shell_state = crate::state::ShellState::new();
        let builtin = SourceBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);

        // Verify that variables are now available in the shell state
        assert_eq!(
            shell_state.get_var("TEST_VAR_FROM_SOURCE"),
            Some("shared_value".to_string())
        );
        assert_eq!(
            shell_state.get_var("ANOTHER_VAR"),
            Some("another_value".to_string())
        );

        // Clean up
        fs::remove_file(temp_script).unwrap();
    }
}
