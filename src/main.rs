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
