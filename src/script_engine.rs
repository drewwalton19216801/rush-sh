use crate::brace_expansion;
use crate::executor;
use crate::lexer;
use crate::parser;
use crate::state;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

/// Check if a line contains a heredoc redirection using proper lexer-based detection
/// Returns the delimiter if found, None otherwise
pub fn line_contains_heredoc(line: &str, shell_state: &state::ShellState) -> Option<String> {
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
pub fn contains_keyword(line: &str, keyword: &str) -> bool {
    let chars = line.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut current_word = String::new();

    for ch in chars {
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

/// Determine whether the first token of a line equals the given keyword, ignoring leading spaces and tabs.
///
/// Returns `true` if the first token is equal to `keyword`, `false` otherwise.
///
/// # Examples
///
/// ```
/// use rush_sh::script_engine::starts_with_keyword;
/// assert!(starts_with_keyword("  if condition", "if"));
/// assert!(!starts_with_keyword("echo if", "if"));
/// ```
pub fn starts_with_keyword(line: &str, keyword: &str) -> bool {
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

    for ch in chars {
        match ch {
            ' ' | '\t' | '\n' | ';' | '|' | '&' | '(' | ')' | '{' | '}' => {
                return current_word == keyword;
            }
            _ => current_word.push(ch),
        }
    }

    current_word == keyword
}

/// Process and execute a single shell command line.
///
/// This performs lexical analysis, alias expansion, brace expansion, parsing, and execution
/// in sequence; prints errors (using the configured color scheme when enabled) and updates
/// the shell state (including the last exit code and, on certain lex errors, exit request).
///
/// # Parameters
///
/// - `line`: the input command line to process.
/// - `shell_state`: mutable shell state used for options (e.g., verbose, colors), color output,
///   and to store execution results such as the last exit code and exit-request flag.
///
/// # Examples
///
/// ```ignore
/// // Example usage (requires a configured ShellState):
/// let mut shell_state = state::ShellState::new();
/// execute_line("echo hello", &mut shell_state);
/// assert_eq!(shell_state.last_exit_code(), 0);
/// ```
pub fn execute_line(line: &str, shell_state: &mut state::ShellState) {
    // Print input line if verbose option (-v) is enabled
    if shell_state.options.verbose {
        if shell_state.colors_enabled {
            eprintln!("{}{}\x1b[0m", shell_state.color_scheme.builtin, line);
        } else {
            eprintln!("{}", line);
        }
    }

    match lexer::lex(line, shell_state) {
        Ok(tokens) => match lexer::expand_aliases(tokens, shell_state, &mut HashSet::new()) {
            Ok(expanded_tokens) => match brace_expansion::expand_braces(expanded_tokens) {
                Ok(brace_expanded_tokens) => match parser::parse(brace_expanded_tokens) {
                    Ok(ast) => {
                        let exit_code = executor::execute(ast, shell_state);
                        shell_state.set_last_exit_code(exit_code);
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
                },
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
            },
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
        },
        Err(e) => {
            if shell_state.colors_enabled {
                eprintln!("{}Lex error: {}\x1b[0m", shell_state.color_scheme.error, e);
            } else {
                eprintln!("Lex error: {}", e);
            }
            shell_state.set_last_exit_code(1);

            // Check if this is a nounset error - if so, request shell exit
            if e.contains("unbound variable") {
                shell_state.exit_requested = true;
                shell_state.exit_code = 1;
            }
        }
    }
}

pub fn execute_script(
    content: &str,
    shell_state: &mut state::ShellState,
    shutdown_flag: Option<&AtomicBool>,
) {
    // Reset line number for script execution
    shell_state.current_line_number = 1;
    
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
    let mut in_until_block = false;
    let mut until_depth = 0;

    // Track quote state across lines to handle multiline strings correctly
    let mut in_double_quote = false;
    let mut in_single_quote = false;

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        
        // Update current line number for $LINENO - must be before any continue statements
        shell_state.current_line_number = i + 1;
        
        // Process pending signals at the start of each line
        state::process_pending_signals(shell_state);

        // Check for shutdown signal
        if let Some(flag) = shutdown_flag
            && flag.load(Ordering::Relaxed)
        {
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
        let chars = line.chars().peekable();
        let mut escaped = false;

        for ch in chars {
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
                '#' => break, // Comment starts
                '\'' => in_single_quote = true,
                '"' => in_double_quote = true,
                '\\' => escaped = true,
                _ => {}
            }
        }

        let trimmed = line.trim();
        if !in_double_quote && !in_single_quote && (trimmed.is_empty() || trimmed.starts_with("#"))
        {
            i += 1;
            continue;
        }

        let keywords_active = !in_double_quote && !in_single_quote;

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
            } else if starts_with_keyword(line, "until") {
                in_until_block = true;
                until_depth += 1;
            } else {
                let is_group_start = {
                    let trimmed = line.trim();
                    trimmed == "{" || trimmed.starts_with("{ ") || trimmed.starts_with("{\t")
                };
                if is_group_start {
                    in_group_block = true;
                    brace_depth += line.matches('{').count() as i32;
                    brace_depth -= line.matches('}').count() as i32;
                }
            }
        }

        if keywords_active
            && (line.contains("() {") || (trimmed.ends_with("()") && !in_function_block))
        {
            in_function_block = true;
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;
        } else if in_function_block || in_group_block {
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;
        }

        if !current_block.is_empty() {
            current_block.push('\n');
        }
        current_block.push_str(line);

        if keywords_active {
            if (in_function_block || in_group_block) && brace_depth == 0 {
                in_function_block = false;
                in_group_block = false;
                execute_line(&current_block, shell_state);
                current_block.clear();

                if shell_state.exit_requested {
                    break;
                }
            } else if in_if_block && contains_keyword(line, "fi") {
                if_depth -= 1;
                if if_depth == 0 {
                    in_if_block = false;
                    // Only execute if we're not inside a loop or other block
                    if !in_for_block
                        && !in_while_block
                        && !in_until_block
                        && !in_function_block
                        && !in_group_block
                        && !in_case_block
                    {
                        execute_line(&current_block, shell_state);
                        current_block.clear();

                        if shell_state.exit_requested {
                            break;
                        }
                    }
                }
            } else if in_for_block && contains_keyword(line, "done") {
                for_depth -= 1;
                if for_depth == 0 {
                    in_for_block = false;
                    execute_line(&current_block, shell_state);
                    current_block.clear();

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

                    if shell_state.exit_requested {
                        break;
                    }
                }
            } else if in_until_block && contains_keyword(line, "done") {
                until_depth -= 1;
                if until_depth == 0 {
                    in_until_block = false;
                    execute_line(&current_block, shell_state);
                    current_block.clear();

                    if shell_state.exit_requested {
                        break;
                    }
                }
            } else if in_case_block && contains_keyword(line, "esac") {
                in_case_block = false;
                execute_line(&current_block, shell_state);
                current_block.clear();

                if shell_state.exit_requested {
                    break;
                }
            } else if !in_if_block
                && !in_case_block
                && !in_function_block
                && !in_group_block
                && !in_for_block
                && !in_while_block
                && !in_until_block
            {
                if let Some(delimiter) = line_contains_heredoc(&current_block, shell_state) {
                    i += 1;
                    let mut heredoc_content = String::new();
                    while i < lines.len() {
                        let content_line = lines[i];
                        if content_line.trim() == delimiter.trim() {
                            break;
                        }
                        if !heredoc_content.is_empty() {
                            heredoc_content.push('\n');
                        }
                        heredoc_content.push_str(content_line);
                        i += 1;
                    }
                    shell_state.pending_heredoc_content = Some(heredoc_content);
                    execute_line(&current_block, shell_state);
                    current_block.clear();
                } else if !in_single_quote
                    && !in_double_quote
                    && (line.ends_with(';') || !line.trim_end().ends_with('\\'))
                {
                    execute_line(&current_block, shell_state);
                    current_block.clear();
                }
            }
        }
        i += 1;
    }
}
