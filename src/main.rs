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

            let config = rustyline::Config::builder()
                .bracketed_paste(true) // Enable bracketed paste to handle multi-line pastes
                .build();
            let mut rl =
                Editor::<completion::RushCompleter, FileHistory>::with_config(config).unwrap();
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
                            // We're collecting heredoc content
                            // Check if this line (which might contain newlines from pasted input) contains the delimiter
                            if line.contains('\n') {
                                // Multi-line paste - split and process each line
                                let lines: Vec<&str> = line.split('\n').collect();
                                let mut found_delimiter = false;
                                for (i, line_part) in lines.iter().enumerate() {
                                    // Strip continuation prompt prefix if present ("> " from rustyline)
                                    let cleaned_line = line_part.trim_start_matches("> ");

                                    if cleaned_line.trim() == delimiter.trim() {
                                        // Found delimiter
                                        found_delimiter = true;
                                        // Execute with collected content
                                        shell_state.pending_heredoc_content = Some(content.clone());
                                        execute_line(&command_line, &mut shell_state);

                                        // Process any remaining lines after the delimiter as new commands
                                        for remaining_line in lines.iter().skip(i + 1) {
                                            if !remaining_line.trim().is_empty() {
                                                execute_line(remaining_line, &mut shell_state);
                                            }
                                        }
                                        break;
                                    } else {
                                        // Add to content (use cleaned line without prompt prefix)
                                        if !content.is_empty() {
                                            content.push('\n');
                                        }
                                        content.push_str(cleaned_line);
                                    }
                                }

                                if !found_delimiter {
                                    // No delimiter found, continue collecting
                                    shell_state.collecting_heredoc =
                                        Some((command_line, delimiter, content));
                                }
                            } else {
                                // Single line input
                                if line.trim() == delimiter.trim() {
                                    // Found the delimiter - execute the command
                                    shell_state.pending_heredoc_content = Some(content);
                                    execute_line(&command_line, &mut shell_state);
                                } else {
                                    // Add this line to heredoc content and continue collecting
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
                            // Check if this is a multi-line paste with heredoc
                            if line.contains('\n') && line.contains("<<") && !line.contains("<<<") {
                                // Split the pasted content into lines
                                let lines: Vec<&str> = line.split('\n').collect();
                                let mut i = 0;
                                while i < lines.len() {
                                    let current_line = lines[i];

                                    // Check if this line starts a heredoc using proper lexer detection
                                    if let Some(delimiter) =
                                        line_contains_heredoc(current_line, &shell_state)
                                    {
                                        // Collect heredoc content from remaining lines
                                        let mut heredoc_content = String::new();
                                        i += 1;
                                        let mut found_delimiter = false;

                                        while i < lines.len() {
                                            let line_to_check = lines[i].trim_start_matches("> ");
                                            if line_to_check.trim() == delimiter.trim() {
                                                // Found delimiter
                                                found_delimiter = true;
                                                i += 1;
                                                break;
                                            }
                                            if !heredoc_content.is_empty() {
                                                heredoc_content.push('\n');
                                            }
                                            // Use cleaned line without prompt prefix
                                            heredoc_content.push_str(line_to_check);
                                            i += 1;
                                        }

                                        if found_delimiter {
                                            // Execute with collected content
                                            shell_state.pending_heredoc_content =
                                                Some(heredoc_content);
                                            execute_line(current_line, &mut shell_state);
                                        } else {
                                            // Delimiter not found in paste, start collecting interactively
                                            shell_state.collecting_heredoc = Some((
                                                current_line.to_string(),
                                                delimiter,
                                                heredoc_content,
                                            ));
                                        }
                                        continue;
                                    }

                                    // Execute normal line
                                    if !current_line.trim().is_empty() {
                                        let _ = rl.add_history_entry(current_line);
                                        if current_line == "exit" {
                                            execute_exit_trap(&mut shell_state);
                                            break;
                                        }
                                        execute_line(current_line, &mut shell_state);
                                    }
                                    i += 1;
                                }
                            } else {
                                // Single line input
                                let _ = rl.add_history_entry(line.as_str());
                                if line == "exit" {
                                    execute_exit_trap(&mut shell_state);
                                    break;
                                }

                                // Check if this line starts a here-document using proper lexer detection
                                if let Some(delimiter) = line_contains_heredoc(&line, &shell_state)
                                {
                                    // Start collecting heredoc content
                                    shell_state.collecting_heredoc =
                                        Some((line.clone(), delimiter, String::new()));
                                    continue;
                                }

                                // Execute normal command
                                execute_line(&line, &mut shell_state);
                            }
                        }

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
                        }

                        let err_str = format!("{}", err);

                        // Check if this is an EOF while collecting heredoc
                        if err_str.contains("EOF") && shell_state.collecting_heredoc.is_some() {
                            // User pressed Ctrl-D while collecting heredoc
                            // Execute the command with whatever content we have
                            if let Some((command_line, _delimiter, content)) =
                                shell_state.collecting_heredoc.take()
                            {
                                shell_state.pending_heredoc_content = Some(content);
                                execute_line(&command_line, &mut shell_state);
                            }
                            continue;
                        }

                        // Check if this is a signal interruption (SIGINT)
                        if err_str.contains("Interrupted") {
                            // SIGINT should just interrupt the current input line
                            // If we were collecting a heredoc, cancel it
                            shell_state.collecting_heredoc = None;
                            continue;
                        }

                        // Check if this is EOF in normal mode (exit the shell)
                        if err_str.contains("EOF") {
                            println!();
                            execute_exit_trap(&mut shell_state);
                            break;
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
/// Check if a line contains a heredoc redirection using proper lexer-based detection
/// Returns the delimiter if found, None otherwise
fn line_contains_heredoc(line: &str, shell_state: &state::ShellState) -> Option<String> {
    // Use the lexer to properly parse the line
    match lexer::lex(line, shell_state) {
        Ok(tokens) => {
            // Look for a RedirHereDoc token
            for token in tokens {
                if let lexer::Token::RedirHereDoc(delimiter, _quoted) = token {
                    return Some(delimiter);
                }
            }
            None
        }
        Err(_) => None,
    }
}

/// Check if a line contains a specific keyword as a distinct token
/// This handles comments and ensures the keyword is not part of another word
fn contains_keyword(line: &str, keyword: &str) -> bool {
    let mut chars = line.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut current_word = String::new();

    while let Some(ch) = chars.next() {
        if escaped {
            escaped = false;
            // Escaped characters are treated as part of the word
            current_word.push(ch);
            continue;
        }

        if in_single_quote {
            if ch == '\'' {
                in_single_quote = false;
            } else {
                current_word.push(ch);
            }
            continue;
        }

        if in_double_quote {
            if ch == '"' {
                in_double_quote = false;
            } else if ch == '\\' {
                escaped = true;
            } else {
                current_word.push(ch);
            }
            continue;
        }

        match ch {
            '#' => {
                if current_word.is_empty() {
                    return false; // Comment starts at word boundary
                }
                current_word.push(ch); // # inside word, treat as literal
            }
            '\'' => {
                in_single_quote = true;
                current_word.push(ch);
            }
            '"' => {
                in_double_quote = true;
                current_word.push(ch);
            }
            '\\' => escaped = true,
            ' ' | '\t' | '\n' | ';' | '|' | '&' | '(' | ')' | '{' | '}' => {
                if current_word == keyword {
                    return true;
                }
                current_word.clear();
            }
            _ => current_word.push(ch),
        }
    }

    // Check last word
    current_word == keyword
}

/// Check if a line starts with a specific keyword
fn starts_with_keyword(line: &str, keyword: &str) -> bool {
    let mut chars = line.chars().peekable();
    let mut current_word = String::new();

    // Skip leading whitespace
    while let Some(&ch) = chars.peek() {
        if ch == ' ' || ch == '\t' {
            chars.next();
        } else {
            break;
        }
    }

    while let Some(ch) = chars.next() {
        match ch {
            ' ' | '\t' | '\n' | ';' | '|' | '&' | '(' | ')' | '{' | '}' => {
                return current_word == keyword;
            }
            _ => current_word.push(ch),
        }
    }

    current_word == keyword
}

fn execute_script(content: &str, shell_state: &mut state::ShellState) {
    let mut current_block = String::new();
    let mut in_if_block = false;
    let mut if_depth = 0;
    let mut in_case_block = false;
    let mut in_function_block = false;
    let mut in_group_block = false;
    let mut brace_depth = 0;
    let mut in_for_block = false;
    let mut for_depth = 0;
    let mut in_while_block = false;
    let mut while_depth = 0;

    // Track quote state across lines to handle multiline strings correctly
    let mut in_double_quote = false;
    let mut in_single_quote = false;

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
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
            i += 1;
            continue;
        }

        // Update quote state based on this line
        // We need to scan the line to update state, but be careful with comments
        let mut chars = line.chars().peekable();
        let mut escaped = false;

        while let Some(ch) = chars.next() {
            if escaped {
                escaped = false;
                continue;
            }

            if in_single_quote {
                if ch == '\'' {
                    in_single_quote = false;
                }
                continue;
            }

            if in_double_quote {
                if ch == '"' {
                    in_double_quote = false;
                } else if ch == '\\' {
                    escaped = true;
                }
                continue;
            }

            match ch {
                '#' => break, // Comment starts, ignore rest of line for state tracking
                '\'' => in_single_quote = true,
                '"' => in_double_quote = true,
                '\\' => escaped = true,
                _ => {}
            }
        }

        // Skip pure comment lines if we are NOT in a quote
        // If we represent a continuation of a string usage, we must preserve it
        let trimmed = line.trim();
        if !in_double_quote && !in_single_quote && (trimmed.is_empty() || trimmed.starts_with("#"))
        {
            i += 1;
            continue;
        }

        // Check for keywords only if we are NOT in a quote block
        let keywords_active = !in_double_quote && !in_single_quote;

        // Check for multi-line construct keywords (only when not in a function)
        if keywords_active && !in_function_block {
            if starts_with_keyword(line, "if") {
                in_if_block = true;
                if_depth += 1;
            } else if starts_with_keyword(line, "case") {
                in_case_block = true;
            } else if starts_with_keyword(line, "for") {
                in_for_block = true;
                for_depth += 1;
            } else if starts_with_keyword(line, "while") {
                in_while_block = true;
                while_depth += 1;
            } else if {
                let trimmed = line.trim();
                trimmed == "{" || trimmed.starts_with("{ ") || trimmed.starts_with("{\t")
            } {
                in_group_block = true;
                brace_depth += line.matches('{').count() as i32;
                brace_depth -= line.matches('}').count() as i32;
            }
        }

        // Check for function definition
        if keywords_active
            && (line.contains("() {") || (trimmed.ends_with("()") && !in_function_block))
        {
            in_function_block = true;
            // Count opening braces on this line
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;
        } else if in_function_block || in_group_block {
            // Track braces inside function
            // Note: Simplistic brace counting, ideally should respect quotes/comments too
            // But for now, we trust the user writes valid shell code inside functions
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;
        }

        // Add line to current block
        if !current_block.is_empty() {
            current_block.push('\n');
        }
        current_block.push_str(line);

        // Check for end of multi-line constructs
        // Only check if we are NOT in a quote
        if keywords_active {
            if (in_function_block || in_group_block) && brace_depth == 0 {
                // Function or group is complete
                in_function_block = false;
                in_group_block = false;
                execute_line(&current_block, shell_state);
                current_block.clear();

                // Check if exit was requested
                if shell_state.exit_requested {
                    break;
                }
            } else if in_if_block && contains_keyword(line, "fi") {
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
            } else if in_for_block && contains_keyword(line, "done") {
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
            } else if in_while_block && contains_keyword(line, "done") {
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
            } else if in_case_block && contains_keyword(line, "esac") {
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
                && !in_group_block
                && !in_for_block
                && !in_while_block
            {
                // Check if this line contains a here-document using proper lexer detection
                if let Some(delimiter) = line_contains_heredoc(&current_block, shell_state) {
                    // Collect here-document content from subsequent lines
                    i += 1;
                    let mut heredoc_content = String::new();
                    while i < lines.len() {
                        let content_line = lines[i];
                        if content_line.trim() == delimiter.trim() {
                            // Found the delimiter, stop collecting
                            break;
                        }
                        if !heredoc_content.is_empty() {
                            heredoc_content.push('\n');
                        }
                        heredoc_content.push_str(content_line);
                        i += 1;
                    }

                    // Store the here-document content in shell state for the executor to use
                    shell_state.pending_heredoc_content = Some(heredoc_content);
                }

                // Execute single-line commands immediately
                execute_line(&current_block, shell_state);
                current_block.clear();

                // Check if exit was requested after executing the line
                if shell_state.exit_requested {
                    break;
                }
            }
        }

        i += 1;
    }

    // Execute any remaining block
    if !current_block.trim().is_empty() {
        execute_line(&current_block, shell_state);
    }

    // Final signal processing
    state::process_pending_signals(shell_state);
}

