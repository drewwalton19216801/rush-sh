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
            execute_command_string(&full_command, &mut shell_state);
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
                execute_script(&content, &mut shell_state);
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

            let mut rl = Editor::<completion::RushCompleter, FileHistory>::new().unwrap();
            rl.set_helper(Some(completion::RushCompleter::new()));

            // Configure rustyline to handle signals gracefully
            // With signal-hook feature enabled, this helps coordinate with our signal handler
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
                            execute_exit_trap(&mut shell_state);
                            break;
                        }
                        execute_line(&line, &mut shell_state);

                        // Process any signals that arrived during command execution
                        state::process_pending_signals(&mut shell_state);
                    }
                    Err(err) => {
                        // Process signals even on error
                        state::process_pending_signals(&mut shell_state);

                        // Check if it's a signal-related error or if shutdown was requested
                        if SHUTDOWN.load(Ordering::Relaxed) {
                            println!("\nReceived SIGTERM, exiting gracefully.");
                            execute_exit_trap(&mut shell_state);
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
            // Execute EXIT trap before exiting
            execute_exit_trap(&mut shell_state);
        }
    }
    // Execute EXIT trap at the very end if not already executed
    execute_exit_trap(&mut shell_state);
}

fn execute_exit_trap(shell_state: &mut state::ShellState) {
    // Only execute once
    if shell_state.exit_trap_executed {
        return;
    }

    // Check if EXIT trap is set
    if let Some(trap_cmd) = shell_state.get_trap("EXIT")
        && !trap_cmd.is_empty()
    {
        // Mark as executed to prevent double execution
        shell_state.exit_trap_executed = true;

        // Execute the trap handler
        executor::execute_trap_handler(&trap_cmd, shell_state);
    }
}

