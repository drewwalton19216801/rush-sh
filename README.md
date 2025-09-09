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

The shell will read and execute each line from the script file.

### Command Mode

Execute a command string directly:

```bash
./target/release/rush -c "echo Hello World"
```

The shell will execute the provided command string and exit.

### Examples

- Run a command: `ls -la`
- Use pipes: `ls | grep txt`
- Redirect output: `echo "Hello" > hello.txt`
- Change directory: `cd /tmp`
- Print working directory: `pwd`

## Architecture

Rush consists of the following components:

- **Lexer**: Tokenizes input into commands, operators, and variables.
- **Parser**: Builds an Abstract Syntax Tree (AST) from tokens.
- **Executor**: Executes the AST, handling pipes, redirections, and built-ins.
- **Shell State**: Manages environment variables and current directory.

## Dependencies

- `rustyline`: For interactive line editing and history.
- `signal-hook`: For robust signal handling.
- `nix`: For Unix system interactions.
- `libc`: For low-level C library bindings.

## Testing

Rush includes a comprehensive test suite to ensure reliability and correctness. The tests cover unit testing for individual components, integration testing for end-to-end functionality, and error handling scenarios.

### Test Structure

- **Lexer Tests** (16 tests): Tokenization of commands, arguments, operators, quotes, variable expansion, and edge cases.
- **Parser Tests** (11 tests): AST construction for single commands, pipelines, redirections, and error cases.
- **Executor Tests** (15 tests): Built-in commands, external command execution, pipelines, redirections, and error handling.
- **Integration Tests** (8 tests): End-to-end command execution, including pipelines, redirections, and variable expansion.
- **Main Tests** (1 test): Error handling for invalid directory changes.

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
- Built-in command functionality (cd, echo, pwd, env, exit, help)
- Pipeline and redirection handling
- Variable expansion
- Error conditions and edge cases
- Signal handling integration

Total: 51 tests, all passing.

## Contributing

Contributions are welcome! Please fork the repository, make your changes, and submit a pull request.

## License

This project is licensed under the MIT License. See [LICENSE.txt](LICENSE.txt) for details.

## Project URL

[https://github.com/drewwalton19216801/rush](https://github.com/drewwalton19216801/rush)