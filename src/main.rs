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
                let prompt = format!("{} $ ", shell_state.get_condensed_cwd());
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
    // Process the entire script at once to handle multi-line constructs
    let mut script_content = String::new();

    for line in content.lines() {
        // Skip shebang lines
        if line.starts_with("#!") {
            continue;
        }
        // Skip pure comment lines (but not inline comments)
        if line.trim_start().starts_with("#")
            && !line.contains(|c: char| !c.is_whitespace() && c != '#')
        {
            continue;
        }
        // Add the line to our script content
        script_content.push_str(line);
        script_content.push('\n');
    }

    // Now execute the entire script content as one unit
    if !script_content.trim().is_empty() {
        execute_line(&script_content, shell_state);
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
    fn test_integration_echo() {
        let line = "echo hello world";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_integration_pipeline() {
        let line = "echo hello | cat";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_integration_redirection_output() {
        let temp_file = "/tmp/rush_test_output.txt";
        let line = &format!("echo test > {}", temp_file);
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
        std::env::set_var("TEST_INTEGRATION_VAR", "expanded");
        let line = "echo $TEST_INTEGRATION_VAR";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        std::env::remove_var("TEST_INTEGRATION_VAR");
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
}
