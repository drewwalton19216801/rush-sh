# Rush - A Unix shell written in Rust

<img src="images/rush_logo.png" alt="Rush Logo" width="50%" style="display: block; margin: 20px auto;">

Rush is a POSIX sh-compatible shell implemented in Rust. It provides both interactive mode with a REPL prompt and script mode for executing commands from files. The shell supports basic shell features like command execution, pipes, redirections, environment variables, and built-in commands.

## Pun-der the Hood

- In a hurry? Don’t bash your head against it—Rush it.
- When your pipelines need a drum solo, put them on Rush and let the commands Neil-Peart through.
- Tom Sawyer tip: chores go faster when you make them look like a Rush job; no need to paint the fence by hand when the shell can whitewash it with a one-liner.
- Alias your productivity: `alias hurry='rush -c "do the thing"'`—because sometimes you just need to rush to judgment.
- This shell doesn’t just run fast; it gives you the Rush of a clean exit status.

## Features

- **Command Execution**: Execute external commands and built-in commands.
- **Pipes**: Chain commands using the `|` operator.
- **Redirections**: Input (`<`) and output (`>`, `>>`) redirections.
- **Command Substitution**: Execute commands and substitute their output inline.
  - `$(command)` syntax: `echo "Current dir: $(pwd)"`
  - `` `command` `` syntax: `echo "Files: `ls | wc -l`"`
  - Variable expansion within substitutions: `echo $(echo $HOME)`
  - Error handling with fallback to literal syntax
- **Environment Variables**: Full support for variable assignment, expansion, and export.
  - Variable assignment: `VAR=value` and `VAR="quoted value"`
  - Variable expansion: `$VAR` and special variables (`$?`, `$$`, `$0`)
  - Export mechanism: `export VAR` and `export VAR=value`
  - Variable scoping: Shell variables vs exported environment variables
- **Control Structures**:
  - `if` statements: `if condition; then commands; elif condition; then commands; else commands; fi`
  - `case` statements with glob pattern matching: `case word in pattern1|pattern2) commands ;; *.txt) commands ;; *) default ;; esac`
- **Built-in Commands**:
  - `cd`: Change directory
  - `exit`: Exit the shell
  - `echo`: Print text
  - `pwd`: Print working directory
  - `env`: List environment variables
  - `export`: Export variables to child processes
  - `unset`: Remove variables
  - `source`: Execute a script file with rush (bypasses shebang)
  - `pushd`: Push directory onto stack and change to it
  - `popd`: Pop directory from stack and change to it
  - `dirs`: Display directory stack
  - `alias`: Define or display aliases
  - `unalias`: Remove alias definitions
  - `help`: Show available commands
- **Tab Completion**: Intelligent completion for commands, files, and directories.
  - **Command Completion**: Built-in commands and executables from PATH
  - **File/Directory Completion**: Files and directories with relative paths
  - **Directory Traversal**: Support for nested paths (`src/`, `../`, `/usr/bin/`)
  - **Home Directory Expansion**: Completion for `~/` and `~/Documents/` paths
- **Signal Handling**: Graceful handling of SIGINT (Ctrl+C) and SIGTERM.
- **Line Editing and History**: Enhanced interactive experience with rustyline.

## Latest Updates

### Environment Variable Support

Rush now provides comprehensive environment variable support with full POSIX compliance:

- **Variable Assignment**: Support for both simple and quoted assignments

  ```bash
  MY_VAR=hello
  MY_VAR="hello world"
  NAME="Alice"
  ```

- **Variable Expansion**: Expand variables in commands with `$VAR` syntax

  ```bash
  echo "Hello $NAME"
  echo "Current directory: $PWD"
  ```

- **Special Variables**: Built-in support for special shell variables

  ```bash
  echo "Last exit code: $?"
  echo "Shell PID: $$"
  echo "Script name: $0"
  ```

- **Export Mechanism**: Export variables to child processes

  ```bash
  export MY_VAR
  export NEW_VAR=value
  ```

- **Variable Management**: Full lifecycle management with `unset`

  ```bash
  unset MY_VAR
  ```

