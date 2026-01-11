use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct SetBuiltin;

impl super::Builtin for SetBuiltin {
    fn name(&self) -> &'static str {
        "set"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Set or unset shell options and positional parameters"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        // If no arguments, display all variables
        if cmd.args.len() == 1 {
            return display_all_variables(shell_state, output_writer);
        }

        // Parse arguments
        match parse_arguments(&cmd.args[1..]) {
            Ok(parsed) => {
                // Handle display modes
                if parsed.display_mode == DisplayMode::AllVariables {
                    return display_all_variables(shell_state, output_writer);
                }
                if parsed.display_mode == DisplayMode::AllOptions {
                    return display_all_options(shell_state, output_writer);
                }

                // Apply short option changes in order (preserves last-wins semantics)
                for (opt, enable) in &parsed.options_in_order {
                    if let Err(e) = shell_state.options.set_by_short_name(*opt, *enable) {
                        print_error(shell_state, &e);
                        return 1;
                    }
                }

                // Apply named option changes
                for (name, value) in &parsed.named_options {
                    if let Err(e) = shell_state.options.set_by_long_name(name, *value) {
                        print_error(shell_state, &e);
                        return 1;
                    }
                }

                // Update positional parameters if provided
                if parsed.found_double_dash || !parsed.positional_args.is_empty() {
                    shell_state.set_positional_params(parsed.positional_args);
                }

                0
            }
            Err(e) => {
                print_error(shell_state, &e);
                1
            }
        }
    }
}

/// Display mode for set command
#[derive(Debug, PartialEq)]
enum DisplayMode {
    None,
    AllVariables,
    AllOptions,
}

/// Parsed arguments from set command
#[derive(Debug)]
struct ParsedArgs {
    // Store options in order with their enable/disable state
    options_in_order: Vec<(char, bool)>,
    named_options: Vec<(String, bool)>,
    positional_args: Vec<String>,
    display_mode: DisplayMode,
    found_double_dash: bool,
}

/// Parse set command arguments
///
/// # Arguments
/// * `args` - Command arguments (excluding "set" itself)
///
/// # Returns
/// * `Ok(ParsedArgs)` on success
/// * `Err(String)` with error message on failure
fn parse_arguments(args: &[String]) -> Result<ParsedArgs, String> {
    let mut options_in_order = Vec::new();
    let mut named_options = Vec::new();
    let mut positional_args = Vec::new();
    let mut display_mode = DisplayMode::None;
    let mut found_double_dash = false;
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        // Check for end of options marker
        if arg == "--" {
            found_double_dash = true;
            positional_args.extend_from_slice(&args[i + 1..]);
            break;
        }

        // Check for option flag (starts with - or +)
        if (arg.starts_with('-') || arg.starts_with('+')) && arg.len() > 1 {
            let enable = arg.starts_with('-');
            let chars: Vec<char> = arg.chars().skip(1).collect();

            // Handle -o or +o (named option)
            if chars[0] == 'o' {
                if chars.len() == 1 {
                    // -o and +o without argument display all options (POSIX compliance)
                    i += 1;
                    if i >= args.len() {
                        // Both -o and +o with no argument display all options
                        display_mode = DisplayMode::AllOptions;
                        i -= 1; // Back up since we didn't consume an argument
                    } else {
                        // Both -o and +o with argument set/unset the option
                        named_options.push((args[i].clone(), enable));
                    }
                } else {
                    // -oOPTION format (no space)
                    let option_name: String = chars[1..].iter().collect();
                    named_options.push((option_name, enable));
                }
            } else {
                // Handle short options (can be combined like -eux)
                // Store them in order to preserve last-wins semantics
                for ch in chars {
                    options_in_order.push((ch, enable));
                }
            }
        } else {
            // Not an option, treat as positional parameter
            positional_args.extend_from_slice(&args[i..]);
            break;
        }

        i += 1;
    }

    // If no arguments at all, display all variables
    if args.is_empty() {
        display_mode = DisplayMode::AllVariables;
    }

    Ok(ParsedArgs {
        options_in_order,
        named_options,
        positional_args,
        display_mode,
        found_double_dash,
    })
}

/// Display all shell variables
fn display_all_variables(shell_state: &ShellState, output_writer: &mut dyn Write) -> i32 {
    // Get all variables sorted by name
    let mut vars: Vec<(&String, &String)> = shell_state.variables.iter().collect();
    vars.sort_by_key(|(name, _)| *name);

    for (name, value) in vars {
        let _ = writeln!(output_writer, "{}={}", name, value);
    }

    0
}

/// Display all shell options with their current state
fn display_all_options(shell_state: &ShellState, output_writer: &mut dyn Write) -> i32 {
    let options = shell_state.options.get_all_options();

    for (long_name, short_name, value) in options {
        let status = if value { "on" } else { "off" };
        
        if short_name == '\0' {
            // No short option (e.g., ignoreeof)
            let _ = writeln!(output_writer, "set -o {:<15} {}", long_name, status);
        } else {
            let _ = writeln!(
                output_writer,
                "set -{} -o {:<15} {}",
                short_name, long_name, status
            );
        }
    }

    0
}