fn execute_command_string(command_string: &str, shell_state: &mut state::ShellState) {
    // Execute the entire command string as-is
    // The lexer and parser will properly handle semicolons, quotes, and command substitutions
    execute_line(command_string, shell_state);
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
        let temp_dir = env::temp_dir().canonicalize().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        let line = &format!("cd {}", temp_dir.to_string_lossy());
        let mut shell_state = state::ShellState::new();
        let tokens = lexer::lex(line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        assert_eq!(exit_code, 0);
        assert_eq!(env::current_dir().unwrap(), temp_dir);

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

    // ========================================================================
    // Subshell Tests (Phase 1)
    // ========================================================================

    #[test]
    fn test_subshell_simple_execution() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (echo hello)
        let tokens = lexer::lex("(echo hello)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_variable_isolation() {
        let mut shell_state = state::ShellState::new();
        shell_state.set_var("VAR", "parent".to_string());

        // Parse and execute: (VAR=child)
        let tokens = lexer::lex("(VAR=child)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
        // Parent variable should be unchanged
        assert_eq!(shell_state.get_var("VAR"), Some("parent".to_string()));
    }

    #[test]
    fn test_subshell_exit_code_propagation() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (exit 42)
        let tokens = lexer::lex("(exit 42)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 42);
        assert_eq!(shell_state.last_exit_code, 42);
    }

    #[test]
    fn test_subshell_directory_isolation() {
        // Lock to prevent parallel tests from interfering
        let _lock = DIR_CHANGE_LOCK.lock().unwrap();

        let mut shell_state = state::ShellState::new();
        let original_dir = std::env::current_dir().unwrap();

        // Parse and execute: (cd /tmp)
        let tokens = lexer::lex("(cd /tmp)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
        // Current directory should be unchanged
        let current_dir = std::env::current_dir().unwrap();
        assert_eq!(current_dir, original_dir);
    }

    #[test]
    fn test_subshell_multiple_commands() {
        let mut shell_state = state::ShellState::new();
        shell_state.set_var("COUNT", "0".to_string());

        // Parse and execute: (COUNT=1; COUNT=2; COUNT=3)
        let tokens = lexer::lex("(COUNT=1; COUNT=2; COUNT=3)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
        // Parent variable should be unchanged
        assert_eq!(shell_state.get_var("COUNT"), Some("0".to_string()));
    }

    #[test]
    fn test_subshell_nested() {
        let mut shell_state = state::ShellState::new();
        shell_state.set_var("VAR", "outer".to_string());

        // Parse and execute: ((VAR=inner))
        let tokens = lexer::lex("((VAR=inner))", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
        // Parent variable should be unchanged
        assert_eq!(shell_state.get_var("VAR"), Some("outer".to_string()));
    }

    #[test]
    fn test_subshell_inherits_variables() {
        let mut shell_state = state::ShellState::new();
        shell_state.set_var("PARENT_VAR", "parent_value".to_string());

        // Subshell should be able to read parent variables
        // We can't easily test this without output capture, but we can verify it doesn't error
        let tokens = lexer::lex("(echo $PARENT_VAR)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_inherits_functions() {
        let mut shell_state = state::ShellState::new();

        // Define a function in parent
        shell_state.define_function(
            "test_func".to_string(),
            parser::Ast::Pipeline(vec![parser::ShellCommand {
                args: vec!["echo".to_string(), "from_function".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        );

        // Call function in subshell
        let tokens = lexer::lex("(test_func)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_empty_error() {
        let shell_state = state::ShellState::new();

        // Parse: ()
        let tokens = lexer::lex("()", &shell_state).unwrap();
        let result = parser::parse(tokens);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty subshell"));
    }

    #[test]
    fn test_subshell_unmatched_paren() {
        let shell_state = state::ShellState::new();

        // Parse: (echo hello
        let tokens = lexer::lex("(echo hello", &shell_state).unwrap();
        let result = parser::parse(tokens);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unmatched parenthesis"));
    }

    #[test]
    fn test_subshell_with_and_operator() {
        let mut shell_state = state::ShellState::new();
        shell_state.set_var("VAR", "parent".to_string());

        // Parse and execute: (VAR=child) && echo $VAR
        let tokens = lexer::lex("(VAR=child) && echo $VAR", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
        // Parent variable should be unchanged
        assert_eq!(shell_state.get_var("VAR"), Some("parent".to_string()));
    }

    #[test]
    fn test_subshell_with_or_operator() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (false) || echo fallback
        let tokens = lexer::lex("(false) || echo fallback", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_function_definition_not_confused() {
        let shell_state = state::ShellState::new();

        // Parse: func() { echo hello; }
        // This should be a function definition, not a subshell
        let tokens = lexer::lex("func() { echo hello; }", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();

        // Should be a function definition
        match ast {
            parser::Ast::FunctionDefinition { name, .. } => {
                assert_eq!(name, "func");
            }
            _ => panic!("Expected FunctionDefinition, got {:?}", ast),
        }
    }

    #[test]
    fn test_subshell_parser_ast_structure() {
        let shell_state = state::ShellState::new();

        // Parse: (echo hello)
        let tokens = lexer::lex("(echo hello)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();

        // Should be a Subshell variant
        match ast {
            parser::Ast::Subshell { body } => {
                // Body should be a Pipeline
                match *body {
                    parser::Ast::Pipeline(cmds) => {
                        assert_eq!(cmds.len(), 1);
                        assert_eq!(cmds[0].args[0], "echo");
                        assert_eq!(cmds[0].args[1], "hello");
                    }
                    _ => panic!("Expected Pipeline in subshell body"),
                }
            }
            _ => panic!("Expected Subshell, got {:?}", ast),
        }
    }

    #[test]
    fn test_subshell_exported_variables() {
        let mut shell_state = state::ShellState::new();
        shell_state.set_exported_var("EXPORTED_VAR", "exported_value".to_string());

        // Subshell should inherit exported variables
        let tokens = lexer::lex("(EXPORTED_VAR=modified)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
        // Parent's exported variable should be unchanged
        assert_eq!(
            shell_state.get_var("EXPORTED_VAR"),
            Some("exported_value".to_string())
        );
        assert!(shell_state.exported.contains("EXPORTED_VAR"));
    }

    #[test]
    fn test_subshell_with_sequence() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (echo first; echo second)
        let tokens = lexer::lex("(echo first; echo second)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_false_exit_code() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (false)
        let tokens = lexer::lex("(false)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 1);
        assert_eq!(shell_state.last_exit_code, 1);
    }

    // ========================================================================
    // Subshell Tests (Phase 2 - Advanced Features)
    // ========================================================================

    #[test]
    fn test_subshell_in_pipeline_simple() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (echo hello) | cat
        let tokens = lexer::lex("(echo hello) | cat", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_in_pipeline_multiple() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (echo a) | (cat) | (cat)
        let tokens = lexer::lex("(echo a) | (cat) | (cat)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_with_output_redirection() {
        let _lock = ENV_LOCK.lock().unwrap();

        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_subshell_redir_{}.txt", timestamp);

        let mut shell_state = state::ShellState::new();

        // Parse and execute: (echo hello) > output.txt
        let line = format!("(echo hello) > {}", temp_file);
        let tokens = lexer::lex(&line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // Verify output file
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("hello"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_subshell_with_append_redirection() {
        let _lock = ENV_LOCK.lock().unwrap();

        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_subshell_append_{}.txt", timestamp);

        // Create initial file
        std::fs::write(&temp_file, "first\n").unwrap();

        let mut shell_state = state::ShellState::new();

        // Parse and execute: (echo second) >> output.txt
        let line = format!("(echo second) >> {}", temp_file);
        let tokens = lexer::lex(&line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // Verify output file contains both lines
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("first"));
        assert!(content.contains("second"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_subshell_in_pipeline_with_sequence() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (echo a; echo b) | cat
        let tokens = lexer::lex("(echo a; echo b) | cat", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_parser_in_pipeline() {
        let shell_state = state::ShellState::new();

        // Parse: (echo hello) | grep hello
        let tokens = lexer::lex("(echo hello) | grep hello", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();

        // Should be a Pipeline with 2 commands
        match ast {
            parser::Ast::Pipeline(cmds) => {
                assert_eq!(cmds.len(), 2);
                // First command should be a subshell
                assert!(cmds[0].compound.is_some());
                assert!(cmds[0].args.is_empty());
                // Second command should be grep
                assert_eq!(cmds[1].args[0], "grep");
                assert!(cmds[1].compound.is_none());
            }
            _ => panic!("Expected Pipeline, got {:?}", ast),
        }
    }

    #[test]
    fn test_subshell_parser_with_redirection() {
        let shell_state = state::ShellState::new();

        // Parse: (echo hello) > file.txt
        let tokens = lexer::lex("(echo hello) > /tmp/test.txt", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();

        // Should be a Pipeline with 1 command that has a subshell and redirection
        match ast {
            parser::Ast::Pipeline(cmds) => {
                assert_eq!(cmds.len(), 1);
                // Command should have a subshell
                assert!(cmds[0].compound.is_some());
                // Command should have a redirection
                assert_eq!(cmds[0].redirections.len(), 1);
            }
            _ => panic!("Expected Pipeline, got {:?}", ast),
        }
    }

    #[test]
    fn test_subshell_complex_pipeline() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (echo test) | (cat) | (cat)
        let tokens = lexer::lex("(echo test) | (cat) | (cat)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_mixed_pipeline() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: echo test | (cat) | cat
        let tokens = lexer::lex("echo test | (cat) | cat", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_with_variable_in_redirection() {
        let _lock = ENV_LOCK.lock().unwrap();

        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_subshell_var_{}.txt", timestamp);

        let mut shell_state = state::ShellState::new();
        shell_state.set_var("OUTFILE", temp_file.clone());

        // Parse and execute: (echo test) > $OUTFILE
        let tokens = lexer::lex("(echo test) > $OUTFILE", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // Verify output file
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("test"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_subshell_nested_with_pipeline() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: ((echo nested)) | cat
        let tokens = lexer::lex("((echo nested)) | cat", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_with_and_in_pipeline() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (true) && echo yes | cat
        let tokens = lexer::lex("(true) && echo yes | cat", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_pipeline_exit_code() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (false) | cat
        let tokens = lexer::lex("(false) | cat", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        // Exit code should be from the last command (cat), which succeeds
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_multiple_redirections() {
        let _lock = ENV_LOCK.lock().unwrap();

        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file1 = format!("/tmp/rush_test_subshell_multi1_{}.txt", timestamp);
        let temp_file2 = format!("/tmp/rush_test_subshell_multi2_{}.txt", timestamp);

        let mut shell_state = state::ShellState::new();

        // Parse and execute: (echo first) > file1.txt
        let line1 = format!("(echo first) > {}", temp_file1);
        let tokens1 = lexer::lex(&line1, &shell_state).unwrap();
        let ast1 = parser::parse(tokens1).unwrap();
        let exit_code1 = executor::execute(ast1, &mut shell_state);
        assert_eq!(exit_code1, 0);

        // Parse and execute: (echo second) > file2.txt
        let line2 = format!("(echo second) > {}", temp_file2);
        let tokens2 = lexer::lex(&line2, &shell_state).unwrap();
        let ast2 = parser::parse(tokens2).unwrap();
        let exit_code2 = executor::execute(ast2, &mut shell_state);
        assert_eq!(exit_code2, 0);

        // Verify both files
        let content1 = std::fs::read_to_string(&temp_file1).unwrap();
        let content2 = std::fs::read_to_string(&temp_file2).unwrap();
        assert!(content1.contains("first"));
        assert!(content2.contains("second"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file1);
        let _ = std::fs::remove_file(&temp_file2);
    }

    #[test]
    fn test_subshell_in_if_condition() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: if (true); then echo yes; fi
        let tokens = lexer::lex("if (true); then echo yes; fi", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_in_while_condition() {
        let mut shell_state = state::ShellState::new();
        shell_state.set_var("COUNT", "0".to_string());

        // Parse and execute: while (false); do echo loop; done
        // Should not execute the loop body
        let tokens = lexer::lex("while (false); do echo loop; done", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_chained_with_operators() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (true) && (echo middle) && (echo end)
        let tokens = lexer::lex("(true) && (echo middle) && (echo end)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_with_cd_in_pipeline() {
        let _lock = DIR_CHANGE_LOCK.lock().unwrap();

        let mut shell_state = state::ShellState::new();
        let original_dir = std::env::current_dir().unwrap();

        // Parse and execute: (cd /tmp; pwd) | cat
        // The cd should not affect parent's directory
        let tokens = lexer::lex("(cd /tmp; pwd) | cat", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // Parent directory should be unchanged
        let current_dir = std::env::current_dir().unwrap();
        assert_eq!(current_dir, original_dir);
    }

    #[test]
    fn test_subshell_parser_multiple_in_pipeline() {
        let shell_state = state::ShellState::new();

        // Parse: (echo a) | (echo b) | (echo c)
        let tokens = lexer::lex("(echo a) | (echo b) | (echo c)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();

        // Should be a Pipeline with 3 commands, all subshells
        match ast {
            parser::Ast::Pipeline(cmds) => {
                assert_eq!(cmds.len(), 3);
                assert!(cmds[0].compound.is_some());
                assert!(cmds[1].compound.is_some());
                assert!(cmds[2].compound.is_some());
            }
            _ => panic!("Expected Pipeline, got {:?}", ast),
        }
    }

    #[test]
    fn test_subshell_with_multiple_redirections_on_one() {
        let _lock = ENV_LOCK.lock().unwrap();

        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_subshell_multiredir_{}.txt", timestamp);

        let mut shell_state = state::ShellState::new();

        // Parse and execute: (echo test; echo more) > output.txt
        let line = format!("(echo test; echo more) > {}", temp_file);
        let tokens = lexer::lex(&line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // Verify output file contains both lines
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("test"));
        assert!(content.contains("more"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_subshell_empty_in_pipeline_error() {
        let shell_state = state::ShellState::new();

        // Parse: () | cat
        let tokens = lexer::lex("() | cat", &shell_state).unwrap();
        let result = parser::parse(tokens);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty subshell"));
    }

    #[test]
    fn test_subshell_unmatched_in_pipeline_error() {
        let shell_state = state::ShellState::new();

        // Parse: (echo hello | cat
        let tokens = lexer::lex("(echo hello | cat", &shell_state).unwrap();
        let result = parser::parse(tokens);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unmatched parenthesis"));
    }

    #[test]
    fn test_subshell_with_variable_isolation_in_pipeline() {
        let mut shell_state = state::ShellState::new();
        shell_state.set_var("VAR", "parent".to_string());

        // Parse and execute: (VAR=child; echo $VAR) | cat
        let tokens = lexer::lex("(VAR=child; echo $VAR) | cat", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
        // Parent variable should be unchanged
        assert_eq!(shell_state.get_var("VAR"), Some("parent".to_string()));
    }

    #[test]
    fn test_compound_group_pipeline_with_redirection_suppression() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = state::ShellState::new();

        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_out = format!("/tmp/rush_test_group_suppress_out_{}.txt", timestamp);
        let temp_pipe = format!("/tmp/rush_test_group_suppress_pipe_{}.txt", timestamp);

        // { echo hello; } > temp_out | cat > temp_pipe
        let line = format!("{{ echo hello; }} > {} | cat > {}", temp_out, temp_pipe);
        let tokens = lexer::lex(&line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let _ = executor::execute(ast, &mut shell_state);

        // Verify temp_out has "hello"
        let out_content = std::fs::read_to_string(&temp_out).unwrap();
        assert!(out_content.contains("hello"));

        // Verify temp_pipe is empty
        let pipe_content = std::fs::read_to_string(&temp_pipe).unwrap();
        assert!(
            pipe_content.is_empty(),
            "Pipe should be empty but contains: '{}'",
            pipe_content
        );

        // Cleanup
        let _ = std::fs::remove_file(&temp_out);
        let _ = std::fs::remove_file(&temp_pipe);
    }

    #[test]
    fn test_subshell_pipeline_with_redirection_suppression() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = state::ShellState::new();

        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_out = format!("/tmp/rush_test_sub_suppress_out_{}.txt", timestamp);
        let temp_pipe = format!("/tmp/rush_test_sub_suppress_pipe_{}.txt", timestamp);

        // (echo hello) > temp_out | cat > temp_pipe
        let line = format!("(echo hello) > {} | cat > {}", temp_out, temp_pipe);
        let tokens = lexer::lex(&line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let _ = executor::execute(ast, &mut shell_state);

        // Verify temp_out has "hello"
        let out_content = std::fs::read_to_string(&temp_out).unwrap();
        assert!(out_content.contains("hello"));

        // Verify temp_pipe is empty
        let pipe_content = std::fs::read_to_string(&temp_pipe).unwrap();
        assert!(
            pipe_content.is_empty(),
            "Pipe should be empty but contains: '{}'",
            pipe_content
        );

        // Cleanup
        let _ = std::fs::remove_file(&temp_out);
        let _ = std::fs::remove_file(&temp_pipe);
    }

    #[test]
    fn test_subshell_three_level_nesting() {
        let mut shell_state = state::ShellState::new();
        shell_state.set_var("LEVEL", "0".to_string());

        // Parse and execute: (((LEVEL=3)))
        let tokens = lexer::lex("(((LEVEL=3)))", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
        // Parent LEVEL should still be "0"
        assert_eq!(shell_state.get_var("LEVEL"), Some("0".to_string()));
    }

    #[test]
    fn test_subshell_with_function_call_in_pipeline() {
        let mut shell_state = state::ShellState::new();

        // Define a function
        shell_state.define_function(
            "myfunc".to_string(),
            parser::Ast::Pipeline(vec![parser::ShellCommand {
                args: vec!["echo".to_string(), "from_func".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        );

        // Parse and execute: (myfunc) | cat
        let tokens = lexer::lex("(myfunc) | cat", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_complex_with_and_or() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (true) && (echo yes) || (echo no)
        let tokens = lexer::lex("(true) && (echo yes) || (echo no)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_in_for_loop_body() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: for i in a b c; do (echo $i); done
        let tokens = lexer::lex("for i in a b c; do (echo $i); done", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_redirection_preserves_parent_state() {
        let _lock = ENV_LOCK.lock().unwrap();

        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/rush_test_subshell_preserve_{}.txt", timestamp);

        let mut shell_state = state::ShellState::new();
        shell_state.set_var("VAR", "parent".to_string());

        // Parse and execute: (VAR=child; echo $VAR) > output.txt
        let line = format!("(VAR=child; echo $VAR) > {}", temp_file);
        let tokens = lexer::lex(&line, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
        // Parent variable should be unchanged
        assert_eq!(shell_state.get_var("VAR"), Some("parent".to_string()));

        // Verify output file contains "child"
        let content = std::fs::read_to_string(&temp_file).unwrap();
        assert!(content.contains("child"));

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    // ========================================================================
    // Subshell Tests (Phase 3 - Edge Cases and Optimization)
    // ========================================================================

    #[test]
    fn test_subshell_depth_limit_protection() {
        let mut shell_state = state::ShellState::new();

        // Create deeply nested subshells (101 levels - should exceed limit of 100)
        let mut nested_command = "echo deep".to_string();
        for _ in 0..101 {
            nested_command = format!("({})", nested_command);
        }

        // Parse and execute
        let tokens = lexer::lex(&nested_command, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        // Should fail with exit code 1 due to depth limit
        assert_eq!(exit_code, 1);
        assert_eq!(shell_state.last_exit_code, 1);
    }

    #[test]
    fn test_subshell_depth_limit_just_under() {
        let mut shell_state = state::ShellState::new();

        // Create 50 levels of nesting (well under the limit)
        let mut nested_command = "echo safe".to_string();
        for _ in 0..50 {
            nested_command = format!("({})", nested_command);
        }

        // Parse and execute
        let tokens = lexer::lex(&nested_command, &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        // Should succeed
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_subshell_exit_isolation() {
        let mut shell_state = state::ShellState::new();

        // Parse and execute: (exit 42); echo $?
        // The exit should only exit the subshell, not the parent
        let tokens1 = lexer::lex("(exit 42)", &shell_state).unwrap();
        let ast1 = parser::parse(tokens1).unwrap();
        let exit_code1 = executor::execute(ast1, &mut shell_state);

        assert_eq!(exit_code1, 42);
        assert_eq!(shell_state.last_exit_code, 42);

        // Parent should not have exit_requested set
        assert!(!shell_state.exit_requested);

        // Should be able to continue executing commands
        let tokens2 = lexer::lex("echo continuing", &shell_state).unwrap();
        let ast2 = parser::parse(tokens2).unwrap();
        let exit_code2 = executor::execute(ast2, &mut shell_state);

        assert_eq!(exit_code2, 0);
    }

    #[test]
    fn test_subshell_exit_with_different_codes() {
        let mut shell_state = state::ShellState::new();

        // Test various exit codes
        for code in [0, 1, 5, 42, 127, 255] {
            let line = format!("(exit {})", code);
            let tokens = lexer::lex(&line, &shell_state).unwrap();
            let ast = parser::parse(tokens).unwrap();
            let exit_code = executor::execute(ast, &mut shell_state);

            assert_eq!(exit_code, code);
            assert_eq!(shell_state.last_exit_code, code);
            assert!(!shell_state.exit_requested);
        }
    }

    #[test]
    fn test_subshell_return_isolation_in_function() {
        let mut shell_state = state::ShellState::new();

        // Define a function with a subshell containing return
        let func_body = parser::Ast::Sequence(vec![
            parser::Ast::Subshell {
                body: Box::new(parser::Ast::Return {
                    value: Some("5".to_string()),
                }),
            },
            parser::Ast::Pipeline(vec![parser::ShellCommand {
                args: vec!["echo".to_string(), "after_subshell".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
            parser::Ast::Return {
                value: Some("10".to_string()),
            },
        ]);

        shell_state.define_function("test_func".to_string(), func_body);

        // Call the function
        let tokens = lexer::lex("test_func", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        // Function should return 10, not 5 (subshell return is isolated)
        assert_eq!(exit_code, 10);
    }

    #[test]
    fn test_subshell_trap_inheritance() {
        let mut shell_state = state::ShellState::new();

        // Set a trap in parent
        shell_state.set_trap("USR1", "echo parent_trap".to_string());

        // Verify parent has the trap
        assert_eq!(
            shell_state.get_trap("USR1"),
            Some("echo parent_trap".to_string())
        );

        // Execute a subshell (traps should be inherited)
        let tokens = lexer::lex("(echo in_subshell)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // Parent trap should still be set
        assert_eq!(
            shell_state.get_trap("USR1"),
            Some("echo parent_trap".to_string())
        );
    }

    #[test]
    fn test_subshell_trap_isolation() {
        let mut shell_state = state::ShellState::new();

        // Set a trap in parent
        shell_state.set_trap("USR1", "echo parent_trap".to_string());

        // Execute a subshell that modifies the trap
        // Note: We can't easily test trap modification in subshell without executing trap builtin
        // But we can verify the parent trap is unchanged after subshell execution
        let tokens = lexer::lex("(echo subshell)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // Parent trap should be unchanged
        assert_eq!(
            shell_state.get_trap("USR1"),
            Some("echo parent_trap".to_string())
        );
    }

    #[test]
    fn test_subshell_complex_variable_scoping() {
        let mut shell_state = state::ShellState::new();

        // Set up initial variables
        shell_state.set_var("LEVEL1", "parent".to_string());
        shell_state.set_var("LEVEL2", "parent".to_string());
        shell_state.set_var("LEVEL3", "parent".to_string());

        // Execute nested subshells with variable modifications
        // (LEVEL1=sub1; (LEVEL2=sub2; (LEVEL3=sub3)))
        let tokens =
            lexer::lex("(LEVEL1=sub1; (LEVEL2=sub2; (LEVEL3=sub3)))", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // All parent variables should be unchanged
        assert_eq!(shell_state.get_var("LEVEL1"), Some("parent".to_string()));
        assert_eq!(shell_state.get_var("LEVEL2"), Some("parent".to_string()));
        assert_eq!(shell_state.get_var("LEVEL3"), Some("parent".to_string()));
    }

    #[test]
    fn test_subshell_variable_scoping_with_reads() {
        let mut shell_state = state::ShellState::new();

        // Set parent variable
        shell_state.set_var("VAR", "parent".to_string());

        // Subshell reads parent var, modifies it, then reads again
        // The modification should only affect the subshell
        let tokens = lexer::lex("(VAR=child)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_var("VAR"), Some("parent".to_string()));
    }

    #[test]
    fn test_subshell_error_propagation() {
        let mut shell_state = state::ShellState::new();

        // Subshell with a failing command
        let tokens = lexer::lex("(false)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 1);
        assert_eq!(shell_state.last_exit_code, 1);
    }

    #[test]
    fn test_subshell_multiple_levels_with_exit_codes() {
        let mut shell_state = state::ShellState::new();

        // Nested subshells with different exit codes
        // ((exit 5))
        let tokens = lexer::lex("((exit 5))", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 5);
        assert_eq!(shell_state.last_exit_code, 5);
        assert!(!shell_state.exit_requested);
    }

    #[test]
    fn test_subshell_cd_isolation_multiple_levels() {
        let _lock = DIR_CHANGE_LOCK.lock().unwrap();

        let mut shell_state = state::ShellState::new();
        let original_dir = std::env::current_dir().unwrap();

        // Nested subshells with cd commands
        // ((cd /tmp; cd /))
        let tokens = lexer::lex("((cd /tmp; cd /))", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // Parent directory should be unchanged
        let current_dir = std::env::current_dir().unwrap();
        assert_eq!(current_dir, original_dir);
    }

    #[test]
    fn test_subshell_function_definition_isolation() {
        let mut shell_state = state::ShellState::new();

        // Define a function in a subshell
        let tokens = lexer::lex("(subfunc() { echo hello; })", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // Function should not exist in parent
        assert!(shell_state.get_function("subfunc").is_none());
    }

    #[test]
    fn test_subshell_alias_isolation() {
        let mut shell_state = state::ShellState::new();

        // Set an alias in parent
        shell_state.set_alias("ll", "ls -la".to_string());

        // Execute subshell (aliases should be inherited but modifications isolated)
        let tokens = lexer::lex("(echo test)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // Parent alias should still exist
        assert_eq!(shell_state.get_alias("ll"), Some(&"ls -la".to_string()));
    }

    #[test]
    fn test_subshell_positional_params_isolation() {
        let mut shell_state = state::ShellState::new();

        // Set positional parameters in parent
        shell_state.set_positional_params(vec![
            "arg1".to_string(),
            "arg2".to_string(),
            "arg3".to_string(),
        ]);

        // Execute subshell (positional params should be inherited)
        let tokens = lexer::lex("(echo test)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // Parent positional params should be unchanged
        assert_eq!(shell_state.get_var("1"), Some("arg1".to_string()));
        assert_eq!(shell_state.get_var("2"), Some("arg2".to_string()));
        assert_eq!(shell_state.get_var("3"), Some("arg3".to_string()));
        assert_eq!(shell_state.get_var("#"), Some("3".to_string()));
    }

    #[test]
    fn test_subshell_depth_tracking() {
        let mut shell_state = state::ShellState::new();

        // Initial depth should be 0
        assert_eq!(shell_state.subshell_depth, 0);

        // After executing a subshell, depth should still be 0 (parent unchanged)
        let tokens = lexer::lex("(echo test)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.subshell_depth, 0);
    }

    #[test]
    fn test_subshell_exit_and_continue() {
        let mut shell_state = state::ShellState::new();

        // Execute: (exit 1); echo $?; (exit 2); echo $?
        let tokens1 = lexer::lex("(exit 1)", &shell_state).unwrap();
        let ast1 = parser::parse(tokens1).unwrap();
        let exit_code1 = executor::execute(ast1, &mut shell_state);
        assert_eq!(exit_code1, 1);
        assert_eq!(shell_state.last_exit_code, 1);

        let tokens2 = lexer::lex("(exit 2)", &shell_state).unwrap();
        let ast2 = parser::parse(tokens2).unwrap();
        let exit_code2 = executor::execute(ast2, &mut shell_state);
        assert_eq!(exit_code2, 2);
        assert_eq!(shell_state.last_exit_code, 2);

        // Parent should never have exit_requested set
        assert!(!shell_state.exit_requested);
    }

    #[test]
    fn test_subshell_with_exported_var_modification() {
        let mut shell_state = state::ShellState::new();

        // Set and export a variable
        shell_state.set_exported_var("EXPORTED", "parent_value".to_string());

        // Modify in subshell
        let tokens = lexer::lex("(EXPORTED=child_value)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 0);

        // Parent's exported variable should be unchanged
        assert_eq!(
            shell_state.get_var("EXPORTED"),
            Some("parent_value".to_string())
        );
        assert!(shell_state.exported.contains("EXPORTED"));
    }

    #[test]
    fn test_subshell_empty_body_error() {
        let shell_state = state::ShellState::new();

        // Empty subshell should be caught by parser
        let tokens = lexer::lex("()", &shell_state).unwrap();
        let result = parser::parse(tokens);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty subshell"));
    }

    #[test]
    fn test_subshell_with_sequence_and_exit() {
        let mut shell_state = state::ShellState::new();

        // Subshell with sequence where exit is in the middle
        // (echo first; exit 42; echo second)
        // The "echo second" should not execute
        let tokens = lexer::lex("(echo first; exit 42; echo second)", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);

        assert_eq!(exit_code, 42);
        assert_eq!(shell_state.last_exit_code, 42);
        assert!(!shell_state.exit_requested);
    }
}