- **Multi-Mode Support**: Variables work consistently across all execution modes
  - Interactive mode: Variables persist across commands
  - Script mode: Variables available throughout script execution
  - Command string mode: Variables work in `-c` command strings

Example usage:

```bash
# Set and use variables
MY_VAR="Hello from Rush"
echo "Message: $MY_VAR"

# Export to child processes
export MY_VAR
env | grep MY_VAR

# Use in pipelines
echo "$MY_VAR" | grep "Rush"

# Special variables
if true; then echo "Success ($?)"; fi
```

### Case Statements with Glob Pattern Matching

Rush now supports advanced case statements with full glob pattern matching capabilities:

- **Glob Patterns**: Use wildcards like `*` (any characters), `?` (single character), and `[abc]` (character classes)
- **Multiple Patterns**: Combine patterns with `|` (e.g., `*.txt|*.md`)
- **POSIX Compliance**: Full support for standard case statement syntax
- **Performance**: Efficient pattern matching using the `glob` crate

Example usage:

```bash
case $filename in
    *.txt|*.md) echo "Text file" ;;
    *.jpg|*.png) echo "Image file" ;;
    file?) echo "Single character file" ;;
    [abc]*) echo "Starts with a, b, or c" ;;
    *) echo "Other file type" ;;
esac
```

### Directory Stack Support (pushd/popd/dirs)

Rush now supports directory stack management with the classic Unix `pushd`, `popd`, and `dirs` commands:

- **`pushd <directory>`**: Changes to the specified directory and pushes the current directory onto the stack
- **`popd`**: Pops the top directory from the stack and changes to it
- **`dirs`**: Displays the current directory stack

Example usage:

```bash
# Start in home directory
pwd
# /home/user

# Push to /tmp and see stack
pushd /tmp
# /tmp /home/user

# Push to another directory
pushd /var
# /var /tmp /home/user

# See current stack
dirs
# /var /tmp /home/user

# Pop back to /tmp
popd
# /tmp /home/user

# Pop back to home
popd
# /home/user
```

This feature is particularly useful for:

- Quickly switching between multiple working directories
- Maintaining context when working on different parts of a project
- Scripting scenarios that require directory navigation

### Command Substitution

Rush now supports comprehensive command substitution with both `$(...)` and `` `...` `` syntax:

- **Dual Syntax Support**: Both `$(command)` and `` `command` `` work identically
- **Immediate Execution**: Commands are executed during lexing and output is substituted inline
- **Variable Expansion**: Variables within substituted commands are properly expanded
- **Error Handling**: Failed commands fall back to literal syntax preservation
- **Environment Integration**: Child processes inherit shell environment variables
- **Multi-line Support**: Handles commands with multiple lines and special characters

Example usage:

### Condensed Current Working Directory in Prompt

Rush now displays a condensed version of the current working directory in the interactive prompt:

- **Condensed Path**: Each directory except the last is abbreviated to its first letter (preserving case)
- **Full Last Directory**: The final directory in the path is shown in full
- **Dynamic Updates**: The prompt updates automatically when changing directories

Example prompt displays:

```bash
/h/d/p/r/rush $
/u/b/s/project $
/h/u/Documents $
```

This feature provides context about your current location while keeping the prompt concise.

```bash
# Basic command substitution
echo "Current directory: $(pwd)"
echo "Files in directory: `ls | wc -l`"

# Variable assignments with substitutions
PROJECT_DIR="$(pwd)/src"
FILE_COUNT="$(ls *.rs 2>/dev/null | wc -l)"

# Complex expressions
echo "Rust version: $(rustc --version | cut -d' ' -f2)"
echo "Files modified today: $(find . -name '*.rs' -mtime -1 | wc -l)"

# Error handling
NONEXISTENT="$(nonexistent_command 2>/dev/null || echo 'command failed')"
echo "Result: $NONEXISTENT"

# Multiple commands
echo "Combined output: $(echo 'Hello'; echo 'World')"
```

Command substitution works seamlessly with:

