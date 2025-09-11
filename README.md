# Rush Shell

Rush is a POSIX sh-compatible shell implemented in Rust. It provides both interactive mode with a REPL prompt and script mode for executing commands from files. The shell supports basic shell features like command execution, pipes, redirections, environment variables, and built-in commands.

## Features

- **Command Execution**: Execute external commands and built-in commands.
- **Pipes**: Chain commands using the `|` operator.
- **Redirections**: Input (`<`) and output (`>`, `>>`) redirections.
- **Environment Variables**: Support for `$VAR` and `${VAR}` expansion.
- **Built-in Commands**:
  - `cd`: Change directory
  - `exit`: Exit the shell
  - `echo`: Print text
  - `pwd`: Print working directory
  - `env`: List environment variables
  - `source`: Execute a script file with rush (bypasses shebang)
  - `help`: Show available commands
- **Tab Completion**: Intelligent completion for commands, files, and directories.
  - **Command Completion**: Built-in commands and executables from PATH
  - **File/Directory Completion**: Files and directories with relative paths
  - **Directory Traversal**: Support for nested paths (`src/`, `../`, `/usr/bin/`)
  - **Home Directory Expansion**: Completion for `~/` and `~/Documents/` paths
- **Signal Handling**: Graceful handling of SIGINT (Ctrl+C) and SIGTERM.
- **Line Editing and History**: Enhanced interactive experience with rustyline.

## Installation

### Prerequisites

- Rust (edition 2021 or later)

### Build

1. Clone the repository:
   ```bash
   git clone https://github.com/drewwalton19216801/rush.git
   cd rush
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

The binary will be available at `target/release/rush`.

## Usage

### Interactive Mode

Run the shell without arguments to enter interactive mode:

```bash
./target/release/rush
```

You'll see a prompt `$ ` where you can type commands. Type `exit` to quit.

### Script Mode

Execute commands from a file:

```bash
./target/release/rush script.sh
```

The shell will read and execute each line from the script file. Note that when using script mode, shebang lines (e.g., `#!/usr/bin/env bash`) are not bypassed - they are executed as regular comments.

### Command Mode

Execute a command string directly:

```bash
./target/release/rush -c "echo Hello World"
```

The shell will execute the provided command string and exit.

### Source Command

The `source` built-in command provides a way to execute script files while bypassing shebang lines that may specify other shells:

```bash
source script.sh
```

This is particularly useful for:
- Executing scripts written for rush that contain `#!/usr/bin/env rush` shebangs
- Running scripts with shebangs for other shells (like `#!/usr/bin/env bash`) using rush instead
- Ensuring consistent execution environment regardless of shebang declarations

Unlike script mode (running `./target/release/rush script.sh`), the `source` command automatically skips shebang lines and executes all commands using the rush interpreter.

### Examples

- Run a command: `ls -la`
- Use pipes: `ls | grep txt`
- Redirect output: `echo "Hello" > hello.txt`
- Change directory: `cd /tmp`
- Print working directory: `pwd`
- Execute a script: `source script.sh`
- Execute a script with shebang bypass: `source examples/basic_commands.sh`
- Tab completion:
  - Complete commands: `cd` → `cd `, `e` → `echo `, `env `, `exit `
  - Complete files: `cat f` → `cat file.txt `
  - Complete directories: `cd src/` → `cd src/main/`
  - Complete from PATH: `l` → `ls `, `g` → `grep `
  - Complete nested paths: `ls src/m` → `ls src/main/`

## Architecture

Rush consists of the following components:

- **Lexer**: Tokenizes input into commands, operators, and variables.
- **Parser**: Builds an Abstract Syntax Tree (AST) from tokens.
- **Executor**: Executes the AST, handling pipes, redirections, and built-ins.
- **Built-in Commands**: Optimized detection and execution of built-in commands using a centralized constant array for improved maintainability and performance.
- **Completion**: Provides intelligent tab-completion for commands, files, and directories.
- **Shell State**: Manages environment variables and current directory.

## Dependencies

- `rustyline`: For interactive line editing and history.
- `signal-hook`: For robust signal handling.
- `nix`: For Unix system interactions.
- `libc`: For low-level C library bindings.

## Testing

Rush includes a comprehensive test suite to ensure reliability and correctness. The tests cover unit testing for individual components, integration testing for end-to-end functionality, and error handling scenarios.

### Test Structure

- **Lexer Tests** Tokenization of commands, arguments, operators, quotes, variable expansion, and edge cases.
- **Parser Tests** AST construction for single commands, pipelines, redirections, and error cases.
- **Executor Tests** Built-in commands, external command execution, pipelines, redirections, and error handling.
- **Completion Tests** Tab-completion for commands, files, directories, path traversal, and edge cases.
- **Integration Tests** End-to-end command execution, including pipelines, redirections, and variable expansion.
- **Main Tests** Error handling for invalid directory changes.

### Running Tests

Run all tests with:
```bash
cargo test
```

Run specific test modules:
```bash
cargo test lexer
cargo test parser
cargo test executor
cargo test integration
```

### Test Coverage

The test suite provides extensive coverage of:
- Command parsing and execution
- Built-in command functionality (cd, echo, pwd, env, exit, help, source)
- Pipeline and redirection handling
- Variable expansion
- Tab-completion for commands, files, and directories
- Path traversal and directory completion
- Error conditions and edge cases
- Signal handling integration


## Contributing

Contributions are welcome! Please fork the repository, make your changes, and submit a pull request.

## License

This project is licensed under the MIT License. See [LICENSE.txt](LICENSE.txt) for details.

## Project URL

[https://github.com/drewwalton19216801/rush](https://github.com/drewwalton19216801/rush)