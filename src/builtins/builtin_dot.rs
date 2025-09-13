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