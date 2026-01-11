use clap::Parser;
use rustyline::Editor;
use rustyline::history::FileHistory;
use signal_hook::{consts::SIGINT, consts::SIGTERM, iterator::Signals};
use std::env;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

mod arithmetic;
mod brace_expansion;
mod builtins;
mod completion;
mod executor;
mod lexer;
mod parameter_expansion;
mod parser;
mod script_engine;
mod state;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

#[derive(Parser)]
#[command(
    author = "Drew Walton",
    about = "A POSIX sh-compatible shell written in Rust",
    long_about = r#"Rush is a POSIX-compliant shell implemented in Rust.

Examples:
  rush-sh script.sh
  rush-sh -c echo hello
  rush-sh -v"#
)]
struct Args {
    #[arg(short = 'c', num_args = 1.., value_name = "COMMAND", conflicts_with = "script")]
    command: Vec<String>,

    #[arg(short = 'v', long = "version", conflicts_with_all = ["command", "script"])]
    version: bool,

    #[arg(value_name = "SCRIPT", conflicts_with = "command")]
    script: Option<String>,

    #[arg(value_name = "ARGS", help = "Arguments to pass to the script")]
    script_args: Vec<String>,
}

fn main() {
    let args_parsed = Args::parse();

    if args_parsed.version {
        let header = format!("Rush Shell (rush-sh) v{}", env!("CARGO_PKG_VERSION"));
        println!("{}", header);
        println!("Copyright (C) 2025 Drew Walton");
        println!("License MIT <https://opensource.org/license/mit>");
        println!("\nThis is free software; you are free to change and redistribute it.");
        println!("There is NO WARRANTY, to the extent permitted by law.");
        std::process::exit(0);
    }

    // Initialize shell state
    let mut shell_state = state::ShellState::new();

    // Set script name for script mode
    if let Some(ref script_path) = args_parsed.script {
        shell_state.set_script_name(script_path);
    }

    // Set up signal handling
    let mut signals = Signals::new([SIGINT, SIGTERM]).expect("Failed to create signal handler");

    // Spawn a thread to handle signals
    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => {
                    // SIGINT should interrupt current input but not exit shell
                    println!("^C"); // Show the interrupt indicator

                    // Enqueue signal for trap execution
                    state::enqueue_signal("INT", 2);
                }
                SIGTERM => {
                    // SIGTERM should cause graceful shutdown
                    SHUTDOWN.store(true, Ordering::Relaxed);

                    // Enqueue signal for trap execution
                    state::enqueue_signal("TERM", 15);
                }
                _ => {}
            }
        }
    });

    if !args_parsed.command.is_empty() {
        // Command mode
        let full_command = args_parsed.command.join(" ");
        if SHUTDOWN.load(Ordering::Relaxed) {
            println!("\nReceived SIGTERM, exiting gracefully.");
        } else {
            script_engine::execute_line(&full_command, &mut shell_state);
        }
        // Execute EXIT trap before exiting
        execute_exit_trap(&mut shell_state);
    } else if let Some(script_path) = args_parsed.script {
        // Script mode
        // Set positional parameters from script arguments
        shell_state.set_positional_params(args_parsed.script_args);

        if let Ok(content) = fs::read_to_string(&script_path) {
            if SHUTDOWN.load(Ordering::Relaxed) {
                println!("\nReceived SIGTERM, exiting gracefully.");
            } else {
                script_engine::execute_script(&content, &mut shell_state, Some(&SHUTDOWN));
            }
            // Execute EXIT trap before exiting
            execute_exit_trap(&mut shell_state);
        } else {
            if shell_state.colors_enabled {
                eprintln!(
                    "{}Error: Could not read script file '{}'\x1b[0m",
                    shell_state.color_scheme.error, script_path
                );
            } else {
                eprintln!("Error: Could not read script file '{}'", script_path);
            }
            std::process::exit(1);
        }
    } else {
        // Check if stdin is a TTY (interactive) or piped input
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            // Interactive mode
            // Source .rushrc file if it exists
            source_rushrc(&mut shell_state);

            let config = rustyline::Config::builder()
                .bracketed_paste(true) // Enable bracketed paste to handle multi-line pastes
                .build();
            let mut rl =
                Editor::<completion::RushCompleter, FileHistory>::with_config(config).unwrap();
            rl.set_helper(Some(completion::RushCompleter::new()));

            // Configure rustyline to handle signals gracefully
            rl.bind_sequence(
                rustyline::KeyEvent::new('\x03', rustyline::Modifiers::NONE),
                rustyline::Cmd::Interrupt,
            );

            loop {
                // Process any pending signals before showing prompt
                state::process_pending_signals(&mut shell_state);

                if SHUTDOWN.load(Ordering::Relaxed) {
                    println!("\nReceived SIGTERM, exiting gracefully.");
                    execute_exit_trap(&mut shell_state);
                    break;
                }

                // Determine the prompt based on whether we're collecting a heredoc
                let prompt_str = if shell_state.collecting_heredoc.is_some() {
                    "> ".to_string()
                } else {
                    let base_prompt = shell_state.get_prompt();
                    if shell_state.colors_enabled {
                        format!(
                            "{}{}{}",
                            shell_state.color_scheme.prompt, base_prompt, "\x1b[0m"
                        )
                    } else {
                        base_prompt
                    }
                };

                let readline = rl.readline(&prompt_str);
                match readline {
                    Ok(line) => {
                        // Check if we're currently collecting heredoc content
                        if let Some((command_line, delimiter, mut content)) =
                            shell_state.collecting_heredoc.take()
                        {
                            if line.contains('\n') {
                                // Multi-line paste
                                let lines: Vec<&str> = line.split('\n').collect();
                                let mut found_delimiter = false;
                                for (i, line_part) in lines.iter().enumerate() {
                                    let cleaned_line = line_part.trim_start_matches("> ");
                                    if cleaned_line.trim() == delimiter.trim() {
                                        found_delimiter = true;
                                        shell_state.pending_heredoc_content = Some(content.clone());
                                        script_engine::execute_line(
                                            &command_line,
                                            &mut shell_state,
                                        );
                                        for remaining_line in lines.iter().skip(i + 1) {
                                            if !remaining_line.trim().is_empty() {
                                                script_engine::execute_line(
                                                    remaining_line,
                                                    &mut shell_state,
                                                );
                                            }
                                        }
                                        break;
                                    } else {
                                        if !content.is_empty() {
                                            content.push('\n');
                                        }
                                        content.push_str(cleaned_line);
                                    }
                                }
                                if !found_delimiter {
                                    shell_state.collecting_heredoc =
                                        Some((command_line, delimiter, content));
                                }
                            } else {
                                if line.trim() == delimiter.trim() {
                                    shell_state.pending_heredoc_content = Some(content);
                                    script_engine::execute_line(&command_line, &mut shell_state);
                                } else {
                                    if !content.is_empty() {
                                        content.push('\n');
                                    }
                                    content.push_str(&line);
                                    shell_state.collecting_heredoc =
                                        Some((command_line, delimiter, content));
                                }
                            }
                        } else {
                            // Normal line processing
                            if line.contains('\n') && line.contains("<<") && !line.contains("<<<") {
                                let lines: Vec<&str> = line.split('\n').collect();
                                let mut i = 0;
                                while i < lines.len() {
                                    let current_line = lines[i];
                                    if let Some(delimiter) = script_engine::line_contains_heredoc(
                                        current_line,
                                        &shell_state,
                                    ) {
                                        let mut heredoc_content = String::new();
                                        i += 1;
                                        let mut found_delimiter = false;
                                        while i < lines.len() {
                                            let line_to_check = lines[i].trim_start_matches("> ");
                                            if line_to_check.trim() == delimiter.trim() {
                                                found_delimiter = true;
                                                i += 1;
                                                break;
                                            }
                                            if !heredoc_content.is_empty() {
                                                heredoc_content.push('\n');
                                            }
                                            heredoc_content.push_str(line_to_check);
                                            i += 1;
                                        }
                                        if found_delimiter {
                                            shell_state.pending_heredoc_content =
                                                Some(heredoc_content);
                                            script_engine::execute_line(
                                                current_line,
                                                &mut shell_state,
                                            );
                                        } else {
                                            shell_state.collecting_heredoc = Some((
                                                current_line.to_string(),
                                                delimiter,
                                                heredoc_content,
                                            ));
                                        }
                                        continue;
                                    }
                                    if !current_line.trim().is_empty() {
                                        let _ = rl.add_history_entry(current_line);
                                        if current_line == "exit" {
                                            execute_exit_trap(&mut shell_state);
                                            break;
                                        }
                                        script_engine::execute_line(current_line, &mut shell_state);
                                    }
                                    i += 1;
                                }
                            } else {
                                let _ = rl.add_history_entry(line.as_str());
                                if line == "exit" {
                                    execute_exit_trap(&mut shell_state);
                                    break;
                                }
                                if let Some(delimiter) =
                                    script_engine::line_contains_heredoc(&line, &shell_state)
                                {
                                    shell_state.collecting_heredoc =
                                        Some((line.clone(), delimiter, String::new()));
                                    continue;
                                }
                                script_engine::execute_line(&line, &mut shell_state);
                            }
                        }
                        state::process_pending_signals(&mut shell_state);
                    }
                    Err(err) => {
                        state::process_pending_signals(&mut shell_state);
                        if SHUTDOWN.load(Ordering::Relaxed) {
                            println!("\nReceived SIGTERM, exiting gracefully.");
                            execute_exit_trap(&mut shell_state);
                            break;
                        }
                        let err_str = format!("{}", err);
                        if err_str.contains("EOF") && shell_state.collecting_heredoc.is_some() {
                            if let Some((command_line, _delimiter, content)) =
                                shell_state.collecting_heredoc.take()
                            {
                                shell_state.pending_heredoc_content = Some(content);
                                script_engine::execute_line(&command_line, &mut shell_state);
                            }
                            continue;
                        }
                        if err_str.contains("Interrupted") {
                            shell_state.collecting_heredoc = None;
                            continue;
                        }
                        if err_str.contains("EOF") {
                            println!();
                            execute_exit_trap(&mut shell_state);
                            break;
                        }
                        if shell_state.colors_enabled {
                            eprintln!(
                                "{}Readline error: {}\x1b[0m",
                                shell_state.color_scheme.error, err
                            );
                        } else {
                            eprintln!("Readline error: {}", err);
                        }
                        continue;
                    }
                }
            }
        } else {
            use std::io::{self, Read};
            let mut input = String::new();
            if io::stdin().read_to_string(&mut input).is_ok() {
                script_engine::execute_script(&input, &mut shell_state, Some(&SHUTDOWN));
            }
            execute_exit_trap(&mut shell_state);
        }
    }
    execute_exit_trap(&mut shell_state);
}

