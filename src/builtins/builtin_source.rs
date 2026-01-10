use std::fs;
use std::io::Write;

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
                crate::script_engine::execute_script(&content, shell_state, None);
                shell_state.last_exit_code
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
            redirections: Vec::new(),
            compound: None,
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
