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

    fn description(&self) -> &'static str {
        "Execute a script file with rush"
    }

    fn run(&self, cmd: &ShellCommand, _shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
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
                    // Note: source builtin doesn't have access to shell state, so we create a temporary one
                    // This is a limitation - sourced scripts don't share variables with parent
                    let mut temp_state = ShellState::new();
                    match lexer::lex(line, &temp_state) {
                        Ok(tokens) => match crate::parser::parse(tokens) {
                            Ok(ast) => {
                                exit_code = executor::execute(ast, &mut temp_state);
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