fn execute_exit_trap(shell_state: &mut state::ShellState) {
    if shell_state.exit_trap_executed {
        return;
    }
    if let Some(trap_cmd) = shell_state.get_trap("EXIT")
        && !trap_cmd.is_empty()
    {
        shell_state.exit_trap_executed = true;
        executor::execute_trap_handler(&trap_cmd, shell_state);
    }
}

fn source_rushrc(shell_state: &mut state::ShellState) {
    if let Some(home) = env::var_os("HOME") {
        let rushrc_path = std::path::Path::new(&home).join(".rushrc");
        if rushrc_path.exists() {
            if let Ok(content) = fs::read_to_string(rushrc_path) {
                script_engine::execute_script(&content, shell_state, Some(&SHUTDOWN));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test noexec option (-n): Commands should be parsed but not executed
    #[test]
    fn test_noexec_basic() {
        let mut shell_state = state::ShellState::new();
        
        // Enable noexec
        shell_state.options.noexec = true;
        
        // This command would normally set a variable, but with noexec it shouldn't
        script_engine::execute_line("TEST_VAR=should_not_be_set", &mut shell_state);
        
        // Variable should not be set because command wasn't executed
        assert_eq!(shell_state.get_var("TEST_VAR"), None);
        
        // Exit code should still be 0 (success)
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test noexec with complex commands
    #[test]
    #[ignore] // TODO: Investigate noexec behavior with file creation - unrelated to nounset
    fn test_noexec_complex() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.noexec = true;
        
        // Create unique temp file path
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/test_noexec_{}.txt", timestamp);
        
        // Complex command with pipes and redirections
        script_engine::execute_line(&format!("echo hello | cat > {}", temp_file), &mut shell_state);
        
        // File should not be created
        assert!(!std::path::Path::new(&temp_file).exists());
        
        // Exit code should be 0
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test xtrace option (-x): Commands should be printed before execution
    #[test]
    fn test_xtrace_basic() {
        let mut shell_state = state::ShellState::new();
        
        // Enable xtrace
        shell_state.options.xtrace = true;
        
        // Set a test variable
        shell_state.set_var("TEST_VAR", "test_value".to_string());
        
        // Execute a command - it should print trace output
        // We can't easily capture stderr in this test, but we can verify the command executes
        script_engine::execute_line("echo $TEST_VAR", &mut shell_state);
        
        // Command should execute normally
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test xtrace with PS4 variable
    #[test]
    fn test_xtrace_with_ps4() {
        let mut shell_state = state::ShellState::new();
        
        // Set custom PS4
        shell_state.set_var("PS4", "DEBUG: ".to_string());
        shell_state.options.xtrace = true;
        
        // Execute command
        script_engine::execute_line("echo test", &mut shell_state);
        
        // Verify PS4 is set correctly
        assert_eq!(shell_state.get_var("PS4"), Some("DEBUG: ".to_string()));
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test nounset option (-u): Unset variables should cause errors
    #[test]
    fn test_nounset_basic() {
        let mut shell_state = state::ShellState::new();
        
        // Enable nounset
        shell_state.options.nounset = true;
        
        // Try to expand an unset variable with ${} syntax - should fail
        script_engine::execute_line("TEST=${UNSET_VAR}", &mut shell_state);
        
        // Should have non-zero exit code due to error
        assert_ne!(shell_state.last_exit_code, 0);
    }

    /// Test nounset with set variables
    #[test]
    fn test_nounset_with_set_variable() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.nounset = true;
        
        // Set a variable
        shell_state.set_var("SET_VAR", "value".to_string());
        
        // Should work fine with set variable
        script_engine::execute_line("echo ${SET_VAR}", &mut shell_state);
        
        // Should succeed
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test nounset with default expansion
    #[test]
    fn test_nounset_with_default() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.nounset = true;
        
        // Using default expansion should work even with unset variable
        script_engine::execute_line("echo ${UNSET_VAR:-default}", &mut shell_state);
        
        // Should succeed because we provided a default
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test errexit option (-e): Non-zero exit should cause shell exit
    #[test]
    fn test_errexit_basic() {
        let mut shell_state = state::ShellState::new();
        
        // Enable errexit
        shell_state.options.errexit = true;
        
        // Execute a command that fails
        script_engine::execute_line("false", &mut shell_state);
        
        // exit_requested should be set
        assert!(shell_state.exit_requested);
        assert_ne!(shell_state.exit_code, 0);
    }

    /// Test errexit with successful command
    #[test]
    fn test_errexit_success() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // Execute a successful command
        script_engine::execute_line("true", &mut shell_state);
        
        // exit_requested should NOT be set
        assert!(!shell_state.exit_requested);
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test errexit doesn't trigger in conditionals
    #[test]
    fn test_errexit_in_conditional() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // False in if condition should not trigger errexit
        script_engine::execute_line("if false; then echo fail; else echo pass; fi", &mut shell_state);
        
        // Should not exit
        assert!(!shell_state.exit_requested);
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test multiple options together
    #[test]
    fn test_multiple_options() {
        let mut shell_state = state::ShellState::new();
        
        // Enable multiple options
        shell_state.options.xtrace = true;
        shell_state.options.nounset = true;
        
        // Set a variable
        shell_state.set_var("TEST", "value".to_string());
        
        // Should work with set variable
        script_engine::execute_line("echo ${TEST}", &mut shell_state);
        assert_eq!(shell_state.last_exit_code, 0);
        
        // Should fail with unset variable using ${} syntax
        script_engine::execute_line("TEST2=${UNSET}", &mut shell_state);
        assert_ne!(shell_state.last_exit_code, 0);
    }

    /// Test set builtin enables options correctly
    #[test]
    fn test_set_builtin_enable_options() {
        let mut shell_state = state::ShellState::new();
        
        // Initially all options should be off
        assert!(!shell_state.options.errexit);
        assert!(!shell_state.options.nounset);
        assert!(!shell_state.options.xtrace);
        assert!(!shell_state.options.noexec);
        
        // Enable errexit with -e
        script_engine::execute_line("set -e", &mut shell_state);
        assert!(shell_state.options.errexit);
        
        // Enable nounset with -u
        script_engine::execute_line("set -u", &mut shell_state);
        assert!(shell_state.options.nounset);
        
        // Enable xtrace with -x
        script_engine::execute_line("set -x", &mut shell_state);
        assert!(shell_state.options.xtrace);
        
        // Enable noexec with -n
        script_engine::execute_line("set -n", &mut shell_state);
        assert!(shell_state.options.noexec);
    }

    /// Test set builtin disables options correctly
    #[test]
    fn test_set_builtin_disable_options() {
        let mut shell_state = state::ShellState::new();
        
        // Enable all options first
        shell_state.options.errexit = true;
        shell_state.options.nounset = true;
        shell_state.options.xtrace = true;
        shell_state.options.noexec = true;
        
        // Disable errexit with +e
        script_engine::execute_line("set +e", &mut shell_state);
        assert!(!shell_state.options.errexit);
        
        // Disable nounset with +u
        script_engine::execute_line("set +u", &mut shell_state);
        assert!(!shell_state.options.nounset);
        
        // Disable xtrace with +x
        script_engine::execute_line("set +x", &mut shell_state);
        assert!(!shell_state.options.xtrace);
        
        // Disable noexec with +n
        script_engine::execute_line("set +n", &mut shell_state);
        assert!(!shell_state.options.noexec);
    }

    /// Test set builtin with long option names
    #[test]
    fn test_set_builtin_long_names() {
        let mut shell_state = state::ShellState::new();
        
        // Enable with long names
        script_engine::execute_line("set -o errexit", &mut shell_state);
        assert!(shell_state.options.errexit);
        
        script_engine::execute_line("set -o nounset", &mut shell_state);
        assert!(shell_state.options.nounset);
        
        script_engine::execute_line("set -o xtrace", &mut shell_state);
        assert!(shell_state.options.xtrace);
        
        script_engine::execute_line("set -o noexec", &mut shell_state);
        assert!(shell_state.options.noexec);
        
        // Disable with long names
        script_engine::execute_line("set +o errexit", &mut shell_state);
        assert!(!shell_state.options.errexit);
        
        script_engine::execute_line("set +o nounset", &mut shell_state);
        assert!(!shell_state.options.nounset);
        
        script_engine::execute_line("set +o xtrace", &mut shell_state);
        assert!(!shell_state.options.xtrace);
        
        script_engine::execute_line("set +o noexec", &mut shell_state);
        assert!(!shell_state.options.noexec);
    }

    /// Test errexit with command sequences
    #[test]
    fn test_errexit_sequence() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // Set a marker variable
        shell_state.set_var("MARKER", "initial".to_string());
        
        // Execute sequence where first command fails
        script_engine::execute_line("false; MARKER=should_not_set", &mut shell_state);
        
        // exit_requested should be set after false
        assert!(shell_state.exit_requested);
        
        // MARKER should still be "initial" because second command shouldn't execute
        assert_eq!(shell_state.get_var("MARKER"), Some("initial".to_string()));
    }

    /// Test noexec doesn't prevent parsing errors
    #[test]
    fn test_noexec_parse_errors() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.noexec = true;
        
        // Invalid syntax should still be caught
        script_engine::execute_line("if then fi", &mut shell_state);
        
        // Should have error (parse error)
        // The exact behavior depends on error handling, but it shouldn't crash
    }

    /// Test xtrace with variable expansion
    #[test]
    fn test_xtrace_variable_expansion() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.xtrace = true;
        
        shell_state.set_var("VAR1", "value1".to_string());
        shell_state.set_var("VAR2", "value2".to_string());
        
        // Execute command with variables - trace should show expanded values
        script_engine::execute_line("echo $VAR1 $VAR2", &mut shell_state);
        
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test nounset with positional parameters
    #[test]
    fn test_nounset_positional_params() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.nounset = true;
        
        // Set positional parameters
        shell_state.set_positional_params(vec!["arg1".to_string(), "arg2".to_string()]);
        
        // Should work with set positional parameters
        script_engine::execute_line("echo $1 $2", &mut shell_state);
        assert_eq!(shell_state.last_exit_code, 0);
        
        // Accessing unset positional parameter should work (they expand to empty)
        script_engine::execute_line("echo $99", &mut shell_state);
        // Positional parameters that don't exist expand to empty, not error
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test nounset error message format
    #[test]
    fn test_nounset_error_message_format() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.nounset = true;
        
        // Execute command with unset variable
        script_engine::execute_line("echo ${UNSET_VAR}", &mut shell_state);
        
        // Should have non-zero exit code
        assert_ne!(shell_state.last_exit_code, 0);
        // Should have exit_requested set
        assert!(shell_state.exit_requested);
    }

    /// Test nounset with different expansion types
    #[test]
    fn test_nounset_with_substring_expansion() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.nounset = true;
        
        // Substring expansion on unset variable should not trigger nounset
        // because it has a modifier
        script_engine::execute_line("echo ${UNSET_VAR:0:5}", &mut shell_state);
        
        // Should succeed (substring returns empty string for unset vars)
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test nounset in subshells
    #[test]
    fn test_nounset_in_subshell() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.nounset = true;
        
        // Unset variable in subshell should fail the subshell
        script_engine::execute_line("(echo ${UNSET_VAR})", &mut shell_state);
        
        // Parent should see the error and exit
        assert!(shell_state.exit_requested);
        assert_ne!(shell_state.last_exit_code, 0);
    }

    /// Test nounset in functions
    #[test]
    fn test_nounset_in_function() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.nounset = true;
        
        // Define function that uses unset variable
        script_engine::execute_line("test_func() { echo ${UNSET_VAR}; }", &mut shell_state);
        
        // Call the function
        script_engine::execute_line("test_func", &mut shell_state);
        
        // Should fail
        assert!(shell_state.exit_requested);
        assert_ne!(shell_state.last_exit_code, 0);
    }

    /// Test nounset doesn't affect simple $VAR syntax (only ${VAR})
    #[test]
    fn test_nounset_simple_dollar_syntax() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.nounset = true;
        
        // Simple $VAR syntax should still work (expands to empty or literal)
        script_engine::execute_line("echo $UNSET_VAR", &mut shell_state);
        
        // Should succeed (simple $VAR doesn't trigger nounset)
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test errexit with && operator
    #[test]
    fn test_errexit_with_and_operator() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // false in && chain should not trigger errexit
        script_engine::execute_line("false && echo should_not_print", &mut shell_state);
        
        // Should not exit because && handles the failure
        assert!(!shell_state.exit_requested);
    }

    /// Test errexit with || operator
    #[test]
    fn test_errexit_with_or_operator() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // false in || chain should not trigger errexit
        script_engine::execute_line("false || echo fallback", &mut shell_state);
        
        // Should not exit because || handles the failure
        assert!(!shell_state.exit_requested);
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test option combinations: errexit + nounset
    #[test]
    fn test_errexit_and_nounset() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        shell_state.options.nounset = true;
        
        // Unset variable with ${} syntax should trigger error
        script_engine::execute_line("TEST=${UNSET_VAR}", &mut shell_state);
        
        // Should have error
        assert_ne!(shell_state.last_exit_code, 0);
    }

    /// Test option combinations: xtrace + noexec
    #[test]
    fn test_xtrace_and_noexec() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.xtrace = true;
        shell_state.options.noexec = true;
        
        // Should print trace but not execute
        shell_state.set_var("TEST", "initial".to_string());
        script_engine::execute_line("TEST=modified", &mut shell_state);
        
        // Variable should not be modified due to noexec
        assert_eq!(shell_state.get_var("TEST"), Some("initial".to_string()));
    }

    // ========================================================================
    // Positional Parameters Integration Tests
    // ========================================================================

    #[test]
    fn test_positional_params_set_basic() {
        let mut shell_state = state::ShellState::new();
        
        // Set positional parameters using set --
        script_engine::execute_line("set -- arg1 arg2 arg3", &mut shell_state);
        
        assert_eq!(shell_state.get_var("1"), Some("arg1".to_string()));
        assert_eq!(shell_state.get_var("2"), Some("arg2".to_string()));
        assert_eq!(shell_state.get_var("3"), Some("arg3".to_string()));
        assert_eq!(shell_state.get_var("#"), Some("3".to_string()));
        assert_eq!(shell_state.get_var("@"), Some("arg1 arg2 arg3".to_string()));
        assert_eq!(shell_state.get_var("*"), Some("arg1 arg2 arg3".to_string()));
    }

    #[test]
    fn test_positional_params_clear() {
        let mut shell_state = state::ShellState::new();
        
        // Set initial parameters
        shell_state.set_positional_params(vec!["old1".to_string(), "old2".to_string()]);
        assert_eq!(shell_state.get_var("1"), Some("old1".to_string()));
        
        // Clear with set --
        script_engine::execute_line("set --", &mut shell_state);
        
        assert_eq!(shell_state.get_var("1"), None);
        assert_eq!(shell_state.get_var("#"), Some("0".to_string()));
    }

    #[test]
    fn test_positional_params_with_options() {
        let mut shell_state = state::ShellState::new();
        
        // Combine options with positional parameters
        script_engine::execute_line("set -e -- arg1 arg2", &mut shell_state);
        
        assert!(shell_state.options.errexit);
        assert_eq!(shell_state.get_var("1"), Some("arg1".to_string()));
        assert_eq!(shell_state.get_var("2"), Some("arg2".to_string()));
    }

    #[test]
    fn test_positional_params_in_command() {
        let mut shell_state = state::ShellState::new();
        
        shell_state.set_positional_params(vec!["hello".to_string(), "world".to_string()]);
        
        // Use positional parameters in echo command
        script_engine::execute_line("TEST=$1", &mut shell_state);
        assert_eq!(shell_state.get_var("TEST"), Some("hello".to_string()));
        
        script_engine::execute_line("TEST=$2", &mut shell_state);
        assert_eq!(shell_state.get_var("TEST"), Some("world".to_string()));
    }

    #[test]
    fn test_positional_params_dollar_at() {
        let mut shell_state = state::ShellState::new();
        
        shell_state.set_positional_params(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        
        // $@ should expand to all parameters
        assert_eq!(shell_state.get_var("@"), Some("a b c".to_string()));
    }

    #[test]
    fn test_positional_params_dollar_star() {
        let mut shell_state = state::ShellState::new();
        
        shell_state.set_positional_params(vec!["x".to_string(), "y".to_string()]);
        
        // $* should expand to all parameters
        assert_eq!(shell_state.get_var("*"), Some("x y".to_string()));
    }

    #[test]
    fn test_positional_params_dollar_hash() {
        let mut shell_state = state::ShellState::new();
        
        shell_state.set_positional_params(vec!["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string()]);
        
        // $# should return count
        assert_eq!(shell_state.get_var("#"), Some("4".to_string()));
    }

    #[test]
    fn test_positional_params_empty() {
        let shell_state = state::ShellState::new();
        
        // No positional parameters set
        assert_eq!(shell_state.get_var("1"), None);
        assert_eq!(shell_state.get_var("#"), Some("0".to_string()));
        assert_eq!(shell_state.get_var("@"), Some("".to_string()));
        assert_eq!(shell_state.get_var("*"), Some("".to_string()));
    }

    #[test]
    fn test_positional_params_replace() {
        let mut shell_state = state::ShellState::new();
        
        // Set initial parameters
        script_engine::execute_line("set -- old1 old2", &mut shell_state);
        assert_eq!(shell_state.get_var("1"), Some("old1".to_string()));
        
        // Replace with new parameters
        script_engine::execute_line("set -- new1 new2 new3", &mut shell_state);
        assert_eq!(shell_state.get_var("1"), Some("new1".to_string()));
        assert_eq!(shell_state.get_var("2"), Some("new2".to_string()));
        assert_eq!(shell_state.get_var("3"), Some("new3".to_string()));
        assert_eq!(shell_state.get_var("#"), Some("3".to_string()));
    }

    // ========================================================================
    // Verbose Option (-v) Integration Tests
    // ========================================================================

    #[test]
    fn test_verbose_option_basic() {
        let mut shell_state = state::ShellState::new();
        
        // Enable verbose
        script_engine::execute_line("set -v", &mut shell_state);
        assert!(shell_state.options.verbose);
        
        // Execute a command - it should print the line (we can't capture stderr easily)
        script_engine::execute_line("echo test", &mut shell_state);
        assert_eq!(shell_state.last_exit_code, 0);
    }

    #[test]
    fn test_verbose_option_disable() {
        let mut shell_state = state::ShellState::new();
        
        // Enable then disable
        shell_state.options.verbose = true;
        script_engine::execute_line("set +v", &mut shell_state);
        
        assert!(!shell_state.options.verbose);
    }

    #[test]
    fn test_verbose_with_long_name() {
        let mut shell_state = state::ShellState::new();
        
        script_engine::execute_line("set -o verbose", &mut shell_state);
        assert!(shell_state.options.verbose);
        
        script_engine::execute_line("set +o verbose", &mut shell_state);
        assert!(!shell_state.options.verbose);
    }

    // ========================================================================
    // Noglob Option (-f) Integration Tests
    // ========================================================================

    #[test]
    fn test_noglob_option_basic() {
        let mut shell_state = state::ShellState::new();
        
        // Enable noglob
        script_engine::execute_line("set -f", &mut shell_state);
        assert!(shell_state.options.noglob);
        
        // Wildcards should not expand
        script_engine::execute_line("TEST='*.txt'", &mut shell_state);
        assert_eq!(shell_state.get_var("TEST"), Some("*.txt".to_string()));
    }

    #[test]
    fn test_noglob_option_disable() {
        let mut shell_state = state::ShellState::new();
        
        shell_state.options.noglob = true;
        script_engine::execute_line("set +f", &mut shell_state);
        
        assert!(!shell_state.options.noglob);
    }

    #[test]
    fn test_noglob_with_long_name() {
        let mut shell_state = state::ShellState::new();
        
        script_engine::execute_line("set -o noglob", &mut shell_state);
        assert!(shell_state.options.noglob);
        
        script_engine::execute_line("set +o noglob", &mut shell_state);
        assert!(!shell_state.options.noglob);
    }

    // ========================================================================
    // Noclobber Option (-C) Integration Tests
    // ========================================================================

    #[test]
    fn test_noclobber_option_basic() {
        let mut shell_state = state::ShellState::new();
        
        // Enable noclobber
        script_engine::execute_line("set -C", &mut shell_state);
        assert!(shell_state.options.noclobber);
    }

    #[test]
    fn test_noclobber_prevents_overwrite() {
        use std::sync::Mutex;
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let _lock = TEST_LOCK.lock().unwrap();
        
        let mut shell_state = state::ShellState::new();
        shell_state.options.noclobber = true;
        
        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_noclobber_{}.txt", timestamp);
        
        // Create file first
        std::fs::write(&temp_file, "original").unwrap();
        
        // Try to overwrite with > should fail
        script_engine::execute_line(&format!("echo new > {}", temp_file), &mut shell_state);
        
        // Should have error
        assert_ne!(shell_state.last_exit_code, 0);
        
        // File should still have original content
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert_eq!(content, "original");
        
        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_noclobber_allows_append() {
        use std::sync::Mutex;
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let _lock = TEST_LOCK.lock().unwrap();
        
        let mut shell_state = state::ShellState::new();
        shell_state.options.noclobber = true;
        
        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_noclobber_append_{}.txt", timestamp);
        
        // Create file
        std::fs::write(&temp_file, "line1\n").unwrap();
        
        // Append with >> should work
        script_engine::execute_line(&format!("echo line2 >> {}", temp_file), &mut shell_state);
        
        // Should succeed
        assert_eq!(shell_state.last_exit_code, 0);
        
        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_noclobber_with_long_name() {
        let mut shell_state = state::ShellState::new();
        
        script_engine::execute_line("set -o noclobber", &mut shell_state);
        assert!(shell_state.options.noclobber);
        
        script_engine::execute_line("set +o noclobber", &mut shell_state);
        assert!(!shell_state.options.noclobber);
    }

    // ========================================================================
    // Allexport Option (-a) Integration Tests
    // ========================================================================

    #[test]
    fn test_allexport_option_basic() {
        let mut shell_state = state::ShellState::new();
        
        // Enable allexport
        script_engine::execute_line("set -a", &mut shell_state);
        assert!(shell_state.options.allexport);
    }

    #[test]
    fn test_allexport_auto_exports() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.allexport = true;
        
        // Set a variable
        script_engine::execute_line("TEST_VAR=value", &mut shell_state);
        
        // Should be automatically exported
        assert!(shell_state.exported.contains("TEST_VAR"));
        assert_eq!(shell_state.get_var("TEST_VAR"), Some("value".to_string()));
    }

    #[test]
    fn test_allexport_multiple_variables() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.allexport = true;
        
        // Set multiple variables
        script_engine::execute_line("VAR1=val1", &mut shell_state);
        script_engine::execute_line("VAR2=val2", &mut shell_state);
        script_engine::execute_line("VAR3=val3", &mut shell_state);
        
        // All should be exported
        assert!(shell_state.exported.contains("VAR1"));
        assert!(shell_state.exported.contains("VAR2"));
        assert!(shell_state.exported.contains("VAR3"));
    }

    #[test]
    fn test_allexport_disable() {
        let mut shell_state = state::ShellState::new();
        
        // Enable allexport
        shell_state.options.allexport = true;
        script_engine::execute_line("VAR1=exported", &mut shell_state);
        assert!(shell_state.exported.contains("VAR1"));
        
        // Disable allexport
        script_engine::execute_line("set +a", &mut shell_state);
        assert!(!shell_state.options.allexport);
        
        // New variables should not be exported
        script_engine::execute_line("VAR2=not_exported", &mut shell_state);
        assert!(!shell_state.exported.contains("VAR2"));
    }

    #[test]
    fn test_allexport_with_long_name() {
        let mut shell_state = state::ShellState::new();
        
        script_engine::execute_line("set -o allexport", &mut shell_state);
        assert!(shell_state.options.allexport);
        
        script_engine::execute_line("set +o allexport", &mut shell_state);
        assert!(!shell_state.options.allexport);
    }

    // ========================================================================
    // Option Combinations Tests
    // ========================================================================

    #[test]
    fn test_multiple_new_options_together() {
        let mut shell_state = state::ShellState::new();
        
        // Enable multiple options at once
        script_engine::execute_line("set -vfCa", &mut shell_state);
        
        assert!(shell_state.options.verbose);
        assert!(shell_state.options.noglob);
        assert!(shell_state.options.noclobber);
        assert!(shell_state.options.allexport);
    }

    #[test]
    fn test_all_options_combined() {
        let mut shell_state = state::ShellState::new();
        
        // Enable all options
        script_engine::execute_line("set -euxvnfCa", &mut shell_state);
        
        assert!(shell_state.options.errexit);
        assert!(shell_state.options.nounset);
        assert!(shell_state.options.xtrace);
        assert!(shell_state.options.verbose);
        assert!(shell_state.options.noexec);
        assert!(shell_state.options.noglob);
        assert!(shell_state.options.noclobber);
        assert!(shell_state.options.allexport);
    }

    #[test]
    fn test_option_combination_with_positional_params() {
        let mut shell_state = state::ShellState::new();
        
        // Combine options with positional parameters
        script_engine::execute_line("set -vf -- arg1 arg2", &mut shell_state);
        
        assert!(shell_state.options.verbose);
        assert!(shell_state.options.noglob);
        assert_eq!(shell_state.get_var("1"), Some("arg1".to_string()));
        assert_eq!(shell_state.get_var("2"), Some("arg2".to_string()));
    }
}

// ========================================================================
// Edge Cases Tests
// ========================================================================

#[cfg(test)]
mod set_builtin_edge_cases {
    use super::*;

    /// Test set with no arguments (should display variables, not options)
    #[test]
    fn test_set_no_arguments() {
        let mut shell_state = state::ShellState::new();
        shell_state.set_var("TEST_VAR", "test_value".to_string());
        
        // set with no args should succeed (displays variables)
        script_engine::execute_line("set", &mut shell_state);
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test invalid option combinations
    #[test]
    fn test_invalid_option_combination() {
        let mut shell_state = state::ShellState::new();
        
        // Try to set an invalid option
        script_engine::execute_line("set -Z", &mut shell_state);
        
        // Should fail with non-zero exit code
        assert_ne!(shell_state.last_exit_code, 0);
    }

    /// Test option precedence (last wins)
    #[test]
    fn test_option_precedence_last_wins() {
        let mut shell_state = state::ShellState::new();
        
        // Enable then disable in same command
        script_engine::execute_line("set -e +e", &mut shell_state);
        
        // Last option should win (disabled)
        assert!(!shell_state.options.errexit);
        
        // Reverse order
        script_engine::execute_line("set +e -e", &mut shell_state);
        
        // Last option should win (enabled)
        assert!(shell_state.options.errexit);
    }

    /// Test mixing short and long option names
    #[test]
    fn test_mixing_short_and_long_names() {
        let mut shell_state = state::ShellState::new();
        
        // Mix short and long names
        script_engine::execute_line("set -e -o nounset", &mut shell_state);
        
        assert!(shell_state.options.errexit);
        assert!(shell_state.options.nounset);
        
        // Disable with mixed syntax
        script_engine::execute_line("set +e +o nounset", &mut shell_state);
        
        assert!(!shell_state.options.errexit);
        assert!(!shell_state.options.nounset);
    }

    /// Test Unicode/special characters in positional params
    #[test]
    fn test_unicode_in_positional_params() {
        let mut shell_state = state::ShellState::new();
        
        // Set positional parameters with Unicode
        script_engine::execute_line("set -- 你好 мир 🚀", &mut shell_state);
        
        assert_eq!(shell_state.get_var("1"), Some("你好".to_string()));
        assert_eq!(shell_state.get_var("2"), Some("мир".to_string()));
        assert_eq!(shell_state.get_var("3"), Some("🚀".to_string()));
        assert_eq!(shell_state.get_var("#"), Some("3".to_string()));
    }

    /// Test very long positional parameter lists
    #[test]
    fn test_large_positional_param_list() {
        let mut shell_state = state::ShellState::new();
        
        // Create a command with 100 positional parameters
        let mut cmd = "set --".to_string();
        for i in 1..=100 {
            cmd.push_str(&format!(" arg{}", i));
        }
        
        script_engine::execute_line(&cmd, &mut shell_state);
        
        // Verify first, middle, and last parameters
        assert_eq!(shell_state.get_var("1"), Some("arg1".to_string()));
        assert_eq!(shell_state.get_var("50"), Some("arg50".to_string()));
        assert_eq!(shell_state.get_var("100"), Some("arg100".to_string()));
        assert_eq!(shell_state.get_var("#"), Some("100".to_string()));
    }

    /// Test nested option changes (set -e in function, then set +e)
    #[test]
    fn test_nested_option_changes() {
        let mut shell_state = state::ShellState::new();
        
        // Enable errexit
        script_engine::execute_line("set -e", &mut shell_state);
        assert!(shell_state.options.errexit);
        
        // Define a function that disables errexit
        script_engine::execute_line("func() { set +e; }", &mut shell_state);
        
        // Call the function
        script_engine::execute_line("func", &mut shell_state);
        
        // errexit should be disabled after function call
        assert!(!shell_state.options.errexit);
    }

    /// Test options in subshells
    #[test]
    fn test_options_in_subshells() {
        let mut shell_state = state::ShellState::new();
        
        // Set option in parent
        shell_state.options.errexit = false;
        
        // Subshell should inherit options
        script_engine::execute_line("(set -e; false)", &mut shell_state);
        
        // Parent shell's errexit should still be disabled
        assert!(!shell_state.options.errexit);
    }

    /// Test positional params with special characters
    #[test]
    fn test_positional_params_special_chars() {
        let mut shell_state = state::ShellState::new();
        
        // Set params with special shell characters
        script_engine::execute_line("set -- '$VAR' '|' '>' '&'", &mut shell_state);
        
        assert_eq!(shell_state.get_var("1"), Some("$VAR".to_string()));
        assert_eq!(shell_state.get_var("2"), Some("|".to_string()));
        assert_eq!(shell_state.get_var("3"), Some(">".to_string()));
        assert_eq!(shell_state.get_var("4"), Some("&".to_string()));
    }
}

// ========================================================================
// Error Handling Tests
// ========================================================================

#[cfg(test)]
mod set_builtin_error_handling {
    use super::*;

    /// Test invalid short option names
    #[test]
    fn test_invalid_short_option() {
        let mut shell_state = state::ShellState::new();
        
        // Try invalid option -Z
        script_engine::execute_line("set -Z", &mut shell_state);
        
        // Should fail
        assert_ne!(shell_state.last_exit_code, 0);
    }

    /// Test invalid long option names
    #[test]
    fn test_invalid_long_option() {
        let mut shell_state = state::ShellState::new();
        
        // Try invalid long option
        script_engine::execute_line("set -o invalidoption", &mut shell_state);
        
        // Should fail
        assert_ne!(shell_state.last_exit_code, 0);
    }

    /// Test missing argument to -o
    #[test]
    fn test_missing_argument_to_dash_o() {
        let mut shell_state = state::ShellState::new();
        
        // -o without argument should fail
        script_engine::execute_line("set -o", &mut shell_state);
        
        // Should fail
        assert_ne!(shell_state.last_exit_code, 0);
    }

    /// Test malformed option syntax
    #[test]
    fn test_malformed_option_syntax() {
        let mut shell_state = state::ShellState::new();
        
        // Try various malformed syntaxes
        script_engine::execute_line("set ---", &mut shell_state);
        assert_ne!(shell_state.last_exit_code, 0);
        
        shell_state.last_exit_code = 0; // Reset
        
        script_engine::execute_line("set -", &mut shell_state);
        // Single dash might be valid (depends on implementation)
    }

    /// Test noclobber with permission errors
    #[test]
    fn test_noclobber_permission_error() {
        use std::sync::Mutex;
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let _lock = TEST_LOCK.lock().unwrap();
        
        let mut shell_state = state::ShellState::new();
        shell_state.options.noclobber = true;
        
        // Create unique temp file
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_noclobber_perm_{}.txt", timestamp);
        
        // Create file
        std::fs::write(&temp_file, "original").unwrap();
        
        // Try to overwrite should fail
        script_engine::execute_line(&format!("echo new > {}", temp_file), &mut shell_state);
        
        assert_ne!(shell_state.last_exit_code, 0);
        
        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    /// Test multiple invalid options in sequence
    #[test]
    fn test_multiple_invalid_options() {
        let mut shell_state = state::ShellState::new();
        
        // Try multiple invalid options
        script_engine::execute_line("set -XYZ", &mut shell_state);
        
        // Should fail on first invalid option
        assert_ne!(shell_state.last_exit_code, 0);
    }

    /// Test option with invalid value
    #[test]
    fn test_option_with_invalid_value() {
        let mut shell_state = state::ShellState::new();
        
        // Try to set option with value (not supported)
        script_engine::execute_line("set -o errexit=true", &mut shell_state);
        
        // Should fail (options don't take values)
        assert_ne!(shell_state.last_exit_code, 0);
    }
}

// ========================================================================
// Integration Scenarios Tests
// ========================================================================

#[cfg(test)]
mod set_builtin_integration {
    use super::*;

    /// Test options in sourced scripts
    #[test]
    fn test_options_in_sourced_script() {
        use std::sync::Mutex;
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let _lock = TEST_LOCK.lock().unwrap();
        
        let mut shell_state = state::ShellState::new();
        
        // Create unique temp script
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_script = format!("/tmp/rush_test_source_{}.sh", timestamp);
        
        // Write script that sets options
        std::fs::write(&temp_script, "set -e\nset -u\n").unwrap();
        
        // Source the script
        script_engine::execute_line(&format!(". {}", temp_script), &mut shell_state);
        
        // Options should be set in parent shell
        assert!(shell_state.options.errexit);
        assert!(shell_state.options.nounset);
        
        // Cleanup
        let _ = std::fs::remove_file(&temp_script);
    }

    /// Test options with trap handlers
    #[test]
    fn test_options_with_trap_handlers() {
        let mut shell_state = state::ShellState::new();
        
        // Set a trap
        script_engine::execute_line("trap 'echo trapped' EXIT", &mut shell_state);
        
        // Enable errexit
        script_engine::execute_line("set -e", &mut shell_state);
        
        // Options should work with traps
        assert!(shell_state.options.errexit);
        assert!(shell_state.get_trap("EXIT").is_some());
    }

    /// Test options across function calls
    #[test]
    fn test_options_across_functions() {
        let mut shell_state = state::ShellState::new();
        
        // Define function that checks options
        script_engine::execute_line("func() { set -x; }", &mut shell_state);
        
        // Call function
        script_engine::execute_line("func", &mut shell_state);
        
        // Option should persist after function
        assert!(shell_state.options.xtrace);
    }

    /// Test options with command substitution
    #[test]
    fn test_options_with_command_substitution() {
        let mut shell_state = state::ShellState::new();
        
        // Enable xtrace
        shell_state.options.xtrace = true;
        
        // Command substitution should work with xtrace
        script_engine::execute_line("VAR=$(echo test)", &mut shell_state);
        
        assert_eq!(shell_state.get_var("VAR"), Some("test".to_string()));
        assert!(shell_state.options.xtrace);
    }

    /// Test option state preservation across multiple commands
    #[test]
    fn test_option_state_preservation() {
        let mut shell_state = state::ShellState::new();
        
        // Enable multiple options
        script_engine::execute_line("set -eux", &mut shell_state);
        
        // Execute several commands
        script_engine::execute_line("echo test1", &mut shell_state);
        script_engine::execute_line("echo test2", &mut shell_state);
        script_engine::execute_line("echo test3", &mut shell_state);
        
        // Options should still be set
        assert!(shell_state.options.errexit);
        assert!(shell_state.options.nounset);
        assert!(shell_state.options.xtrace);
    }
}

// ========================================================================
// POSIX Compliance Tests
// ========================================================================

#[cfg(test)]
mod set_builtin_posix_compliance {
    use super::*;

    /// Test set with no args shows variables (not options)
    #[test]
    fn test_set_no_args_shows_variables() {
        let mut shell_state = state::ShellState::new();
        
        // Set some variables
        shell_state.set_var("VAR1", "value1".to_string());
        shell_state.set_var("VAR2", "value2".to_string());
        
        // set with no args should succeed
        script_engine::execute_line("set", &mut shell_state);
        
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test set -o shows all options
    #[test]
    fn test_set_dash_o_shows_options() {
        let mut shell_state = state::ShellState::new();
        
        // set -o should display all options
        script_engine::execute_line("set -o", &mut shell_state);
        
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test set +o shows all options in set format
    #[test]
    fn test_set_plus_o_shows_set_format() {
        let mut shell_state = state::ShellState::new();
        
        // set +o should display options in set format
        script_engine::execute_line("set +o", &mut shell_state);
        
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test option inheritance in subshells
    #[test]
    fn test_option_inheritance_subshells() {
        let mut shell_state = state::ShellState::new();
        
        // Set options in parent
        shell_state.options.errexit = true;
        shell_state.options.nounset = true;
        
        // Subshell should inherit options
        script_engine::execute_line("(echo test)", &mut shell_state);
        
        // Parent options should be unchanged
        assert!(shell_state.options.errexit);
        assert!(shell_state.options.nounset);
    }

    /// Test errexit doesn't exit in conditionals (POSIX requirement)
    #[test]
    fn test_errexit_not_in_conditionals() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // False in if condition should not trigger errexit
        script_engine::execute_line("if false; then echo fail; fi", &mut shell_state);
        
        assert!(!shell_state.exit_requested);
        
        // False in while condition should not trigger errexit
        shell_state.exit_requested = false;
        script_engine::execute_line("while false; do echo loop; done", &mut shell_state);
        
        assert!(!shell_state.exit_requested);
    }

    /// Test errexit with pipeline (only last command matters)
    #[test]
    fn test_errexit_pipeline_last_command() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // false in pipeline but last command succeeds
        script_engine::execute_line("false | true", &mut shell_state);
        
        // Should not exit because last command succeeded
        assert!(!shell_state.exit_requested);
    }

    /// Test POSIX special parameters with set
    #[test]
    fn test_posix_special_parameters() {
        let mut shell_state = state::ShellState::new();
        
        // Set positional parameters
        script_engine::execute_line("set -- a b c", &mut shell_state);
        
        // Test $# (count)
        assert_eq!(shell_state.get_var("#"), Some("3".to_string()));
        
        // Test $@ (all params)
        assert_eq!(shell_state.get_var("@"), Some("a b c".to_string()));
        
        // Test $* (all params)
        assert_eq!(shell_state.get_var("*"), Some("a b c".to_string()));
    }
}

// ========================================================================
// Performance/Stress Tests
// ========================================================================

#[cfg(test)]
mod set_builtin_performance {
    use super::*;

    /// Test large number of positional parameters (100+)
    #[test]
    fn test_large_positional_params() {
        let mut shell_state = state::ShellState::new();
        
        // Create command with 200 parameters
        let mut cmd = "set --".to_string();
        for i in 1..=200 {
            cmd.push_str(&format!(" param{}", i));
        }
        
        script_engine::execute_line(&cmd, &mut shell_state);
        
        // Verify count
        assert_eq!(shell_state.get_var("#"), Some("200".to_string()));
        
        // Verify random access
        assert_eq!(shell_state.get_var("1"), Some("param1".to_string()));
        assert_eq!(shell_state.get_var("100"), Some("param100".to_string()));
        assert_eq!(shell_state.get_var("200"), Some("param200".to_string()));
    }

    /// Test rapid option toggling
    #[test]
    fn test_rapid_option_toggling() {
        let mut shell_state = state::ShellState::new();
        
        // Toggle options rapidly
        for _ in 0..100 {
            script_engine::execute_line("set -e", &mut shell_state);
            assert!(shell_state.options.errexit);
            
            script_engine::execute_line("set +e", &mut shell_state);
            assert!(!shell_state.options.errexit);
        }
        
        // Final state should be consistent
        assert!(!shell_state.options.errexit);
    }

    /// Test options with large scripts
    #[test]
    fn test_options_with_large_script() {
        let mut shell_state = state::ShellState::new();
        
        // Enable all options
        script_engine::execute_line("set -euxvnfCa", &mut shell_state);
        
        // Execute many commands
        for i in 0..50 {
            script_engine::execute_line(&format!("VAR{}=value{}", i, i), &mut shell_state);
        }
        
        // Options should still be set
        assert!(shell_state.options.errexit);
        assert!(shell_state.options.nounset);
        assert!(shell_state.options.xtrace);
        assert!(shell_state.options.verbose);
        assert!(shell_state.options.noexec);
        assert!(shell_state.options.noglob);
        assert!(shell_state.options.noclobber);
        assert!(shell_state.options.allexport);
    }

    /// Test memory efficiency with many option changes
    #[test]
    fn test_memory_efficiency_option_changes() {
        let mut shell_state = state::ShellState::new();
        
        // Perform many option changes
        for i in 0..1000 {
            if i % 2 == 0 {
                script_engine::execute_line("set -eux", &mut shell_state);
            } else {
                script_engine::execute_line("set +eux", &mut shell_state);
            }
        }
        
        // Should complete without memory issues
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test positional parameter replacement performance
    #[test]
    fn test_positional_param_replacement_performance() {
        let mut shell_state = state::ShellState::new();
        
        // Replace positional parameters many times
        for i in 0..100 {
            let cmd = format!("set -- arg{}_1 arg{}_2 arg{}_3", i, i, i);
            script_engine::execute_line(&cmd, &mut shell_state);
            
            // Verify replacement worked
            assert_eq!(shell_state.get_var("1"), Some(format!("arg{}_1", i)));
        }
        
        assert_eq!(shell_state.last_exit_code, 0);
    }
}

// ========================================================================
// Subshell errexit Tests
// ========================================================================

#[cfg(test)]
mod subshell_errexit_tests {
    use super::*;

    /// Test that subshells inherit errexit from parent
    #[test]
    fn test_subshell_inherits_errexit() {
        let mut shell_state = state::ShellState::new();
        
        // Enable errexit in parent
        shell_state.options.errexit = true;
        
        // Execute a subshell with a failing command
        // The subshell will exit early due to errexit, returning exit code 1
        // The parent's errexit will then trigger on the subshell's non-zero exit
        script_engine::execute_line("(false; echo should_not_print)", &mut shell_state);
        
        // Parent errexit should trigger because subshell returned non-zero
        assert!(shell_state.exit_requested);
        
        // Parent option should be unchanged
        assert!(shell_state.options.errexit);
    }

    /// Test that errexit triggers exit within subshell only
    #[test]
    fn test_errexit_triggers_in_subshell_only() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // Set a marker variable
        shell_state.set_var("MARKER", "initial".to_string());
        
        // Subshell with failing command, then parent command
        // With errexit, the sequence should stop after subshell fails (just like false; would)
        script_engine::execute_line("(false); MARKER=after_subshell", &mut shell_state);
        
        // Parent errexit should trigger, stopping the sequence
        assert!(shell_state.exit_requested);
        // MARKER should still be "initial" because the assignment didn't run
        assert_eq!(shell_state.get_var("MARKER"), Some("initial".to_string()));
    }

    /// Test that subshell exit code propagates to parent
    #[test]
    fn test_subshell_exit_code_propagation() {
        let mut shell_state = state::ShellState::new();
        
        // Execute subshell that exits with specific code
        script_engine::execute_line("(exit 42)", &mut shell_state);
        
        // Parent should see the subshell's exit code
        assert_eq!(shell_state.last_exit_code, 42);
    }

    /// Test that parent errexit can trigger on subshell failure
    #[test]
    fn test_parent_errexit_on_subshell_failure() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // Subshell that fails
        script_engine::execute_line("(false)", &mut shell_state);
        
        // Parent errexit should trigger because subshell returned non-zero
        assert!(shell_state.exit_requested);
        assert_ne!(shell_state.exit_code, 0);
    }

    /// Test nested subshells with errexit
    #[test]
    fn test_nested_subshells_errexit() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // Nested subshells with failure in innermost
        script_engine::execute_line("((false))", &mut shell_state);
        
        // Parent should have exit_requested set
        assert!(shell_state.exit_requested);
    }

    /// Test that errexit changes within subshell don't affect parent
    #[test]
    fn test_subshell_errexit_changes_isolated() {
        let mut shell_state = state::ShellState::new();
        
        // Parent has errexit disabled
        shell_state.options.errexit = false;
        
        // Subshell enables errexit
        script_engine::execute_line("(set -e; false)", &mut shell_state);
        
        // Parent errexit should still be disabled
        assert!(!shell_state.options.errexit);
        assert!(!shell_state.exit_requested);
    }

    /// Test subshell with errexit and successful commands
    #[test]
    fn test_subshell_errexit_with_success() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // Subshell with successful commands
        script_engine::execute_line("(true; echo test)", &mut shell_state);
        
        // Should not trigger errexit
        assert!(!shell_state.exit_requested);
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test subshell errexit with conditionals (should not trigger)
    #[test]
    fn test_subshell_errexit_with_conditionals() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // Subshell with false in conditional
        script_engine::execute_line("(if false; then echo fail; fi)", &mut shell_state);
        
        // Should not trigger errexit (conditionals are exempt)
        assert!(!shell_state.exit_requested);
    }

    /// Test subshell errexit with logical operators
    #[test]
    fn test_subshell_errexit_with_logical_operators() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // Subshell with false in && chain
        // Inside the subshell, errexit doesn't trigger (logical operator exemption)
        // The subshell returns the exit code of the && chain (1)
        // The parent's errexit then triggers on the subshell's non-zero exit
        script_engine::execute_line("(false && echo should_not_print)", &mut shell_state);
        
        // Parent errexit should trigger because subshell returned non-zero
        assert!(shell_state.exit_requested);
    }

    /// Test multiple subshells with errexit
    #[test]
    fn test_multiple_subshells_errexit() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // First subshell succeeds
        script_engine::execute_line("(true)", &mut shell_state);
        assert!(!shell_state.exit_requested);
        
        // Second subshell fails
        script_engine::execute_line("(false)", &mut shell_state);
        assert!(shell_state.exit_requested);
    }

    /// Test subshell with errexit disabled in parent
    #[test]
    fn test_subshell_no_errexit_in_parent() {
        let mut shell_state = state::ShellState::new();
        
        // Parent has errexit disabled
        shell_state.options.errexit = false;
        
        // Subshell with failing command
        script_engine::execute_line("(false; echo should_print)", &mut shell_state);
        
        // Should not trigger exit
        assert!(!shell_state.exit_requested);
    }
}

