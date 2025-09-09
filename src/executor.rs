use std::env;
use std::process::{Command, Stdio};
use std::path::Path;
use std::fs::File;
use std::io::{self, Write};

use super::parser::{Ast, ShellCommand};

pub fn is_builtin(cmd: &str) -> bool {
    matches!(cmd, "cd" | "echo" | "pwd" | "env" | "exit" | "help")
}

pub fn execute_builtin(cmd: &ShellCommand) -> i32 {
    // Handle input redirection for built-ins that might need it
    let _input_content = if let Some(ref input_file) = cmd.input {
        match std::fs::read_to_string(input_file) {
            Ok(content) => Some(content),
            Err(e) => {
                eprintln!("Error reading input file '{}': {}", input_file, e);
                return 1;
            }
        }
    } else {
        None
    };

    // Prepare output destination
    let mut output_writer: Box<dyn Write> = if let Some(ref output_file) = cmd.output {
        match File::create(output_file) {
            Ok(file) => Box::new(file),
            Err(e) => {
                eprintln!("Error creating output file '{}': {}", output_file, e);
                return 1;
            }
        }
    } else if let Some(ref append_file) = cmd.append {
        match File::options().append(true).create(true).open(append_file) {
            Ok(file) => Box::new(file),
            Err(e) => {
                eprintln!("Error opening append file '{}': {}", append_file, e);
                return 1;
            }
        }
    } else {
        Box::new(io::stdout())
    };

    match cmd.args[0].as_str() {
        "cd" => {
            // cd doesn't produce output, so redirections don't make sense for it
            // But we still handle it for consistency
            let dir = if cmd.args.len() > 1 {
                cmd.args[1].clone()
            } else {
                "~".to_string()
            };
            let path = if dir == "~" {
                env::var("HOME").unwrap_or_else(|_| "/".to_string())
            } else {
                dir
            };
            if let Err(e) = env::set_current_dir(Path::new(&path)) {
                let _ = writeln!(output_writer, "cd: {}: {}", path, e);
                1
            } else {
                0
            }
        }
        "echo" => {
            let output = cmd.args[1..].join(" ");
            let _ = writeln!(output_writer, "{}", output);
            0
        }
        "pwd" => {
            match env::current_dir() {
                Ok(path) => {
                    let _ = writeln!(output_writer, "{}", path.display());
                    0
                }
                Err(e) => {
                    let _ = writeln!(output_writer, "pwd: {}", e);
                    1
                }
            }
        }
        "env" => {
            for (key, value) in env::vars() {
                let _ = writeln!(output_writer, "{}={}", key, value);
            }
            0
        }
        "exit" => {
            // For now, just return 0; main handles exit
            0
        }
        "help" => {
            let _ = writeln!(output_writer, "Rush Shell v{}", env!("CARGO_PKG_VERSION"));
            let _ = writeln!(output_writer, "");
            let _ = writeln!(output_writer, "Available built-in commands:");
            let builtins = [
                ("cd", "Change directory"),
                ("echo", "Print arguments"),
                ("pwd", "Print working directory"),
                ("env", "Print environment variables"),
                ("exit", "Exit the shell"),
                ("help", "Show this help message")
            ];
            for (cmd, desc) in &builtins {
                let _ = writeln!(output_writer, "  {:<8} {}", cmd, desc);
            }
            0
        }
        _ => 1,
    }
}

pub fn execute(ast: Ast) -> i32 {
    let Ast::Pipeline(commands) = ast;

    if commands.is_empty() {
        return 0;
    }

    if commands.len() == 1 {
        // Single command, handle redirections
        execute_single_command(&commands[0])
    } else {
        // Pipeline
        execute_pipeline(&commands)
    }
}

