use rustyline::history::FileHistory;
use rustyline::Editor;
use signal_hook::{consts::SIGINT, consts::SIGTERM, iterator::Signals};
use std::env;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

mod builtins;
mod completion;
mod executor;
mod lexer;
mod parser;
mod state;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

fn main() {
    let args: Vec<String> = env::args().collect();

    // Initialize shell state
    let mut shell_state = state::ShellState::new();

    // Set script name for script mode
    if args.len() > 1 && args[1] != "-c" {
        shell_state.set_script_name(&args[1]);
    }

    // Set up signal handling
    let mut signals = Signals::new(&[SIGINT, SIGTERM]).expect("Failed to create signal handler");

    // Spawn a thread to handle signals
    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => {
                    // SIGINT should interrupt current input but not exit shell
                    // We'll handle this by breaking out of readline gracefully
                    println!("^C"); // Show the interrupt indicator
                }
                SIGTERM => {
                    // SIGTERM should cause graceful shutdown
                    SHUTDOWN.store(true, Ordering::Relaxed);
                }
                _ => {}
            }
        }
    });

    if args.len() > 1 {
        if args[1] == "-c" {
            // Command mode
            if args.len() > 2 {
                if SHUTDOWN.load(Ordering::Relaxed) {
                    println!("\nReceived SIGTERM, exiting gracefully.");
                } else {
                    execute_command_string(&args[2], &mut shell_state);
                }
            } else {
                eprintln!("Error: -c requires a command string");
                std::process::exit(1);
            }
        } else {
            // Script mode
            if let Ok(content) = fs::read_to_string(&args[1]) {
                if SHUTDOWN.load(Ordering::Relaxed) {
                    println!("\nReceived SIGTERM, exiting gracefully.");
                } else {
                    execute_script(&content, &mut shell_state);
                }
            } else {
                eprintln!("Error: Could not read script file '{}'", args[1]);
                std::process::exit(1);
            }
        }
    } else {
        // Check if stdin is a TTY (interactive) or piped input
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            // Interactive mode
            // Source .rushrc file if it exists
            source_rushrc(&mut shell_state);

            println!("Rush shell started. Type 'exit' to quit.");
            let mut rl = Editor::<completion::RushCompleter, FileHistory>::new().unwrap();
            rl.set_helper(Some(completion::RushCompleter::new()));

            // Configure rustyline to handle signals gracefully
            // With signal-hook feature enabled, this helps coordinate with our signal handler
            rl.bind_sequence(
                rustyline::KeyEvent::new('\x03', rustyline::Modifiers::NONE),
                rustyline::Cmd::Interrupt,
            );

            loop {
                if SHUTDOWN.load(Ordering::Relaxed) {
                    println!("\nReceived SIGTERM, exiting gracefully.");
                    break;
                }
                let prompt = shell_state.get_prompt();
                let readline = rl.readline(&prompt);
                match readline {
                    Ok(line) => {
                        let _ = rl.add_history_entry(line.as_str());
                        if line == "exit" {
                            break;
                        }
                        execute_line(&line, &mut shell_state);
                    }
                    Err(err) => {
                        // Check if it's a signal-related error or if shutdown was requested
                        if SHUTDOWN.load(Ordering::Relaxed) {
                            println!("\nReceived SIGTERM, exiting gracefully.");
                            break;
                        } else {
                            // Check if this is a signal interruption (SIGINT)
                            let err_str = format!("{}", err);
                            if err_str.contains("Interrupted") {
                                // SIGINT should just interrupt the current input line
                                // Continue the loop to show a new prompt
                                continue;
                            }
                            // For other errors, print and continue (don't break)
                            eprintln!("Readline error: {}", err);
                            // Continue instead of breaking to keep shell running
                            continue;
                        }
                    }
                }
            }
        } else {
            // Non-interactive mode (piped input)
            use std::io::{self, Read};
            let mut input = String::new();
            if let Ok(_) = io::stdin().read_to_string(&mut input) {
                execute_script(&input, &mut shell_state);
            }
        }
    }
}