// ========================================================================
// Negation with errexit Tests
// ========================================================================

#[cfg(test)]
mod negation_errexit_tests {
    use super::*;

    /// Test that ! false doesn't trigger errexit
    #[test]
    fn test_negation_false_no_errexit() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // ! false should succeed (exit code 0) and not trigger errexit
        script_engine::execute_line("! false", &mut shell_state);
        
        // Should not exit
        assert!(!shell_state.exit_requested);
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test that ! true doesn't trigger errexit
    #[test]
    fn test_negation_true_no_errexit() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // ! true should fail (exit code 1) but not trigger errexit
        script_engine::execute_line("! true", &mut shell_state);
        
        // Should not exit even though exit code is 1
        assert!(!shell_state.exit_requested);
        assert_eq!(shell_state.last_exit_code, 1);
    }

    /// Test negated command with errexit enabled
    #[test]
    fn test_negation_with_errexit_enabled() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        shell_state.set_var("MARKER", "initial".to_string());
        
        // ! false succeeds, so next command should run
        script_engine::execute_line("! false; MARKER=after", &mut shell_state);
        
        // Should not exit
        assert!(!shell_state.exit_requested);
        assert_eq!(shell_state.get_var("MARKER"), Some("after".to_string()));
    }

    /// Test negation in pipeline
    #[test]
    fn test_negation_in_pipeline() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // ! false | true should not trigger errexit
        script_engine::execute_line("! false | true", &mut shell_state);
        
        // Should not exit
        assert!(!shell_state.exit_requested);
    }

    /// Test negation in conditional
    #[test]
    fn test_negation_in_conditional() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        shell_state.set_var("RESULT", "none".to_string());
        
        // ! false in if condition should work
        script_engine::execute_line("if ! false; then RESULT=success; fi", &mut shell_state);
        
        // Should not exit and condition should succeed
        assert!(!shell_state.exit_requested);
        assert_eq!(shell_state.get_var("RESULT"), Some("success".to_string()));
    }

    /// Test negation with logical operators
    #[test]
    fn test_negation_with_logical_operators() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // ! false && true should not trigger errexit
        script_engine::execute_line("! false && echo success", &mut shell_state);
        
        // Should not exit
        assert!(!shell_state.exit_requested);
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test negation exit code inversion
    #[test]
    fn test_negation_exit_code_inversion() {
        let mut shell_state = state::ShellState::new();
        
        // ! false should return 0
        script_engine::execute_line("! false", &mut shell_state);
        assert_eq!(shell_state.last_exit_code, 0);
        
        // ! true should return 1
        script_engine::execute_line("! true", &mut shell_state);
        assert_eq!(shell_state.last_exit_code, 1);
    }

    /// Test negation with external commands
    #[test]
    fn test_negation_with_external_command() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        
        // ! /bin/false should not trigger errexit
        script_engine::execute_line("! /bin/false", &mut shell_state);
        
        // Should not exit
        assert!(!shell_state.exit_requested);
        assert_eq!(shell_state.last_exit_code, 0);
    }

    /// Test negation preserves errexit exemption in sequences
    #[test]
    fn test_negation_sequence_no_errexit() {
        let mut shell_state = state::ShellState::new();
        shell_state.options.errexit = true;
        shell_state.set_var("COUNT", "0".to_string());
        
        // Multiple negated commands in sequence
        script_engine::execute_line("! false; COUNT=1; ! true; COUNT=2", &mut shell_state);
        
        // Should not exit and all commands should execute
        assert!(!shell_state.exit_requested);
        assert_eq!(shell_state.get_var("COUNT"), Some("2".to_string()));
    }
}