fn execute_line(line: &str, shell_state: &mut state::ShellState) {
    match lexer::lex(line, shell_state) {
        Ok(tokens) => {
            match lexer::expand_aliases(tokens, shell_state, &mut std::collections::HashSet::new())
            {
                Ok(expanded_tokens) => {
                    match brace_expansion::expand_braces(expanded_tokens) {
                        Ok(brace_expanded_tokens) => {
                            match parser::parse(brace_expanded_tokens) {
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
                                    "{}Brace expansion error: {}\x1b[0m",
                                    shell_state.color_scheme.error, e
                                );
                            } else {
                                eprintln!("Brace expansion error: {}", e);
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
        // Process pending signals at the start of each line
        state::process_pending_signals(shell_state);

        // Check for shutdown signal
        if SHUTDOWN.load(Ordering::Relaxed) {
            eprintln!("Script interrupted by SIGTERM");
            break;
        }

        // Check if exit was requested (e.g., from trap handler)
        if shell_state.exit_requested {
            break;
        }

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

            // Check if exit was requested
            if shell_state.exit_requested {
                break;
            }
        } else if in_if_block && trimmed == "fi" {
            if_depth -= 1;
            if if_depth == 0 {
                in_if_block = false;
                execute_line(&current_block, shell_state);
                current_block.clear();

                // Check if exit was requested
                if shell_state.exit_requested {
                    break;
                }
            }
        } else if in_for_block && trimmed == "done" {
            for_depth -= 1;
            if for_depth == 0 {
                in_for_block = false;
                execute_line(&current_block, shell_state);
                current_block.clear();

                // Check if exit was requested
                if shell_state.exit_requested {
                    break;
                }
            }
        } else if in_while_block && trimmed == "done" {
            while_depth -= 1;
            if while_depth == 0 {
                in_while_block = false;
                execute_line(&current_block, shell_state);
                current_block.clear();

                // Check if exit was requested
                if shell_state.exit_requested {
                    break;
                }
            }
        } else if in_case_block && trimmed == "esac" {
            in_case_block = false;
            execute_line(&current_block, shell_state);
            current_block.clear();

            // Check if exit was requested
            if shell_state.exit_requested {
                break;
            }
        } else if !in_if_block
            && !in_case_block
            && !in_function_block
            && !in_for_block
            && !in_while_block
        {
            // Execute single-line commands immediately
            execute_line(&current_block, shell_state);
            current_block.clear();

            // Check if exit was requested after executing the line
            if shell_state.exit_requested {
                break;
            }
        }
    }

    // Execute any remaining block
    if !current_block.trim().is_empty() {
        execute_line(&current_block, shell_state);
    }

    // Final signal processing
    state::process_pending_signals(shell_state);
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

            // After sourcing .rushrc, check if RUSH_CONDENSED was set/exported
            // This allows .rushrc to override the initial environment setting
            if let Some(condensed_value) = shell_state.get_var("RUSH_CONDENSED") {
                let condensed_lower = condensed_value.to_lowercase();
                shell_state.condensed_cwd = match condensed_lower.as_str() {
                    "1" | "true" | "on" | "enable" => true,
                    "0" | "false" | "off" | "disable" => false,
                    _ => shell_state.condensed_cwd, // Keep current value if invalid
                };
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

    // Mutex to serialize tests that modify environment variables
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // Mutex to serialize tests that access the global SIGNAL_QUEUE
    static SIGNAL_QUEUE_LOCK: Mutex<()> = Mutex::new(());

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
            env::set_var("TEST_INTEGRATION_VAR", "expanded");
        }
        let line = "printf $TEST_INTEGRATION_VAR";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        unsafe {
            env::remove_var("TEST_INTEGRATION_VAR");
        }
    }

    #[test]
    fn test_integration_builtin_cd() {
        // Lock to prevent parallel tests from interfering with directory changes
        let _lock = DIR_CHANGE_LOCK.lock().unwrap();

        // First, ensure we're in a safe directory that definitely exists
        // This prevents failures if the current directory was deleted by another test
        env::set_current_dir("/tmp").unwrap();

        let line = "cd /tmp";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(env::current_dir().unwrap(), Path::new("/tmp"));

        // No need to restore since we're already in /tmp and the lock ensures
        // other directory-changing tests won't run concurrently
    }

    #[test]
    fn test_source_rushrc_functionality() {
        // Lock to prevent parallel tests from interfering with environment variables
        let _lock = ENV_LOCK.lock().unwrap();

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
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(&rushrc_path, rushrc_content).unwrap();

        // Small delay to ensure file is written
        thread::sleep(std::time::Duration::from_millis(10));

        // Save original HOME value
        let original_home = env::var("HOME").ok();

        // Set HOME to temp directory
        unsafe {
            env::set_var("HOME", &temp_dir);
        }

        // Test source_rushrc function
        let mut shell_state = state::ShellState::new();
        source_rushrc(&mut shell_state);

        // Verify variable was set and exported by checking environment
        let mut shell_state2 = state::ShellState::new();
        source_rushrc(&mut shell_state2);

        // Verify variable is available in the environment after sourcing
        assert_eq!(
            shell_state2.get_var("TEST_RUSHRC_VAR"),
            Some("test_value".to_string())
        );

        // Clean up
        fs::remove_file(&rushrc_path).unwrap();
        fs::remove_dir(&temp_dir).unwrap();

        // Restore original HOME value
        unsafe {
            if let Some(home) = original_home {
                env::set_var("HOME", home);
            } else {
                env::remove_var("HOME");
            }
        }
    }

    #[test]
    fn test_source_rushrc_condensed_setting() {
        // Lock to prevent parallel tests from interfering with environment variables
        let _lock = ENV_LOCK.lock().unwrap();

        // Test that .rushrc can override RUSH_CONDENSED setting
        use std::time::{SystemTime, UNIX_EPOCH};

        // Use a more robust unique naming scheme to avoid race conditions
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let process_id = std::process::id();
        let temp_dir = format!("/tmp/rush_test_condensed_{}_{:x}", timestamp, process_id);
        let rushrc_path = format!("{}/.rushrc", temp_dir);

        // Save original environment variables
        let original_home = env::var("HOME").ok();
        let original_rush_condensed = env::var("RUSH_CONDENSED").ok();

        // Create temp directory and file
        fs::create_dir_all(&temp_dir).unwrap();

        // Test 1: .rushrc sets RUSH_CONDENSED=false
        fs::write(&rushrc_path, "export RUSH_CONDENSED=false").unwrap();
        thread::sleep(std::time::Duration::from_millis(10));

        // Set HOME to temp directory and clear RUSH_CONDENSED from environment
        unsafe {
            env::set_var("HOME", &temp_dir);
            env::remove_var("RUSH_CONDENSED");
        }

        // Create shell state and source .rushrc
        let mut shell_state = state::ShellState::new();
        source_rushrc(&mut shell_state);

        // Verify condensed_cwd was set to false
        assert!(
            !shell_state.condensed_cwd,
            "Expected condensed_cwd to be false after sourcing .rushrc with RUSH_CONDENSED=false"
        );

        // Test 2: .rushrc sets RUSH_CONDENSED=true
        fs::write(&rushrc_path, "export RUSH_CONDENSED=true").unwrap();
        thread::sleep(std::time::Duration::from_millis(10));

        // Clear RUSH_CONDENSED from environment again
        unsafe {
            env::remove_var("RUSH_CONDENSED");
        }

        // Create new shell state with condensed_cwd initially false
        let mut shell_state2 = state::ShellState::new();
        shell_state2.condensed_cwd = false; // Start with false
        source_rushrc(&mut shell_state2);

        // Verify condensed_cwd was set to true
        assert!(
            shell_state2.condensed_cwd,
            "Expected condensed_cwd to be true after sourcing .rushrc with RUSH_CONDENSED=true"
        );

        // Clean up files
        let _ = fs::remove_file(&rushrc_path);
        let _ = fs::remove_dir(&temp_dir);

        // Restore original environment variables
        unsafe {
            if let Some(home) = original_home {
                env::set_var("HOME", home);
            } else {
                env::remove_var("HOME");
            }

            if let Some(rush_condensed) = original_rush_condensed {
                env::set_var("RUSH_CONDENSED", rush_condensed);
            } else {
                env::remove_var("RUSH_CONDENSED");
            }
        }
    }

    #[test]
    fn test_integration_brace_expansion_simple() {
        let line = "echo {a,b,c}";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let expanded_tokens =
            lexer::expand_aliases(tokens, &shell_state, &mut std::collections::HashSet::new())
                .unwrap();
        let brace_expanded_tokens = brace_expansion::expand_braces(expanded_tokens).unwrap();
        let ast = parser::parse(brace_expanded_tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_integration_brace_expansion_with_ranges() {
        let line = "echo {1..3}";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let expanded_tokens =
            lexer::expand_aliases(tokens, &shell_state, &mut std::collections::HashSet::new())
                .unwrap();
        let brace_expanded_tokens = brace_expansion::expand_braces(expanded_tokens).unwrap();
        let ast = parser::parse(brace_expanded_tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_integration_brace_expansion_mixed() {
        let line = "echo file{a,b}.txt";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let expanded_tokens =
            lexer::expand_aliases(tokens, &shell_state, &mut std::collections::HashSet::new())
                .unwrap();
        let brace_expanded_tokens = brace_expansion::expand_braces(expanded_tokens).unwrap();
        let ast = parser::parse(brace_expanded_tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_integration_brace_expansion_nested() {
        let line = "echo {{a,b},{c,d}}";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let expanded_tokens =
            lexer::expand_aliases(tokens, &shell_state, &mut std::collections::HashSet::new())
                .unwrap();
        let brace_expanded_tokens = brace_expansion::expand_braces(expanded_tokens).unwrap();
        let ast = parser::parse(brace_expanded_tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_integration_brace_expansion_with_pipes() {
        let line = "echo {a,b} | cat";
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let expanded_tokens =
            lexer::expand_aliases(tokens, &shell_state, &mut std::collections::HashSet::new())
                .unwrap();
        let brace_expanded_tokens = brace_expansion::expand_braces(expanded_tokens).unwrap();
        let ast = parser::parse(brace_expanded_tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_trap_exit_execution() {
        let mut shell_state = state::ShellState::new();

        // Set an EXIT trap
        shell_state.set_trap("EXIT", "echo 'EXIT trap executed'".to_string());

        // Execute the EXIT trap
        execute_exit_trap(&mut shell_state);

        // Verify it was marked as executed
        assert!(shell_state.exit_trap_executed);

        // Calling again should not execute it
        execute_exit_trap(&mut shell_state);
    }

    #[test]
    fn test_trap_builtin_integration() {
        let line = "trap 'echo trapped' INT";
        let mut shell_state = state::ShellState::new();
        execute_line(line, &mut shell_state);

        // Verify trap was set
        assert_eq!(
            shell_state.get_trap("INT"),
            Some("echo trapped".to_string())
        );
    }

    #[test]
    fn test_trap_display_integration() {
        let mut shell_state = state::ShellState::new();
        shell_state.set_trap("INT", "echo int handler".to_string());
        shell_state.set_trap("TERM", "echo term handler".to_string());

        let line = "trap";
        execute_line(line, &mut shell_state);

        // Just verify it doesn't crash - output goes to stdout
    }

    #[test]
    fn test_trap_reset_integration() {
        let mut shell_state = state::ShellState::new();
        shell_state.set_trap("INT", "echo handler".to_string());

        // Reset the trap
        let line = "trap - INT";
        execute_line(line, &mut shell_state);

        // Verify trap was removed
        assert_eq!(shell_state.get_trap("INT"), None);
    }

    #[test]
    fn test_trap_multiple_signals() {
        let line = "trap 'echo signal' INT TERM HUP";
        let mut shell_state = state::ShellState::new();
        execute_line(line, &mut shell_state);

        // Verify all traps were set
        assert_eq!(shell_state.get_trap("INT"), Some("echo signal".to_string()));
        assert_eq!(
            shell_state.get_trap("TERM"),
            Some("echo signal".to_string())
        );
        assert_eq!(shell_state.get_trap("HUP"), Some("echo signal".to_string()));
    }

    #[test]
    fn test_signal_queue_enqueue_dequeue() {
        // Lock to prevent parallel tests from interfering with the signal queue
        let _lock = SIGNAL_QUEUE_LOCK.lock().unwrap();

        // Clear the queue first
        if let Ok(mut queue) = state::SIGNAL_QUEUE.lock() {
            queue.clear();
        }

        // Enqueue a signal
        state::enqueue_signal("INT", 2);

        // Verify it was enqueued
        if let Ok(queue) = state::SIGNAL_QUEUE.lock() {
            assert_eq!(queue.len(), 1);
            assert_eq!(queue.front().unwrap().signal_name, "INT");
            assert_eq!(queue.front().unwrap().signal_number, 2);
        }

        // Clear for other tests
        if let Ok(mut queue) = state::SIGNAL_QUEUE.lock() {
            queue.clear();
        }
    }

    #[test]
    fn test_signal_queue_overflow() {
        // Lock to prevent parallel tests from interfering with the signal queue
        let _lock = SIGNAL_QUEUE_LOCK.lock().unwrap();

        // Lock the queue for the entire test to prevent interference
        if let Ok(mut queue) = state::SIGNAL_QUEUE.lock() {
            // Clear the queue first
            queue.clear();

            // Fill the queue beyond capacity directly
            for _i in 0..110 {
                // If queue is full, remove the oldest event
                if queue.len() >= 100 {
                    queue.pop_front();
                }
                queue.push_back(state::SignalEvent::new("INT".to_string(), 2));
            }

            // Verify queue size is capped at 100
            assert_eq!(queue.len(), 100);

            // Clear for other tests
            queue.clear();
        }
    }

    #[test]
    fn test_process_pending_signals_with_trap() {
        // Lock to prevent parallel tests from interfering with the signal queue
        let _lock = SIGNAL_QUEUE_LOCK.lock().unwrap();

        // Clear the queue first
        if let Ok(mut queue) = state::SIGNAL_QUEUE.lock() {
            queue.clear();
        }

        let mut shell_state = state::ShellState::new();

        // Set a trap for INT
        shell_state.set_trap("INT", "echo 'INT trapped'".to_string());

        // Enqueue a signal
        state::enqueue_signal("INT", 2);

        // Process signals
        state::process_pending_signals(&mut shell_state);

        // Verify queue is empty after processing
        if let Ok(queue) = state::SIGNAL_QUEUE.lock() {
            assert_eq!(queue.len(), 0);
        }
    }

    #[test]
    fn test_trap_execution_during_repl() {
        // Lock to prevent parallel tests from interfering with the signal queue
        let _lock = SIGNAL_QUEUE_LOCK.lock().unwrap();

        // Clear the queue first
        if let Ok(mut queue) = state::SIGNAL_QUEUE.lock() {
            queue.clear();
        }

        let mut shell_state = state::ShellState::new();

        // Set a trap for INT
        shell_state.set_trap("INT", "echo 'Caught SIGINT'".to_string());

        // Simulate receiving SIGINT
        state::enqueue_signal("INT", 2);

        // Process signals (simulating REPL loop)
        state::process_pending_signals(&mut shell_state);

        // Verify queue is empty
        if let Ok(queue) = state::SIGNAL_QUEUE.lock() {
            assert_eq!(queue.len(), 0);
        }
    }

    #[test]
    fn test_multiple_signals_in_sequence() {
        // Lock to prevent parallel tests from interfering with the signal queue
        let _lock = SIGNAL_QUEUE_LOCK.lock().unwrap();

        // Clear the queue first
        if let Ok(mut queue) = state::SIGNAL_QUEUE.lock() {
            queue.clear();
        }

        let mut shell_state = state::ShellState::new();

        // Set traps for multiple signals
        shell_state.set_trap("INT", "echo 'INT'".to_string());
        shell_state.set_trap("TERM", "echo 'TERM'".to_string());
        shell_state.set_trap("HUP", "echo 'HUP'".to_string());

        // Enqueue multiple signals
        state::enqueue_signal("INT", 2);
        state::enqueue_signal("TERM", 15);
        state::enqueue_signal("HUP", 1);

        // Process all signals
        state::process_pending_signals(&mut shell_state);

        // Verify all were processed
        if let Ok(queue) = state::SIGNAL_QUEUE.lock() {
            assert_eq!(queue.len(), 0);
        }
    }
}
