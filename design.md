# Rush Shell Design

## Overview
Rush is a POSIX sh-compatible shell implemented in Rust. It supports interactive mode (REPL with prompt) and script mode (executing commands from a file). The goal is to mimic basic sh behavior: command execution, pipes, redirections, environment variables, and built-ins.

## High-Level Architecture

### Components
1. **Lexer**: Tokenizes input lines into tokens. Handles:
   - Words (commands, args, with variable expansion $VAR)
   - Operators: | (pipe), > (output redirect), < (input redirect), >> (append redirect)
   - Quotes: "double" for expansion, 'single' for literal
   - Escapes: \ for special chars
   - Variables: $VAR, ${VAR}

2. **Parser**: Builds an Abstract Syntax Tree (AST) from tokens. Simple grammar:
   - Pipeline: Command | Command | ...
   - Command: words + redirections (input/output files)
   - No complex control flow initially (add if/while later if needed)

3. **Executor**: Runs the AST:
   - For each command in pipeline: If built-in, execute internally; else spawn std::process::Command
   - Pipes: Connect stdout of one to stdin of next using child processes and pipes (std::process::Stdio)
   - Redirections: Open files with std::fs, set stdin/stdout accordingly
   - Environment: Inherit from shell state (HashMap<String, String>), pass to commands via env vars

4. **Shell State**:
   - Environment variables: std::collections::HashMap<String, String>
   - Current working directory: managed via built-in cd
   - Built-ins: cd (change dir), exit (quit), echo (print), pwd (print dir), env (list vars)

5. **Main Loop**:
    - Parse args: If first arg is file path, read file lines, execute each
    - Else, interactive: Loop reading lines using rustyline Editor for enhanced line editing and history, print prompt "$ ", lex/parse/exec
    - Handle EOF (Ctrl+D) to exit interactive

6. **Signal Handling**:
    - Uses signal-hook crate for robust signal processing in a separate thread
    - SIGINT (Ctrl+C): Interrupts current input line, prints "^C", continues shell execution
    - SIGTERM: Triggers graceful shutdown with "Received SIGTERM, exiting gracefully."
    - Thread-safe communication via AtomicBool flag for shutdown coordination
    - Integrates with rustyline's signal-hook feature for seamless interruption handling

### Dependencies
- Rely on std::process for command execution, std::env for initial env
- rustyline crate (v12.0) with signal-hook feature for interactive line editing and command history
- signal-hook crate (v0.3) for robust signal handling that works with rustyline
- libc crate for low-level system interactions
- nix crate (available for future advanced Unix I/O if needed)

### Error Handling
- Print errors to stderr, continue in interactive mode
- Signal interruptions: Handle gracefully without exiting shell
- Readline errors: Detect signal-related interruptions and continue execution
- Exit codes: Propagate from last command in pipeline

### POSIX Compliance Notes
- Word splitting: After expansion, split on whitespace
- Globbing: Basic * ? support if time allows (use glob crate)
- Signals: Handle SIGINT (Ctrl+C) to interrupt current input line, SIGTERM for graceful shutdown
- Start simple, iterate to add features

## Implementation Order
Follow the todo list: Start with basic main structure, then lexer/parser, etc.