- **Pipes and Redirections**: `$(echo hello | grep ll) > output.txt`
- **Variable Expansion**: `echo $(echo $HOME)`
- **Quoted Strings**: `echo "Path: $(pwd)"`
- **Complex Commands**: `$(find . -name "*.rs" -exec wc -l {} \;)`

### Built-in Alias Support

Rush now provides comprehensive built-in alias support, allowing you to create shortcuts for frequently used commands:

- **Create Aliases**: Define shortcuts with `alias name=value` syntax
- **List Aliases**: View all defined aliases with `alias` command
- **Show Specific Alias**: Display a single alias with `alias name`
- **Remove Aliases**: Delete aliases with `unalias name`
- **Automatic Expansion**: Aliases are expanded automatically during command execution
- **Recursion Prevention**: Built-in protection against infinite alias loops

Example usage:

```bash
# Create aliases
alias ll='ls -l'
alias la='ls -la'
alias ..='cd ..'
alias grep='grep --color=auto'

# List all aliases
alias
# Output:
# alias ll='ls -l'
# alias la='ls -la'
# alias ..='cd ..'
# alias grep='grep --color=auto'

# Show specific alias
alias ll
# Output: alias ll='ls -l'

# Use aliases (they expand automatically)
ll
la /tmp
..

# Remove aliases
unalias ll
alias  # ll is no longer listed

# Error handling
unalias nonexistent  # Shows: unalias: nonexistent: not found
```

**Key Features:**

- **POSIX Compliance**: Follows standard alias syntax and behavior
- **Session Persistence**: Aliases persist throughout the shell session
- **Complex Commands**: Support for multi-word commands and pipelines
- **Variable Expansion**: Variables in aliases are expanded when defined
- **Safety**: Automatic detection and prevention of recursive aliases

**Advanced Usage:**

```bash
# Complex aliases with pipes and redirections
alias backup='cp -r ~/Documents ~/Documents.backup && echo "Backup completed"'
alias count='find . -name "*.rs" | wc -l'

# Aliases with variables (expanded at definition time)
MY_DIR="/tmp"
alias cleanup="rm -rf $MY_DIR/*"

# Function-like aliases
alias mkcd='mkdir -p "$1" && cd "$1"'  # Note: $1 won't work as expected
```

**Implementation Details:**

- Aliases are expanded after lexing but before parsing
- Only the first word of a command can be an alias
- Expansion is recursive (aliases can reference other aliases)
- Built-in protection against infinite recursion
- Aliases work in all execution modes (interactive, script, command)

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

You'll see a prompt showing the condensed current working directory followed by `$ ` (e.g., `/h/d/p/r/rush $ `) where you can type commands. Type `exit` to quit.

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
- Directory stack management:
  - Push directory: `pushd /tmp`
  - Pop directory: `popd`
  - Show stack: `dirs`
- Execute a script: `source script.sh`
- Execute a script with shebang bypass: `source examples/basic_commands.sh`
- Execute elif example script: `source examples/elif_example.sh`
- Execute case example script: `source examples/case_example.sh`
- Execute variables example script: `source examples/variables_example.sh`
- Execute complex example script with command substitution: `source examples/complex_example.sh`
- Alias management:
  - Create aliases: `alias ll='ls -l'; alias la='ls -la'`
  - List aliases: `alias`
  - Show specific alias: `alias ll`
  - Remove aliases: `unalias ll`
  - Use aliases: `ll /tmp`
- Environment variables:
  - Set variables: `MY_VAR=hello; echo $MY_VAR`
  - Export variables: `export MY_VAR=value; env | grep MY_VAR`
  - Special variables: `echo "Exit code: $?"; echo "PID: $$"`
  - Quoted values: `NAME="John Doe"; echo "Hello $NAME"`
- Use control structures:
  - If statement: `if true; then echo yes; else echo no; fi`
  - If-elif-else statement: `if false; then echo no; elif true; then echo yes; else echo maybe; fi`
  - Case statement with glob patterns:
    - Simple match: `case hello in hello) echo match ;; *) echo no match ;; esac`
    - Glob patterns: `case file.txt in *.txt) echo "Text file" ;; *.jpg) echo "Image" ;; *) echo "Other" ;; esac`
    - Multiple patterns: `case file in *.txt|*.md) echo "Document" ;; *.exe|*.bin) echo "Executable" ;; *) echo "Other" ;; esac`
    - Character classes: `case letter in [abc]) echo "A, B, or C" ;; *) echo "Other letter" ;; esac`
