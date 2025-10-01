use clap::Parser;
use rustyline::Editor;
use rustyline::history::FileHistory;
use signal_hook::{consts::SIGINT, consts::SIGTERM, iterator::Signals};
use std::env;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

mod arithmetic;
mod builtins;
mod completion;
mod executor;
mod lexer;
mod parameter_expansion;
mod parser;
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

    if !args_parsed.command.is_empty() {
        // Command mode
        let full_command = args_parsed.command.join(" ");
        if SHUTDOWN.load(Ordering::Relaxed) {
            println!("\nReceived SIGTERM, exiting gracefully.");
        } else {
            execute_command_string(&full_command, &mut shell_state);
        }
    } else if let Some(script_path) = args_parsed.script {
        // Script mode
        // Set positional parameters from script arguments
        shell_state.set_positional_params(args_parsed.script_args);

        if let Ok(content) = fs::read_to_string(&script_path) {
            if SHUTDOWN.load(Ordering::Relaxed) {
                println!("\nReceived SIGTERM, exiting gracefully.");
            } else {
                execute_script(&content, &mut shell_state);
            }
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
                let base_prompt = shell_state.get_prompt();
                let prompt = if shell_state.colors_enabled {
                    format!(
                        "{}{}{}",
                        shell_state.color_scheme.prompt, base_prompt, "\x1b[0m"
                    )
                } else {
                    base_prompt
                };
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
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}Readline error: {}\x1b[0m",
                                    shell_state.color_scheme.error, err
                                );
                            } else {
                                eprintln!("Readline error: {}", err);
                            }
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
            if io::stdin().read_to_string(&mut input).is_ok() {
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
                            if shell_state.colors_enabled {
                                eprintln!(
                                    "{}Parse error: {}\x1b[0m",
                                    shell_state.color_scheme.error, e
                                );
                            } else {
                                eprintln!("Parse error: {}", e);
                            }
                            shell_state.set_last_exit_code(1);
                        }
                    }
                }
                Err(e) => {
                    if shell_state.colors_enabled {
                        eprintln!(
                            "{}Alias expansion error: {}\x1b[0m",
                            shell_state.color_scheme.error, e
                        );
                    } else {
                        eprintln!("Alias expansion error: {}", e);
                    }
                    shell_state.set_last_exit_code(1);
                }
            }
        }
        Err(e) => {
            if shell_state.colors_enabled {
                eprintln!("{}Lex error: {}\x1b[0m", shell_state.color_scheme.error, e);
            } else {
                eprintln!("Lex error: {}", e);
            }
            shell_state.set_last_exit_code(1);
        }
    }
}

fn execute_script(content: &str, shell_state: &mut state::ShellState) {
    let mut current_block = String::new();
    let mut in_if_block = false;
    let mut if_depth = 0;
    let mut in_case_block = false;
    let mut in_function_block = false;
    let mut brace_depth = 0;
    let mut in_for_block = false;
    let mut for_depth = 0;
    let mut in_while_block = false;
    let mut while_depth = 0;

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

        // Check for multi-line construct keywords (only when not in a function)
        if !in_function_block {
            if trimmed.starts_with("if ") || trimmed == "if" {
                in_if_block = true;
                if_depth += 1;
            } else if trimmed.starts_with("case ") || trimmed == "case" {
                in_case_block = true;
            } else if trimmed.starts_with("for ") || trimmed == "for" {
                in_for_block = true;
                for_depth += 1;
            } else if trimmed.starts_with("while ") || trimmed == "while" {
                in_while_block = true;
                while_depth += 1;
            }
        }

        // Check for function definition
        if trimmed.contains("() {") || (trimmed.ends_with("()") && !in_function_block) {
            in_function_block = true;
            // Count opening braces on this line
            brace_depth += trimmed.matches('{').count() as i32;
            brace_depth -= trimmed.matches('}').count() as i32;
        } else if in_function_block {
            // Track braces inside function
            brace_depth += trimmed.matches('{').count() as i32;
            brace_depth -= trimmed.matches('}').count() as i32;
        }

        // Add line to current block
        if !current_block.is_empty() {
            current_block.push('\n');
        }
        current_block.push_str(line);

        // Check for end of multi-line constructs
        if in_function_block && brace_depth == 0 {
            // Function is complete
            in_function_block = false;
            execute_line(&current_block, shell_state);
            current_block.clear();
        } else if in_if_block && trimmed == "fi" {
            if_depth -= 1;
            if if_depth == 0 {
                in_if_block = false;
                execute_line(&current_block, shell_state);
                current_block.clear();
            }
        } else if in_for_block && trimmed == "done" {
            for_depth -= 1;
            if for_depth == 0 {
                in_for_block = false;
                execute_line(&current_block, shell_state);
                current_block.clear();
            }
        } else if in_while_block && trimmed == "done" {
            while_depth -= 1;
            if while_depth == 0 {
                in_while_block = false;
                execute_line(&current_block, shell_state);
                current_block.clear();
            }
        } else if in_case_block && trimmed == "esac" {
            in_case_block = false;
            execute_line(&current_block, shell_state);
            current_block.clear();
        } else if !in_if_block && !in_case_block && !in_function_block && !in_for_block && !in_while_block {
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
    use std::sync::Mutex;

    // Mutex to serialize tests that change the current directory
    static DIR_CHANGE_LOCK: Mutex<()> = Mutex::new(());

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
        unsafe {
            std::env::set_var("TEST_INTEGRATION_VAR", "expanded");
        }
        let line = "printf $TEST_INTEGRATION_VAR";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        unsafe {
            std::env::remove_var("TEST_INTEGRATION_VAR");
        }
    }

    #[test]
    fn test_integration_builtin_cd() {
        // Lock to prevent parallel tests from interfering with directory changes
        let _lock = DIR_CHANGE_LOCK.lock().unwrap();
        
        // First, ensure we're in a safe directory that definitely exists
        // This prevents failures if the current directory was deleted by another test
        std::env::set_current_dir("/tmp").unwrap();
        
        let line = "cd /tmp";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(std::env::current_dir().unwrap(), Path::new("/tmp"));
        
        // No need to restore since we're already in /tmp and the lock ensures
        // other directory-changing tests won't run concurrently
    }

    #[test]
    fn test_source_rushrc_functionality() {
        // Use a unique temporary directory to avoid conflicts with parallel tests
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = format!("/tmp/rush_test_home_{}", timestamp);
        let rushrc_path = format!("{}/.rushrc", temp_dir);
        let rushrc_content = "TEST_RUSHRC_VAR=test_value\nexport TEST_RUSHRC_VAR";

        // Create temp directory and file
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(&rushrc_path, rushrc_content).unwrap();

        // Save original HOME value
        let original_home = std::env::var("HOME").ok();

        // Set HOME to temp directory
        unsafe {
            std::env::set_var("HOME", &temp_dir);
        }

        // Test source_rushrc function
        let mut shell_state = state::ShellState::new();
        source_rushrc(&mut shell_state);

        // Verify variable was set
        assert_eq!(
            shell_state.get_var("TEST_RUSHRC_VAR"),
            Some("test_value".to_string())
        );

        // Verify variable is exported
        let child_env = shell_state.get_env_for_child();
        assert_eq!(
            child_env.get("TEST_RUSHRC_VAR"),
            Some(&"test_value".to_string())
        );

        // Clean up
        std::fs::remove_file(&rushrc_path).unwrap();
        std::fs::remove_dir(&temp_dir).unwrap();
        
        // Restore original HOME value
        unsafe {
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }
}
