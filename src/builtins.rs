use std::fs::File;
use std::io::{self, Write};

use crate::parser::ShellCommand;
use crate::state::ShellState;

/// A writer wrapper for output handling
pub struct ColoredWriter<W: Write> {
    inner: W,
}

impl<W: Write> ColoredWriter<W> {
    pub fn new(inner: W) -> Self {
        Self { inner }
    }
}

impl<W: Write> Write for ColoredWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

mod builtin_alias;
mod builtin_cd;
mod builtin_declare;
mod builtin_dirs;
mod builtin_env;
mod builtin_exit;
mod builtin_export;
mod builtin_help;
mod builtin_popd;
mod builtin_pushd;
mod builtin_pwd;
mod builtin_set_color_scheme;
mod builtin_set_colors;
mod builtin_set_condensed;
mod builtin_shift;
mod builtin_source;
mod builtin_test;
mod builtin_trap;
mod builtin_unalias;
mod builtin_unset;

pub trait Builtin {
    fn name(&self) -> &'static str;
    fn names(&self) -> Vec<&'static str>;
    fn description(&self) -> &'static str;
    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32;
}

fn get_builtins() -> Vec<Box<dyn Builtin>> {
    vec![
        Box::new(builtin_cd::CdBuiltin),
        Box::new(builtin_pwd::PwdBuiltin),
        Box::new(builtin_env::EnvBuiltin),
        Box::new(builtin_exit::ExitBuiltin),
        Box::new(builtin_help::HelpBuiltin),
        Box::new(builtin_source::SourceBuiltin),
        Box::new(builtin_export::ExportBuiltin),
        Box::new(builtin_unset::UnsetBuiltin),
        Box::new(builtin_pushd::PushdBuiltin),
        Box::new(builtin_popd::PopdBuiltin),
        Box::new(builtin_dirs::DirsBuiltin),
        Box::new(builtin_alias::AliasBuiltin),
        Box::new(builtin_unalias::UnaliasBuiltin),
        Box::new(builtin_test::TestBuiltin),
        Box::new(builtin_set_colors::SetColorsBuiltin),
        Box::new(builtin_set_color_scheme::SetColorSchemeBuiltin),
        Box::new(builtin_set_condensed::SetCondensedBuiltin),
        Box::new(builtin_shift::ShiftBuiltin),
        Box::new(builtin_declare::DeclareBuiltin),
        Box::new(builtin_trap::TrapBuiltin),
    ]
}

pub fn is_builtin(cmd: &str) -> bool {
    get_builtins().iter().any(|b| b.names().contains(&cmd))
}

pub fn get_builtin_commands() -> Vec<String> {
    let builtins = get_builtins();
    let mut commands = Vec::new();
    for b in builtins {
        for &name in &b.names() {
            commands.push(name.to_string());
        }
    }
    commands
}

