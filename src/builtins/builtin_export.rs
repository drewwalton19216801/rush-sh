use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct ExportBuiltin;

impl super::Builtin for ExportBuiltin {
    fn name(&self) -> &'static str {
        "export"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Export variables to environment"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        if cmd.args.len() < 2 {
            // Print all exported variables
            for var_name in &shell_state.exported {
                if let Some(value) = shell_state.variables.get(var_name) {
                    let _ = writeln!(output_writer, "export {}={}", var_name, value);
                }
            }
            0
        } else {
            let arg = &cmd.args[1];
            if let Some(eq_pos) = arg.find('=') {
                // export VAR=value
                let var = arg[..eq_pos].to_string();
                let value = arg[eq_pos + 1..].to_string();
                shell_state.set_exported_var(&var, value);
                0
            } else {
                // export VAR (mark existing var as exported)
                shell_state.export_var(arg);
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;

    #[test]
    fn test_export_builtin_list() {
        let cmd = ShellCommand {
            args: vec!["export".to_string()],
            redirections: Vec::new(),
        };
        let mut shell_state = ShellState::new();
        shell_state.set_exported_var("TEST_VAR", "test_value".to_string());
        let builtin = ExportBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("export TEST_VAR=test_value"));
    }

    #[test]
    fn test_export_builtin_set() {
        let cmd = ShellCommand {
            args: vec!["export".to_string(), "NEW_VAR=new_value".to_string()],
            redirections: Vec::new(),
        };
        let mut shell_state = ShellState::new();
        let builtin = ExportBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert_eq!(
            shell_state.get_var("NEW_VAR"),
            Some("new_value".to_string())
        );
        assert!(shell_state.exported.contains("NEW_VAR"));
    }

    #[test]
    fn test_export_builtin_export_existing() {
        let cmd = ShellCommand {
            args: vec!["export".to_string(), "EXISTING_VAR".to_string()],
            redirections: Vec::new(),
        };
        let mut shell_state = ShellState::new();
        shell_state.set_var("EXISTING_VAR", "existing_value".to_string());
        let builtin = ExportBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        assert_eq!(exit_code, 0);
        assert!(shell_state.exported.contains("EXISTING_VAR"));
    }
}