fn execute_line(line: &str, shell_state: &mut state::ShellState) {
    match lexer::lex(line, shell_state) {
        Ok(tokens) => {
            match lexer::expand_aliases(tokens, shell_state, &mut std::collections::HashSet::new())
            {
                Ok(expanded_tokens) => {
                    match parser::parse(expanded_tokens) {
                        Ok(ast) => {
                            let exit_code = executor::execute(ast, shell_state);
                            shell_state.set_last_exit_code(exit_code);
                            // TODO: For now, no printing of AST
                        }
                        Err(e) => {
                            eprintln!("Parse error: {}", e);
                            shell_state.set_last_exit_code(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Alias expansion error: {}", e);
                    shell_state.set_last_exit_code(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Lex error: {}", e);
            shell_state.set_last_exit_code(1);
        }
    }
}

fn execute_script(content: &str, shell_state: &mut state::ShellState) {
    let mut current_block = String::new();
    let mut in_if_block = false;
    let mut if_depth = 0;
    let mut in_case_block = false;

    for line in content.lines() {
        // Skip shebang lines
        if line.starts_with("#!") {
            continue;
        }

        // Skip pure comment lines
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("#") {
            continue;
        }

        // Check for multi-line construct keywords
        if trimmed.starts_with("if ") || trimmed == "if" {
            in_if_block = true;
            if_depth += 1;
        } else if trimmed.starts_with("case ") || trimmed == "case" {
            in_case_block = true;
        }

        // Add line to current block
        if !current_block.is_empty() {
            current_block.push('\n');
        }
        current_block.push_str(line);

        // Check for end of multi-line constructs
        if in_if_block && trimmed == "fi" {
            if_depth -= 1;
            if if_depth == 0 {
                in_if_block = false;
                execute_line(&current_block, shell_state);
                current_block.clear();
            }
        } else if in_case_block && trimmed == "esac" {
            in_case_block = false;
            execute_line(&current_block, shell_state);
            current_block.clear();
        } else if !in_if_block && !in_case_block {
            // Execute single-line commands immediately
            execute_line(&current_block, shell_state);
            current_block.clear();
        }
    }

    // Execute any remaining block
    if !current_block.trim().is_empty() {
        execute_line(&current_block, shell_state);
    }
}

fn execute_command_string(command_string: &str, shell_state: &mut state::ShellState) {
    // Split on semicolons and execute each command separately
    for cmd in command_string.split(';') {
        let cmd = cmd.trim();
        if !cmd.is_empty() {
            execute_line(cmd, shell_state);
        }
    }
}

fn source_rushrc(shell_state: &mut state::ShellState) {
    // Get the home directory
    if let Ok(home_dir) = env::var("HOME") {
        let rushrc_path = format!("{}/.rushrc", home_dir);

        // Try to read the .rushrc file
        if let Ok(content) = fs::read_to_string(&rushrc_path) {
            // Process the content similar to execute_script
            let mut current_block = String::new();
            let mut in_if_block = false;
            let mut if_depth = 0;
            let mut in_case_block = false;

            for line in content.lines() {
                // Skip shebang lines
                if line.starts_with("#!") {
                    continue;
                }

                // Skip pure comment lines
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with("#") {
                    continue;
                }

                // Check for multi-line construct keywords
                if trimmed.starts_with("if ") || trimmed == "if" {
                    in_if_block = true;
                    if_depth += 1;
                } else if trimmed.starts_with("case ") || trimmed == "case" {
                    in_case_block = true;
                }

                // Add line to current block
                if !current_block.is_empty() {
                    current_block.push('\n');
                }
                current_block.push_str(line);

                // Check for end of multi-line constructs
                if in_if_block && trimmed == "fi" {
                    if_depth -= 1;
                    if if_depth == 0 {
                        in_if_block = false;
                        execute_line(&current_block, shell_state);
                        current_block.clear();
                    }
                } else if in_case_block && trimmed == "esac" {
                    in_case_block = false;
                    execute_line(&current_block, shell_state);
                    current_block.clear();
                } else if !in_if_block && !in_case_block {
                    // Execute single-line commands immediately
                    execute_line(&current_block, shell_state);
                    current_block.clear();
                }
            }

            // Execute any remaining block
            if !current_block.trim().is_empty() {
                execute_line(&current_block, shell_state);
            }
        }
        // If file doesn't exist or can't be read, silently continue
    }
    // If HOME is not set, silently continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_integration_true() {
        let line = "true";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_integration_false() {
        let line = "false";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_integration_pipeline() {
        let line = "printf hello | cat";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_integration_redirection_output() {
        let temp_file = "/tmp/rush_test_output.txt";
        let line = &format!("printf test > {}", temp_file);
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert!(Path::new(temp_file).exists());
        let content = fs::read_to_string(temp_file).unwrap();
        assert_eq!(content.trim(), "test");
        fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_integration_redirection_input() {
        let temp_file = "/tmp/rush_test_input.txt";
        fs::write(temp_file, "input content").unwrap();
        let line = &format!("cat < {}", temp_file);
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_integration_variable_expansion() {
        unsafe { std::env::set_var("TEST_INTEGRATION_VAR", "expanded"); }
        let line = "printf $TEST_INTEGRATION_VAR";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        unsafe { std::env::remove_var("TEST_INTEGRATION_VAR"); }
    }

    #[test]
    fn test_integration_builtin_cd() {
        let original_dir = std::env::current_dir().unwrap();
        let line = "cd /tmp";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(std::env::current_dir().unwrap(), Path::new("/tmp"));
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_source_rushrc_functionality() {
        // Create a temporary .rushrc file
        let temp_dir = "/tmp/rush_test_home";
        let rushrc_path = format!("{}/.rushrc", temp_dir);
        let rushrc_content = "TEST_RUSHRC_VAR=test_value\nexport TEST_RUSHRC_VAR";

        // Create temp directory and file
        std::fs::create_dir_all(temp_dir).unwrap();
        std::fs::write(&rushrc_path, rushrc_content).unwrap();

        // Set HOME to temp directory
        unsafe { std::env::set_var("HOME", temp_dir); }

        // Test source_rushrc function
        let mut shell_state = state::ShellState::new();
        source_rushrc(&mut shell_state);

        // Verify variable was set
        assert_eq!(shell_state.get_var("TEST_RUSHRC_VAR"), Some("test_value".to_string()));

        // Verify variable is exported
        let child_env = shell_state.get_env_for_child();
        assert_eq!(child_env.get("TEST_RUSHRC_VAR"), Some(&"test_value".to_string()));

        // Clean up
        std::fs::remove_file(&rushrc_path).unwrap();
        std::fs::remove_dir(temp_dir).unwrap();
        unsafe { std::env::remove_var("HOME"); }
    }
}
