# Rush - A Unix shell written in Rust

[![](https://tokei.rs/b1/github/drewwalton19216801/rush-sh)](https://github.com/drewwalton19216801/rush-sh) [![dependency status](https://deps.rs/repo/github/drewwalton19216801/rush-sh/status.svg)](https://deps.rs/repo/github/drewwalton19216801/rush-sh)

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
  - `pwd`: Print working directory
  - `env`: List environment variables
  - `export`: Export variables to child processes
  - `unset`: Remove variables
  - `source` / `.`: Execute a script file with rush (bypasses shebang and comment lines)
  - `pushd`: Push directory onto stack and change to it
  - `popd`: Pop directory from stack and change to it
  - `dirs`: Display directory stack
  - `alias`: Define or display aliases
  - `unalias`: Remove alias definitions
  - `test` / `[`: POSIX-compatible test builtin with string and file tests
  - `set_colors`: Enable/disable color output dynamically
  - `set_color_scheme`: Switch between color themes (default/dark/light)
  - `help`: Show available commands
- **Configuration File**: Automatic sourcing of `~/.rushrc` on interactive shell startup
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

### Test Builtin with Conditional Logic

Rush now provides comprehensive support for the POSIX `test` builtin command and its `[` bracket syntax, enabling powerful conditional logic in shell scripts:

- **String Tests**: Check if strings are empty (`-z`) or non-empty (`-n`)
- **File Tests**: Test file existence (`-e`), regular files (`-f`), and directories (`-d`)
- **Dual Syntax Support**: Both `test` and `[` syntax work identically
- **POSIX Compliance**: Full compatibility with standard test command behavior
- **Error Handling**: Proper exit codes (0=true, 1=false, 2=error)
- **Integration**: Seamless integration with shell control structures

Example usage:

```bash
# String tests
if test -z ""; then echo "Empty string"; fi
if [ -n "hello" ]; then echo "Non-empty string"; fi

# File tests
if test -e "/tmp/file.txt"; then echo "File exists"; fi
if [ -d "/tmp" ]; then echo "Is directory"; fi
if test -f "/etc/passwd"; then echo "Is regular file"; fi

# Complex conditions
if [ -n "$MY_VAR" ] && test -e "$CONFIG_FILE"; then
    echo "Variable set and config file exists"
fi

# Error handling
test -x "invalid_option"  # Returns exit code 2
exit_code=$?
if [ $exit_code -eq 2 ]; then echo "Invalid option used"; fi
```

**Key Features:**

- **String Operations**: `-z` (zero length) and `-n` (non-zero length) tests
- **File Operations**: `-e` (exists), `-f` (regular file), `-d` (directory)
- **Bracket Syntax**: `[ condition ]` works identically to `test condition`
- **Exit Codes**: 0 (true), 1 (false), 2 (error/invalid usage)
- **Variable Expansion**: Variables are properly expanded in test conditions
- **Nested Conditions**: Works with complex if/elif/else structures

**Advanced Usage:**

```bash
# Variable existence checks
MY_VAR="hello world"
if test -n "$MY_VAR"; then
    echo "MY_VAR is set to: $MY_VAR"
fi

# Safe file operations
TARGET_FILE="/tmp/safe_file.txt"
if test -e "$TARGET_FILE"; then
    echo "File exists, backing up..."
    mv "$TARGET_FILE" "$TARGET_FILE.backup"
fi

# Directory creation with checks
TARGET_DIR="/tmp/test_dir"
if test -d "$TARGET_DIR"; then
    echo "Directory already exists"
else
    mkdir -p "$TARGET_DIR"
    echo "Directory created"
fi
```

The test builtin is fully integrated with Rush's control structures, enabling complex conditional logic in scripts while maintaining POSIX compatibility.

### Color Support

Rush now provides comprehensive color support for enhanced terminal output with automatic detection and flexible configuration:

- **Automatic Terminal Detection**: Colors are enabled in interactive terminals and disabled for pipes/files
- **Environment Variable Control**: Support for `NO_COLOR=1` (accessibility standard) and `RUSH_COLORS` (explicit control)
- **Multiple Color Schemes**: Default, dark, and light themes with customizable ANSI color codes
- **Colored Built-in Commands**: Enhanced output for `help`, `pwd`, `env` with contextual coloring
- **Error Highlighting**: Red coloring for error messages throughout the shell
- **Success Indicators**: Green coloring for successful operations
- **Runtime Configuration**: Dynamic color control with `set_colors` and `set_color_scheme` builtins

Example usage:

```bash
# Enable colors explicitly
export RUSH_COLORS=on

# Disable colors for accessibility
export NO_COLOR=1

# Switch color schemes
set_color_scheme dark
set_color_scheme light
set_color_scheme default

# Control colors dynamically
set_colors on
set_colors off
set_colors status  # Show current status
```

**Key Features:**

- **Smart Detection**: Automatically detects terminal capabilities and disables colors for non-interactive output
- **Accessibility**: Respects `NO_COLOR=1` environment variable for users who prefer monochrome output
- **Flexible Control**: `RUSH_COLORS` variable supports `auto`, `on`, `off`, `1`, `0`, `true`, `false` values
- **Multiple Themes**: Three built-in color schemes optimized for different terminal backgrounds
- **Contextual Coloring**: Different colors for prompts, errors, success messages, and builtin output
- **Performance**: Minimal overhead when colors are disabled

**Color Schemes:**

- **Default**: Standard ANSI colors (green prompt, red errors, cyan builtins, blue directories)
- **Dark**: Bright colors optimized for dark terminal backgrounds
- **Light**: Darker colors optimized for light terminal backgrounds

**Configuration Options:**

```bash
# Environment variables
export NO_COLOR=1           # Disable colors (accessibility)
export RUSH_COLORS=auto    # Auto-detect (default)
export RUSH_COLORS=on      # Force enable
export RUSH_COLORS=off     # Force disable

# Runtime commands
set_colors on              # Enable colors
set_colors off             # Disable colors
set_colors status          # Show current status

set_color_scheme default   # Standard colors
set_color_scheme dark      # Dark theme
set_color_scheme light     # Light theme
```

The color system is designed to be both powerful and unobtrusive, providing visual enhancements while respecting user preferences and accessibility needs.

### .rushrc Configuration File

Rush automatically sources a configuration file `~/.rushrc` when starting in interactive mode, similar to bash's `.bashrc`. This allows you to customize your shell environment with:

- **Environment Variables**: Set default variables and export them to child processes
- **Aliases**: Define command shortcuts that persist across the session
- **Shell Configuration**: Customize prompt, PATH, or other shell settings
- **Initialization Commands**: Run setup commands on shell startup

Example `~/.rushrc` file:

```bash
# Set environment variables
export EDITOR=vim
export PATH="$HOME/bin:$PATH"

# Create useful aliases
alias ll='ls -la'
alias ..='cd ..'
alias grep='grep --color=auto'

# Set custom variables
MY_PROJECTS="$HOME/projects"
WORKSPACE="$HOME/workspace"

# Display welcome message
echo "Welcome to Rush shell!"
echo "Type 'help' for available commands."
```

**Key Features:**

- **Automatic Loading**: Sourced automatically when entering interactive mode
- **Silent Failures**: Missing or invalid `.rushrc` files don't prevent shell startup
- **Variable Persistence**: Variables and aliases set in `.rushrc` are available throughout the session
- **Error Resilience**: Syntax errors in `.rushrc` are handled gracefully
- **Standard Location**: Uses `~/.rushrc` following Unix conventions

**Usage Notes:**

- Only loaded in interactive mode (not in script or command-line modes)
- Variables set in `.rushrc` are available to all subsequent commands
- Use `export` to make variables available to child processes
- Comments (lines starting with `#`) are ignored
- Multi-line constructs (if/fi, case/esac) are supported

## Installation

### Prerequisites

- Rust (edition 2024 or later)

### Cargo Installation

1. Install rush-sh from crates.io:

   ```bash
   cargo install rush-sh
   ```

### Build

1. Clone the repository:

   ```bash
   git clone https://github.com/drewwalton19216801/rush-sh.git
   cd rush-sh
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

The binary will be available at `target/release/rush-sh`.

## Usage

### Interactive Mode

Run the shell without arguments to enter interactive mode:

```bash
./target/release/rush-sh
```

or

```bash
rush-sh
```

You'll see a prompt showing the condensed current working directory followed by `$ ` (e.g., `/h/d/p/r/rush-sh $ `) where you can type commands. Type `exit` to quit.

**Configuration**: Rush automatically sources `~/.rushrc` on startup if it exists, allowing you to set up aliases, environment variables, and other customizations.

### Script Mode

Execute commands from a file:

```bash
./target/release/rush-sh script.sh
```

or

```bash
rush-sh script.sh
```

The shell will read and execute each line from the script file. Note that when using script mode, shebang lines (e.g., `#!/usr/bin/env bash`) are not bypassed - they are executed as regular comments.

### Command Mode

Execute a command string directly:

```bash
./target/release/rush-sh -c "echo Hello World"
```

or

```bash
rush-sh -c "echo Hello World"
```

The shell will execute the provided command string and exit.

### Source Command

The `source` (or `.`) built-in command provides a way to execute script files while bypassing shebang lines and comment lines that may specify other shells:

```bash
source script.sh
. script.sh
```

This is particularly useful for:

- Executing scripts written for rush that contain `#!/usr/bin/env rush-sh` shebangs
- Running scripts with shebangs for other shells (like `#!/usr/bin/env bash`) using rush instead
- Ensuring consistent execution environment regardless of shebang declarations
- Sharing variables between the sourced script and the parent shell

Unlike script mode (running `./target/release/rush-sh script.sh`), the `source` command automatically skips shebang lines and comment lines, and executes all commands using the rush interpreter. Variables set in the sourced script are available in the parent shell.

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
- Execute a script with dot: `. script.sh`
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
- Test builtin for conditional logic:
  - String tests: `if test -z "$VAR"; then echo "Variable is empty"; fi`
  - File tests: `if [ -f "/etc/passwd" ]; then echo "File exists"; fi`
  - Combined conditions: `if test -n "$NAME" && [ -d "/tmp" ]; then echo "Ready"; fi`
  - Error handling: `test -x "invalid"; echo "Exit code: $?"`
- Command substitution:
  - Basic substitution: `echo "Current dir: $(pwd)"`
  - Backtick syntax: `echo "Files: `ls | wc -l`"`
  - Variable assignments: `PROJECT_DIR="$(pwd)/src"`
  - Complex commands: `echo "Rust version: $(rustc --version | cut -d' ' -f2)"`
  - Error handling: `RESULT="$(nonexistent_command 2>/dev/null || echo 'failed')"`
  - With pipes: `$(echo hello | grep ll) > output.txt`
  - Multiple commands: `echo "Output: $(echo 'First'; echo 'Second')"`
- Tab completion:
  - Complete commands: `cd` → `cd `, `env `, `exit `
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
cargo test builtins
cargo test completion
cargo test executor
cargo test lexer
cargo test main
cargo test parser
cargo test state
cargo test integration
```

### Test Coverage

The test suite provides extensive coverage of:

- Command parsing and execution
- Built-in command functionality (cd, pwd, env, exit, help, source, export, unset, pushd, popd, dirs, alias, unalias, test, [)
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