pub fn execute_builtin(
    cmd: &ShellCommand,
    shell_state: &mut ShellState,
    output_override: Option<Box<dyn Write>>,
) -> i32 {
    // Helper function for colored error messages
    let print_error = |msg: &str| {
        if shell_state.colors_enabled {
            eprintln!("{}{}\x1b[0m", shell_state.color_scheme.error, msg);
        } else {
            eprintln!("{}", msg);
        }
    };
    // Handle redirections for built-ins
    use crate::parser::Redirection;
    
    // Check for input redirections
    let _input_content = cmd.redirections.iter().find_map(|r| {
        if let Redirection::Input(file) = r {
            match std::fs::read_to_string(file) {
                Ok(content) => Some(content),
                Err(e) => {
                    print_error(&format!("Error reading input file '{}': {}", file, e));
                    None
                }
            }
        } else {
            None
        }
    });

    // Prepare output destination
    let mut output_writer: Box<dyn Write> = if let Some(override_writer) = output_override {
        override_writer
    } else {
        // Check for output redirections
        let mut found_output = false;
        for redir in &cmd.redirections {
            match redir {
                Redirection::Output(file) => {
                    match File::create(file) {
                        Ok(_f) => {
                            found_output = true;
                            break;
                        }
                        Err(e) => {
                            print_error(&format!("Error creating output file '{}': {}", file, e));
                            return 1;
                        }
                    }
                }
                Redirection::Append(file) => {
                    match File::options().append(true).create(true).open(file) {
                        Ok(_f) => {
                            found_output = true;
                            break;
                        }
                        Err(e) => {
                            print_error(&format!("Error opening append file '{}': {}", file, e));
                            return 1;
                        }
                    }
                }
                _ => {}
            }
        }
        
        if found_output {
            // Re-open the file for writing (this is a simplified approach)
            // In a real implementation, we'd want to handle this more efficiently
            for redir in &cmd.redirections {
                match redir {
                    Redirection::Output(file) => {
                        return match File::create(file) {
                            Ok(f) => {
                                let builtins = get_builtins();
                                if let Some(builtin) = builtins
                                    .into_iter()
                                    .find(|b| b.names().contains(&cmd.args[0].as_str()))
                                {
                                    builtin.run(cmd, shell_state, &mut Box::new(f) as &mut dyn Write)
                                } else {
                                    1
                                }
                            }
                            Err(e) => {
                                print_error(&format!("Error creating output file '{}': {}", file, e));
                                1
                            }
                        };
                    }
                    Redirection::Append(file) => {
                        return match File::options().append(true).create(true).open(file) {
                            Ok(f) => {
                                let builtins = get_builtins();
                                if let Some(builtin) = builtins
                                    .into_iter()
                                    .find(|b| b.names().contains(&cmd.args[0].as_str()))
                                {
                                    builtin.run(cmd, shell_state, &mut Box::new(f) as &mut dyn Write)
                                } else {
                                    1
                                }
                            }
                            Err(e) => {
                                print_error(&format!("Error opening append file '{}': {}", file, e));
                                1
                            }
                        };
                    }
                    _ => {}
                }
            }
        }
        
        // Terminal output
        Box::new(ColoredWriter::new(io::stdout()))
    };

    let builtins = get_builtins();
    if let Some(builtin) = builtins
        .into_iter()
        .find(|b| b.names().contains(&cmd.args[0].as_str()))
    {
        builtin.run(cmd, shell_state, &mut *output_writer)
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin() {
        assert!(is_builtin("cd"));
        assert!(is_builtin("pwd"));
        assert!(is_builtin("env"));
        assert!(is_builtin("exit"));
        assert!(is_builtin("help"));
        assert!(is_builtin("alias"));
        assert!(is_builtin("unalias"));
        assert!(is_builtin("test"));
        assert!(is_builtin("["));
        assert!(is_builtin("."));
        assert!(!is_builtin("ls"));
        assert!(!is_builtin("grep"));
        assert!(!is_builtin("echo"));
    }

    #[test]
    fn test_execute_builtin_unknown() {
        let cmd = ShellCommand {
            args: vec!["unknown".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let exit_code = execute_builtin(&cmd, &mut shell_state, None);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_get_builtin_commands() {
        let commands = get_builtin_commands();
        assert!(commands.contains(&"cd".to_string()));
        assert!(commands.contains(&"pwd".to_string()));
        assert!(commands.contains(&"env".to_string()));
        assert!(commands.contains(&"exit".to_string()));
        assert!(commands.contains(&"help".to_string()));
        assert!(commands.contains(&"source".to_string()));
        assert!(commands.contains(&"export".to_string()));
        assert!(commands.contains(&"unset".to_string()));
        assert!(commands.contains(&"pushd".to_string()));
        assert!(commands.contains(&"popd".to_string()));
        assert!(commands.contains(&"dirs".to_string()));
        assert!(commands.contains(&"alias".to_string()));
        assert!(commands.contains(&"unalias".to_string()));
        assert!(commands.contains(&"test".to_string()));
        assert!(commands.contains(&"[".to_string()));
        assert!(commands.contains(&".".to_string()));
        assert!(commands.contains(&"set_colors".to_string()));
        assert!(commands.contains(&"set_color_scheme".to_string()));
        assert!(commands.contains(&"set_condensed".to_string()));
        assert_eq!(commands.len(), 22);
    }
}
