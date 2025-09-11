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

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

fn main() {
    let args: Vec<String> = env::args().collect();

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
                    execute_line(&args[2]);
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
                    execute_line(&content);
                }
            } else {
                eprintln!("Error: Could not read script file '{}'", args[1]);
                std::process::exit(1);
            }
        }
    } else {
        // Interactive mode
        println!("Rush shell started. Type 'exit' to quit.");
        let mut rl = Editor::<completion::RushCompleter, FileHistory>::new().unwrap();
        rl.set_helper(Some(completion::RushCompleter::new()));

        // Configure rustyline to handle signals gracefully
        // With signal-hook feature enabled, this helps coordinate with our signal handler
        rl.bind_sequence(rustyline::KeyEvent::new('\x03', rustyline::Modifiers::NONE), rustyline::Cmd::Interrupt);

        loop {
            if SHUTDOWN.load(Ordering::Relaxed) {
                println!("\nReceived SIGTERM, exiting gracefully.");
                break;
            }
            let readline = rl.readline("$ ");
            match readline {
                Ok(line) => {
                    let _ = rl.add_history_entry(line.as_str());
                    if line == "exit" {
                        break;
                    }
                    execute_line(&line);
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
    }
}

fn execute_line(line: &str) {
    match lexer::lex(line) {
        Ok(tokens) => {
            match parser::parse(tokens) {
                Ok(ast) => {
                    let _exit_code = executor::execute(ast);
                    // TODO: For now, no printing of AST
                }
                Err(e) => {
                    eprintln!("Parse error: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Lex error: {}", e);
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
        let tokens = lexer::lex(line).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_integration_false() {
        let line = "false";
        let tokens = lexer::lex(line).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_integration_echo() {
        let line = "echo hello world";
        let tokens = lexer::lex(line).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_integration_pipeline() {
        let line = "echo hello | cat";
        let tokens = lexer::lex(line).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_integration_redirection_output() {
        let temp_file = "/tmp/rush_test_output.txt";
        let line = &format!("echo test > {}", temp_file);
        let tokens = lexer::lex(line).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast);
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
        let tokens = lexer::lex(line).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast);
        assert_eq!(exit_code, 0);
        fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_integration_variable_expansion() {
        std::env::set_var("TEST_INTEGRATION_VAR", "expanded");
        let line = "echo $TEST_INTEGRATION_VAR";
        let tokens = lexer::lex(line).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast);
        assert_eq!(exit_code, 0);
        std::env::remove_var("TEST_INTEGRATION_VAR");
    }

    #[test]
    fn test_integration_builtin_cd() {
        let original_dir = std::env::current_dir().unwrap();
        let line = "cd /tmp";
        let tokens = lexer::lex(line).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast);
        assert_eq!(exit_code, 0);
        assert_eq!(std::env::current_dir().unwrap(), Path::new("/tmp"));
        std::env::set_current_dir(original_dir).unwrap();
    }
}