fn execute_single_command(cmd: &ShellCommand) -> i32 {
    if cmd.args.is_empty() {
        return 0;
    }

    if is_builtin(&cmd.args[0]) {
        execute_builtin(cmd)
    } else {
        let mut command = Command::new(&cmd.args[0]);
        command.args(&cmd.args[1..]);

        // Handle input redirection
        if let Some(ref input_file) = cmd.input {
            match File::open(input_file) {
                Ok(file) => {
                    command.stdin(Stdio::from(file));
                }
                Err(e) => {
                    eprintln!("Error opening input file '{}': {}", input_file, e);
                    return 1;
                }
            }
        }

        // Handle output redirection
        if let Some(ref output_file) = cmd.output {
            match File::create(output_file) {
                Ok(file) => {
                    command.stdout(Stdio::from(file));
                }
                Err(e) => {
                    eprintln!("Error creating output file '{}': {}", output_file, e);
                    return 1;
                }
            }
        } else if let Some(ref append_file) = cmd.append {
            match File::options().append(true).create(true).open(append_file) {
                Ok(file) => {
                    command.stdout(Stdio::from(file));
                }
                Err(e) => {
                    eprintln!("Error opening append file '{}': {}", append_file, e);
                    return 1;
                }
            }
        }

        match command.status() {
            Ok(status) => status.code().unwrap_or(0),
            Err(e) => {
                eprintln!("Command execution error: {}", e);
                1
            }
        }
    }
}

fn execute_pipeline(commands: &[ShellCommand]) -> i32 {
    let mut exit_code = 0;
    let mut previous_stdout = None;

    for (i, cmd) in commands.iter().enumerate() {
        if cmd.args.is_empty() {
            continue;
        }

        let is_last = i == commands.len() - 1;

        if is_builtin(&cmd.args[0]) {
            // Built-ins in pipelines are tricky - for now, execute them separately
            // This is not perfect but better than nothing
            if let Some(_prev) = previous_stdout {
                // We can't easily pipe to built-ins, so just execute
                eprintln!("Warning: Built-in in pipeline may not work as expected");
            }
            exit_code = execute_builtin(cmd);
            previous_stdout = None;
        } else {
            let mut command = Command::new(&cmd.args[0]);
            command.args(&cmd.args[1..]);

            // Set stdin from previous command's stdout
            if let Some(prev) = previous_stdout.take() {
                command.stdin(prev);
            }

            // Set stdout for next command, unless this is the last
            if !is_last {
                command.stdout(Stdio::piped());
            }

            // Handle input redirection (only for first command)
            if i == 0 {
                if let Some(ref input_file) = cmd.input {
                    match File::open(input_file) {
                        Ok(file) => {
                            command.stdin(Stdio::from(file));
                        }
                        Err(e) => {
                            eprintln!("Error opening input file '{}': {}", input_file, e);
                            return 1;
                        }
                    }
                }
            }

            // Handle output redirection (only for last command)
            if is_last {
                if let Some(ref output_file) = cmd.output {
                    match File::create(output_file) {
                        Ok(file) => {
                            command.stdout(Stdio::from(file));
                        }
                        Err(e) => {
                            eprintln!("Error creating output file '{}': {}", output_file, e);
                            return 1;
                        }
                    }
                } else if let Some(ref append_file) = cmd.append {
                    match File::options().append(true).create(true).open(append_file) {
                        Ok(file) => {
                            command.stdout(Stdio::from(file));
                        }
                        Err(e) => {
                            eprintln!("Error opening append file '{}': {}", append_file, e);
                            return 1;
                        }
                    }
                }
            }

            match command.spawn() {
                Ok(mut child) => {
                    if !is_last {
                        previous_stdout = child.stdout.take().map(Stdio::from);
                    }
                    match child.wait() {
                        Ok(status) => {
                            exit_code = status.code().unwrap_or(0);
                        }
                        Err(e) => {
                            eprintln!("Error waiting for command: {}", e);
                            exit_code = 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error spawning command '{}': {}", cmd.args[0], e);
                    exit_code = 1;
                }
            }
        }
    }

    exit_code
}