/// Print error message with color support
fn print_error(shell_state: &ShellState, msg: &str) {
    if shell_state.colors_enabled {
        eprintln!("{}{}\x1b[0m", shell_state.color_scheme.error, msg);
    } else {
        eprintln!("{}", msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;

    #[test]
    fn test_set_builtin_no_args_displays_variables() {
        let cmd = ShellCommand {
            args: vec!["set".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_var("TEST_VAR", "test_value".to_string());

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("TEST_VAR=test_value"));
    }

    #[test]
    fn test_set_builtin_enable_single_option() {
        let cmd = ShellCommand {
            args: vec!["set".to_string(), "-e".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert!(shell_state.options.errexit);
    }

    #[test]
    fn test_set_builtin_disable_single_option() {
        let cmd = ShellCommand {
            args: vec!["set".to_string(), "+e".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.options.errexit = true;

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert!(!shell_state.options.errexit);
    }

    #[test]
    fn test_set_builtin_combined_options() {
        let cmd = ShellCommand {
            args: vec!["set".to_string(), "-eux".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert!(shell_state.options.errexit);
        assert!(shell_state.options.nounset);
        assert!(shell_state.options.xtrace);
    }

    #[test]
    fn test_set_builtin_named_option() {
        let cmd = ShellCommand {
            args: vec!["set".to_string(), "-o".to_string(), "errexit".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert!(shell_state.options.errexit);
    }

    #[test]
    fn test_set_builtin_disable_named_option() {
        let cmd = ShellCommand {
            args: vec!["set".to_string(), "+o".to_string(), "errexit".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.options.errexit = true;

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert!(!shell_state.options.errexit);
    }

    #[test]
    fn test_set_builtin_display_all_options() {
        let cmd = ShellCommand {
            args: vec!["set".to_string(), "+o".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.options.errexit = true;

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("errexit"));
        assert!(output_str.contains("on"));
    }

    #[test]
    fn test_set_builtin_positional_params() {
        let cmd = ShellCommand {
            args: vec![
                "set".to_string(),
                "--".to_string(),
                "arg1".to_string(),
                "arg2".to_string(),
                "arg3".to_string(),
            ],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("1"), Some("arg1".to_string()));
        assert_eq!(shell_state.get_var("2"), Some("arg2".to_string()));
        assert_eq!(shell_state.get_var("3"), Some("arg3".to_string()));
        assert_eq!(shell_state.get_var("#"), Some("3".to_string()));
    }

    #[test]
    fn test_set_builtin_clear_positional_params() {
        let cmd = ShellCommand {
            args: vec!["set".to_string(), "--".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.set_positional_params(vec!["old1".to_string(), "old2".to_string()]);

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("1"), None);
        assert_eq!(shell_state.get_var("#"), Some("0".to_string()));
    }

    #[test]
    fn test_set_builtin_options_and_positional_params() {
        let cmd = ShellCommand {
            args: vec![
                "set".to_string(),
                "-e".to_string(),
                "--".to_string(),
                "arg1".to_string(),
            ],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert!(shell_state.options.errexit);
        assert_eq!(shell_state.get_var("1"), Some("arg1".to_string()));
    }

    #[test]
    fn test_set_builtin_invalid_option() {
        let cmd = ShellCommand {
            args: vec!["set".to_string(), "-Z".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_set_builtin_invalid_named_option() {
        let cmd = ShellCommand {
            args: vec![
                "set".to_string(),
                "-o".to_string(),
                "invalid_option".to_string(),
            ],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_set_builtin_dash_o_displays_options() {
        let cmd = ShellCommand {
            args: vec!["set".to_string(), "-o".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.options.errexit = true;

        let builtin = SetBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // POSIX: set -o without argument displays all options
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("errexit"));
        assert!(output_str.contains("on"));
    }

    #[test]
    fn test_parse_arguments_empty() {
        let result = parse_arguments(&[]).unwrap();
        assert_eq!(result.display_mode, DisplayMode::AllVariables);
        assert!(result.options_in_order.is_empty());
    }

    #[test]
    fn test_parse_arguments_single_option() {
        let result = parse_arguments(&["-e".to_string()]).unwrap();
        assert_eq!(result.options_in_order, vec![('e', true)]);
    }

    #[test]
    fn test_parse_arguments_combined_options() {
        let result = parse_arguments(&["-eux".to_string()]).unwrap();
        assert_eq!(result.options_in_order, vec![('e', true), ('u', true), ('x', true)]);
    }

    #[test]
    fn test_parse_arguments_disable_option() {
        let result = parse_arguments(&["+e".to_string()]).unwrap();
        assert_eq!(result.options_in_order, vec![('e', false)]);
    }

    #[test]
    fn test_parse_arguments_named_option() {
        let result = parse_arguments(&["-o".to_string(), "errexit".to_string()]).unwrap();
        assert_eq!(result.named_options, vec![("errexit".to_string(), true)]);
    }

    #[test]
    fn test_parse_arguments_display_options() {
        let result = parse_arguments(&["+o".to_string()]).unwrap();
        assert_eq!(result.display_mode, DisplayMode::AllOptions);
    }

    #[test]
    fn test_parse_arguments_positional_params() {
        let result = parse_arguments(&[
            "--".to_string(),
            "arg1".to_string(),
            "arg2".to_string(),
        ])
        .unwrap();
        assert!(result.found_double_dash);
        assert_eq!(result.positional_args, vec!["arg1", "arg2"]);
    }

    #[test]
    fn test_parse_arguments_dash_o_displays_options() {
        let result = parse_arguments(&["-o".to_string()]).unwrap();
        assert_eq!(result.display_mode, DisplayMode::AllOptions);
    }
}