- Command substitution:
  - Basic substitution: `echo "Current dir: $(pwd)"`
  - Backtick syntax: `echo "Files: `ls | wc -l`"`
  - Variable assignments: `PROJECT_DIR="$(pwd)/src"`
  - Complex commands: `echo "Rust version: $(rustc --version | cut -d' ' -f2)"`
  - Error handling: `RESULT="$(nonexistent_command 2>/dev/null || echo 'failed')"`
  - With pipes: `$(echo hello | grep ll) > output.txt`
  - Multiple commands: `echo "Output: $(echo 'First'; echo 'Second')"`
- Tab completion:
  - Complete commands: `cd` → `cd `, `e` → `echo `, `env `, `exit `
  - Complete files: `cat f` → `cat file.txt `
  - Complete directories: `cd src/` → `cd src/main/`
  - Complete from PATH: `l` → `ls `, `g` → `grep `
  - Complete nested paths: `ls src/m` → `ls src/main/`

## Architecture

Rush consists of the following components:

- **Lexer**: Tokenizes input into commands, operators, and variables with support for variable expansion, command substitution (`$(...)` and `` `...` `` syntax), and alias expansion.
- **Parser**: Builds an Abstract Syntax Tree (AST) from tokens, including support for complex control structures, case statements with glob patterns, and variable assignments.
- **Executor**: Executes the AST, handling pipes, redirections, built-ins, glob pattern matching, environment variable inheritance, and command substitution execution.
- **Shell State**: Comprehensive state management for environment variables, exported variables, special variables (`$?`, `$$`, `$0`), current directory, directory stack, and command aliases.
- **Built-in Commands**: Optimized detection and execution of built-in commands including variable management (`export`, `unset`, `env`) and alias management (`alias`, `unalias`).
- **Completion**: Provides intelligent tab-completion for commands, files, and directories.

## Dependencies

- `rustyline`: For interactive line editing and history.
- `signal-hook`: For robust signal handling.
- `nix`: For Unix system interactions.
- `libc`: For low-level C library bindings.
- `glob`: For pattern matching in case statements.

## Testing

Rush includes a comprehensive test suite to ensure reliability and correctness. The tests cover unit testing for individual components, integration testing for end-to-end functionality, and error handling scenarios.

### Test Structure

- **Lexer Tests** Tokenization of commands, arguments, operators, quotes, variable expansion, command substitution, and edge cases.
- **Parser Tests** AST construction for single commands, pipelines, redirections, if-elif-else statements, case statements with glob patterns, and error cases.
- **Executor Tests** Built-in commands, external command execution, pipelines, redirections, case statement execution with glob matching, command substitution execution, and error handling.
- **Completion Tests** Tab-completion for commands, files, directories, path traversal, and edge cases.
- **Integration Tests** End-to-end command execution, including pipelines, redirections, variable expansion, case statements, and command substitution.
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
- Built-in command functionality (cd, echo, pwd, env, exit, help, source, export, unset, pushd, popd, dirs, alias, unalias)
- Pipeline and redirection handling
- Control structures (if-elif-else statements, case statements with glob patterns)
- Command substitution (`$(...)` and `` `...` `` syntax, error handling, variable expansion)
- Environment variable support (assignment, expansion, export, special variables)
- Variable scoping and inheritance
- Tab-completion for commands, files, and directories
- Path traversal and directory completion
- Error conditions and edge cases
- Signal handling integration

## Contributing

Contributions are welcome! Please fork the repository, make your changes, and submit a pull request.

## License

This project is licensed under the MIT License. See [LICENSE.txt](LICENSE.txt) for details.

## Project URL

[https://github.com/drewwalton19216801/rush-sh](https://github.com/drewwalton19216801/rush-sh)
