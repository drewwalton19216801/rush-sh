# Rush - A Unix shell written in Rust

**Version 0.6.0** - A comprehensive POSIX sh-compatible shell implementation

[![dependency status](https://deps.rs/repo/github/drewwalton19216801/rush-sh/status.svg)](https://deps.rs/repo/github/drewwalton19216801/rush-sh)

![Rush Logo](images/rush_logo.png)

Rush is a POSIX sh-compatible shell implemented in Rust (~90% POSIX compliant). It provides both interactive mode with a REPL prompt and script mode for executing commands from files. The shell supports comprehensive shell features including command execution, pipes, redirections, subshells, file descriptor operations, environment variables, and 20 built-in commands.

## Table of Contents

- [Features](#features)
- [What's New](#whats-new)
  - [🚀 Major Feature Additions](#-major-feature-additions)
  - [Detailed Feature Updates](#detailed-feature-updates)
- [Installation](#installation)
- [Usage](#usage)
  - [Interactive Mode](#interactive-mode)
  - [Script Mode](#script-mode)
  - [Command Mode](#command-mode)
  - [Source Command](#source-command)
  - [Examples](#examples)
- [Architecture](#architecture)
- [Dependencies](#dependencies)
- [Quality Assurance](#quality-assurance)
  - [Comprehensive Test Suite](#comprehensive-test-suite)
  - [Performance Benchmarking](#performance-benchmarking)
- [Contributing](#contributing)
- [License](#license)

## Pun-der the Hood

- In a hurry? Don’t bash your head against it—Rush it.
- When your pipelines need a drum solo, put them on Rush and let the commands Neil-Peart through.
- Tom Sawyer tip: chores go faster when you make them look like a Rush job; no need to paint the fence by hand when the shell can whitewash it with a one-liner.
- Alias your productivity: `alias hurry='rush -c "do the thing"'`—because sometimes you just need to rush to judgment.
- This shell doesn’t just run fast; it gives you the Rush of a clean exit status.

## Features

- **Command Execution**: Execute external commands and built-in commands.
- **Subshells**: Full POSIX-compliant subshell support with `(command)` syntax
  - State isolation with proper variable scoping
  - Exit code propagation from subshell to parent
  - Trap inheritance for signal handlers
  - Depth limit protection (max 100 levels)
  - 60+ comprehensive test cases
- **Pipes**: Chain commands using the `|` operator.
- **Redirections**: Input (`<`) and output (`>`, `>>`) redirections, here-documents (`<<`), and here-strings (`<<<`).
- **File Descriptor Operations**: Full POSIX-compliant file descriptor management
  - FD output redirection: `2>errors.log`, `3>output.txt`
  - FD input redirection: `3<input.txt`, `4<data.txt`
  - FD append: `2>>errors.log`, `3>>output.txt`
  - FD duplication: `2>&1` (stderr to stdout), `1>&2` (stdout to stderr), `3>&1`
  - FD closing: `2>&-` (close stderr), `3>&-`
  - FD read/write: `3<>file.txt` (open for both reading and writing)
  - Multiple redirections: `cmd >out.txt 2>err.txt 3>custom.txt`
  - FD swapping and advanced patterns
- **Brace Expansion**: Generate multiple strings from patterns with braces.
  - Comma-separated lists: `{a,b,c}` expands to `a b c`
  - Numeric ranges: `{1..5}` expands to `1 2 3 4 5`
  - Alphabetic ranges: `{a..e}` expands to `a b c d e`
  - Prefix/suffix combinations: `file{1,2,3}.txt` expands to `file1.txt file2.txt file3.txt`
  - Nested patterns: `{{a,b},{c,d}}` expands to `a b c d`
- **Command Substitution**: Execute commands and substitute their output inline **within the current shell context**.
  - `$(command)` syntax: `echo "Current dir: $(pwd)"`
  - `` `command` `` syntax: `echo "Files:`ls | wc -l`"`
  - **In-context execution**: Commands execute in the current shell, accessing functions, aliases, and variables
  - **Performance optimized**: No external process spawning for builtin commands
  - Variable expansion within substitutions: `echo $(echo $HOME)`
  - Error handling with fallback to literal syntax
- **Arithmetic Expansion**: Evaluate mathematical expressions using `$((...))` syntax.
  - Basic arithmetic: `echo $((2 + 3 * 4))`
  - Variable integration: `result=$((x * y + z))`
  - Comparison operations: `$((5 > 3))` returns 1 (true) or 0 (false)
  - Bitwise and logical operations: `$((5 & 3))`, `$((x && y))`
  - Operator precedence with parentheses support
- **Environment Variables**: Full support for variable assignment, expansion, and export.
  - Variable assignment: `VAR=value` and `VAR="quoted value"`
  - Variable expansion: `$VAR` and special variables (`$?`, `$$`, `$0`)
  - **Parameter Expansion with Modifiers**: Advanced variable expansion with POSIX sh modifiers
    - `${VAR:-default}` - use default if VAR is unset or null
    - `${VAR:=default}` - assign default if VAR is unset or null
    - `${VAR:+replacement}` - use replacement if VAR is set and not null
    - `${VAR:?error}` - display error if VAR is unset or null
    - `${VAR:offset}` - substring starting at offset
    - `${VAR:offset:length}` - substring with length
    - `${#VAR}` - length of VAR
    - `${VAR#pattern}` - remove shortest match from beginning
    - `${VAR##pattern}` - remove longest match from beginning
    - `${VAR%pattern}` - remove shortest match from end
    - `${VAR%%pattern}` - remove longest match from end
    - `${VAR/pattern/replacement}` - pattern substitution
    - `${VAR//pattern/replacement}` - global pattern substitution
    - `${!name}` - indirect expansion (bash extension)
    - `${!prefix*}` and `${!prefix@}` - variable name expansion (bash extension)
  - Export mechanism: `export VAR` and `export VAR=value`
  - Variable scoping: Shell variables vs exported environment variables
- **Positional Parameters**: Complete support for script arguments and parameter manipulation.
  - Individual parameters: `$1`, `$2`, `$3`, etc. for accessing script arguments
  - All parameters: `$*` and `$@` for accessing all arguments as a single string
  - Parameter count: `$#` returns the number of positional parameters
  - Parameter shifting: `shift [n]` builtin to shift positional parameters
  - Script argument passing: `./rush-sh script.sh arg1 arg2 arg3`
- **Control Structures**:
  - `if` statements: `if condition; then commands; elif condition; then commands; else commands; fi`
  - `case` statements with glob pattern matching: `case word in pattern1|pattern2) commands ;; *.txt) commands ;; *) default ;; esac`
  - `for` loops: `for variable in item1 item2 item3; do commands; done`
  - `while` loops: `while condition; do commands; done`
  - **Functions**: Complete function support with definition, calls, local variables, return statements, and recursion
    - Function definition: `name() { commands; }`
    - Function calls: `name arg1 arg2`
    - Local variables: `local var=value`
    - Return statements: `return [value]`
    - Function introspection: `declare -f [function_name]`
- **Built-in Commands** (20 total):
  - `cd`: Change directory
  - `exit`: Exit the shell
  - `pwd`: Print working directory
  - `env`: List environment variables
  - `export`: Export variables to child processes
  - `unset`: Remove variables
  - `shift`: Shift positional parameters
  - `source` / `.`: Execute a script file with rush (bypasses shebang and comment lines)
  - `pushd`: Push directory onto stack and change to it
  - `popd`: Pop directory from stack and change to it
  - `dirs`: Display directory stack
  - `alias`: Define or display aliases
  - `unalias`: Remove alias definitions
  - `test` / `[`: POSIX-compatible test builtin with string and file tests
  - `set_colors`: Enable/disable color output dynamically
  - `set_color_scheme`: Switch between color themes (default/dark/light)
  - `set_condensed`: Enable/disable condensed cwd display in prompt
  - `declare`: Display function definitions and list function names
  - `trap`: Set or display signal handlers
  - `help`: Show available commands
- **Configuration File**: Automatic sourcing of `~/.rushrc` on interactive shell startup
- **Tab Completion**: Intelligent completion for commands, files, and directories.
  - **Command Completion**: Built-in commands and executables from PATH
  - **File/Directory Completion**: Files and directories with relative paths
  - **Directory Traversal**: Support for nested paths (`src/`, `../`, `/usr/bin/`)
  - **Home Directory Expansion**: Completion for `~/` and `~/Documents/` paths
  - **Multi-Match Cycling**: Subsequent tab presses cycle through available completions when multiple matches exist
- **Real-Time Signal Handling**: Enhanced trap system with signal normalization, multiple handlers, trap display/reset, and signal queue with overflow protection. Traps execute immediately when signals are received during interactive sessions and script execution.
- **Line Editing and History**: Enhanced interactive experience with rustyline.

## What's New

### 🚀 Major Feature Additions

**Subshell Support** - Full POSIX-compliant subshell implementation with state isolation, exit code propagation, trap inheritance, and depth limit protection (max 100 levels). Includes 60+ comprehensive test cases covering all aspects of subshell behavior.

**File Descriptor Operations** - Complete FD table management with duplication (N>&M, N<&M), closing (N>&-, N<&-), and read/write (N<>) operations. Includes 30+ test cases for comprehensive coverage.

**Complete Control Structures** - Full implementation of POSIX control structures including `for` loops, `while` loops, and function definitions with local variable scoping, return statements, and recursion support.

**Function System** - Comprehensive function implementation with definition, calls, local variables (`local` keyword), return statements, recursion, and function introspection (`declare -f`).

**Complete POSIX Parameter Expansion** - Full implementation of `${VAR:-default}`, `${VAR#pattern}`, `${VAR/pattern/replacement}`, and all other POSIX parameter expansion modifiers with comprehensive pattern matching and string manipulation capabilities.

**Advanced Arithmetic Expansion** - Complete `$((...))` arithmetic expression evaluator with proper operator precedence, variable integration, bitwise operations, logical operations, and comprehensive error handling using the Shunting-yard algorithm.

**Enhanced Built-in Command Suite** - Comprehensive set of 20 built-in commands including directory stack management (`pushd`/`popd`/`dirs`), alias management (`alias`/`unalias`), color theming (`set_colors`/`set_color_scheme`), function introspection (`declare`), signal handling (`trap`), and POSIX-compliant `test` builtin.

**Intelligent Tab Completion** - Advanced completion system for commands, files, directories, and paths with support for nested directory traversal and home directory expansion.

## Detailed Feature Updates

### Subshell Support

Rush provides full POSIX-compliant subshell support, enabling command execution in isolated environments:

- **Syntax**: Execute commands in subshells using `(command)` syntax
- **State Isolation**: Subshells inherit parent state but modifications don't affect the parent
- **Exit Code Propagation**: Subshell exit codes are properly returned to the parent shell
- **Trap Inheritance**: Signal handlers (traps) are inherited from parent to subshell
- **Depth Protection**: Maximum subshell nesting depth of 100 levels prevents stack overflow
- **Variable Scoping**: Variables set in subshells don't leak to parent environment
- **FD Management**: File descriptor table is properly saved and restored

**Basic Usage:**

```bash
# Execute command in subshell
(cd /tmp && ls)
pwd  # Still in original directory

# Subshell with variable isolation
VAR=parent
(VAR=child; echo "In subshell: $VAR")
echo "In parent: $VAR"  # Still shows "parent"

# Exit code propagation
(exit 42)
echo "Subshell exit code: $?"  # Shows 42

# Trap inheritance
trap 'echo "Signal received"' INT
(sleep 10)  # Ctrl+C will trigger inherited trap
```

**Advanced Usage:**

```bash
# Complex command isolation
(
    cd /tmp
    export TEMP_VAR=value
    echo "Working in: $(pwd)"
)
# Original directory and environment preserved

# Pipeline with subshells
(echo "data" | grep "pattern") | wc -l

# Nested subshells (up to 100 levels)
(echo "Level 1"; (echo "Level 2"; (echo "Level 3")))

# Subshell with redirections
(echo "output" > file.txt; cat file.txt) 2>errors.log
```

**Key Features:**

- **POSIX Compliance**: Full compatibility with POSIX subshell specifications
- **Performance**: Efficient state management with minimal overhead
- **Safety**: Depth limit protection prevents infinite recursion
- **Integration**: Works seamlessly with all shell features (pipes, redirections, traps)
- **Test Coverage**: 60+ comprehensive test cases covering all subshell behaviors

**Implementation Details:**

- Subshells create isolated execution contexts with copied state
- File descriptor table is saved before subshell and restored after
- Trap handlers are inherited but can be modified independently
- Exit codes are properly propagated through nested subshells
- Maximum depth of 100 levels prevents stack overflow attacks

For practical examples, see the example scripts that demonstrate subshell usage in various scenarios.

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

### Brace Expansion

Rush now supports comprehensive brace expansion, a powerful feature for generating multiple strings from patterns:

- **Comma-Separated Lists**: `{a,b,c}` generates multiple alternatives
- **Numeric Ranges**: `{1..10}` generates sequences of numbers
- **Alphabetic Ranges**: `{a..z}` generates sequences of letters
- **Prefix/Suffix Combinations**: Combine braces with text for complex patterns
- **Nested Patterns**: Support for nested brace expressions
- **Integration**: Works seamlessly with all shell features (pipes, redirections, variables)

**Basic Syntax:**

```bash
# Comma-separated alternatives
echo {a,b,c}
# Output: a b c

echo file{1,2,3}.txt
# Output: file1.txt file2.txt file3.txt

# Numeric ranges
echo {1..5}
# Output: 1 2 3 4 5

echo test{1..3}.log
# Output: test1.log test2.log test3.log

# Alphabetic ranges
echo {a..e}
# Output: a b c d e

echo file{a..c}.txt
# Output: filea.txt fileb.txt filec.txt
```

**Advanced Usage:**

```bash
# Nested brace expansion
echo {{a,b},{c,d}}
# Output: a b c d

echo file{{1,2},{a,b}}.txt
# Output: file1.txt file2.txt filea.txt fileb.txt

# Multiple brace patterns in one command
echo {a,b}{1,2}
# Output: a1 a2 b1 b2

# With prefixes and suffixes
echo prefix_{a,b,c}_suffix
# Output: prefix_a_suffix prefix_b_suffix prefix_c_suffix
```

**Real-World Examples:**

```bash
# Create multiple directories
mkdir -p project/{src,test,docs}

# Create numbered backup files
cp important.txt important.txt.{1..5}

# Process multiple file types
cat file.{txt,md,log}

# Generate test data
echo user{1..100}@example.com

# Create directory structure
mkdir -p app/{controllers,models,views}/{admin,public}

# Batch file operations
mv photo{1..10}.jpg backup/

# Generate configuration files
touch config.{dev,staging,prod}.yml
```

**Integration with Other Features:**

```bash
# With pipes
echo {a,b,c} | tr ' ' '\n'

# With redirections
echo {1..5} > numbers.txt

# With variables
PREFIX="test"
echo ${PREFIX}{1..3}

# With command substitution
echo file{$(seq 1 5)}.txt

# In for loops
for file in document{1..5}.txt; do
    touch "$file"
done
```

**Key Features:**

- **POSIX-Compatible**: Follows standard brace expansion behavior
- **Performance**: Efficient expansion with minimal memory overhead
- **Error Handling**: Graceful handling of malformed patterns
- **Nested Support**: Full support for complex nested patterns
- **Integration**: Works with all shell features (variables, pipes, redirections)

**Implementation Details:**

- Brace expansion occurs after alias expansion but before parsing
- Patterns are detected and expanded by the lexer
- Supports both simple and complex nested patterns
- Maintains proper order of expansion for predictable results

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

Rush now supports comprehensive command substitution with both `$(...)` and `` `...` `` syntax, **executing commands within the current shell context** for optimal performance and functionality:

- **In-Context Execution**: Commands execute in the current shell, not external `/bin/sh` processes
- **Function Access**: Shell functions defined in the current session can be called in substitutions
- **Alias Expansion**: Aliases are expanded before execution in command substitutions
- **Variable Scope**: Access to local variables, shell variables, and exported environment variables
- **Performance**: 10-50x faster for builtin commands (no process spawning overhead)
- **Dual Syntax Support**: Both `$(command)` and `` `command` `` work identically
- **Variable Expansion**: Variables within substituted commands are properly expanded
- **Error Handling**: Failed commands fall back to literal syntax preservation
- **Multi-line Support**: Handles commands with multiple lines and special characters

**Architecture**:

Command substitution in Rush is implemented through a sophisticated multi-stage architecture that ensures seamless integration with the shell's execution model:

### **1. Lexical Recognition**

- **Syntax Detection**: The lexer identifies command substitution patterns (`$(...)` and `` `...` ``) during tokenization
- **Literal Preservation**: Substitutions are kept as literal tokens to preserve exact syntax for runtime expansion
- **No Premature Expansion**: Variables within substitutions are not expanded during lexing to maintain execution context

### **2. Parse-Time Handling**

- **AST Integration**: Command substitutions are embedded as string literals within the Abstract Syntax Tree
- **Deferred Processing**: The parser treats substitutions as regular word tokens, deferring expansion to execution time
- **Context Preservation**: Original quoting and whitespace context is maintained for accurate reconstruction

### **3. Runtime Execution Architecture**

The core execution engine implements a three-tier approach:

#### **A. Variable Expansion Engine** (`expand_variables_in_string`)

```rust
// Located in src/executor.rs:144-451
pub fn expand_variables_in_string(input: &str, shell_state: &mut ShellState) -> String
```

- **Pattern Recognition**: Scans input strings for `$()` and `` ` `` patterns
- **Recursive Processing**: Handles nested substitutions and complex expressions
- **Context Isolation**: Maintains separate execution context for each substitution

#### **B. Command Execution** (`execute_and_capture_output`)

```rust
// Located in src/executor.rs:10-130
fn execute_and_capture_output(ast: Ast, shell_state: &mut ShellState) -> Result<String, String>
```

- **Full Shell Context**: Executes commands using the complete shell execution pipeline
- **Output Capture**: Implements dual mechanisms for capturing command output:
  - **Pipe-based capture** for external commands
  - **Buffer-based capture** for builtin commands
- **State Inheritance**: Command substitutions inherit the full shell environment including:
  - Variables (local, global, exported)
  - Functions and aliases
  - Working directory and shell state

#### **C. State Management Integration**

- **Shared State**: Command substitutions access the same `ShellState` as the parent shell
- **Variable Scoping**: Local variables from functions are accessible within substitutions
- **Function Access**: Shell functions can be called within command substitutions
- **Alias Expansion**: Aliases are properly expanded before command execution

### **4. Output Integration**

- **Seamless Substitution**: Captured output is trimmed and integrated back into the original command string
- **Error Handling**: Failed commands fall back to literal syntax preservation
- **Multi-line Support**: Properly handles commands producing multiple lines of output

### **5. Performance Optimizations**

- **In-Context Execution**: Commands execute within the current shell process (no `/bin/sh` spawning)
- **Builtin Optimization**: Builtin commands avoid process creation entirely (10-50x performance improvement)
- **Memory Efficiency**: Uses streaming output capture to handle large command outputs
- **Lazy Evaluation**: Substitutions are only executed when actually encountered during expansion

### **6. Advanced Features**

- **Nested Substitutions**: Supports command substitutions within command substitutions
- **Complex Commands**: Handles pipelines, redirections, and control structures within substitutions
- **Error Recovery**: Graceful fallback to literal syntax when execution fails
- **Cross-Platform**: Consistent behavior across different operating systems

This architecture enables command substitutions to behave identically to bash while leveraging Rust's type safety and performance characteristics, providing a seamless experience for shell scripting and interactive use.

### Condensed Current Working Directory in Prompt

Rush displays a condensed version of the current working directory in the interactive prompt by default, but this behavior is now fully configurable:

- **Condensed Path**: Each directory except the last is abbreviated to its first letter (preserving case)
- **Full Last Directory**: The final directory in the path is shown in full
- **Dynamic Updates**: The prompt updates automatically when changing directories
- **Configurable Display**: Choose between condensed or full path display

**Configuration Options:**

```bash
# Environment variable (set before starting shell)
export RUSH_CONDENSED=true    # Enable condensed display (default)
export RUSH_CONDENSED=false   # Enable full path display

# Runtime control (in already running shell)
set_condensed on              # Enable condensed display
set_condensed off             # Enable full path display
set_condensed status          # Show current setting
```

**Example Prompt Displays:**

Condensed mode (default):

```bash
/h/d/p/r/rush $
/u/b/s/project $
/h/u/Documents $
```

Full path mode:

```bash
/home/drew/projects/rush-sh $
/usr/bin/src/project $
/home/user/Documents $
```

**Usage Examples:**

```bash
# Start with condensed display (default behavior)
rush-sh

# Start with full path display
RUSH_CONDENSED=false rush-sh

# Toggle in running shell
set_condensed off    # Switch to full paths
set_condensed on     # Switch back to condensed
set_condensed status # Check current setting

# In ~/.rushrc for permanent preference
echo 'export RUSH_CONDENSED=false' >> ~/.rushrc
```

This feature provides flexible control over prompt display while maintaining backward compatibility with the existing condensed format as the default.

### Arithmetic Expansion

Rush now supports comprehensive arithmetic expansion using the POSIX-standard `$((...))` syntax, enabling mathematical computations directly in shell commands and scripts:

- **Basic Arithmetic**: Addition, subtraction, multiplication, division, and modulo operations
- **Operator Precedence**: Standard mathematical precedence with parentheses support
- **Variable Integration**: Use shell variables directly in arithmetic expressions
- **Comparison Operations**: Less than, greater than, equal, not equal comparisons (return 1 for true, 0 for false)
- **Bitwise Operations**: AND, OR, XOR, shift operations for bit-level computations
- **Logical Operations**: AND, OR, NOT for boolean logic (return 1 for true, 0 for false)
- **Error Handling**: Division by zero and undefined variable detection

**Basic Syntax:**

```bash
# Simple arithmetic
echo "Result: $((2 + 3))"
echo "Multiplication: $((5 * 4))"
echo "Division: $((20 / 4))"
echo "Modulo: $((17 % 3))"
```

**Variable Usage:**

```bash
# Variables in arithmetic expressions
x=10
y=3
echo "x + y = $((x + y))"
echo "x * y = $((x * y))"
echo "x squared = $((x * x))"
```

**Operator Precedence:**

```bash
# Standard precedence: * / % before + -
echo "2 + 3 * 4 = $((2 + 3 * 4))"        # 14 (not 20)

# Use parentheses to override precedence
echo "(2 + 3) * 4 = $(((2 + 3) * 4))"    # 20

# Complex expressions
echo "2 * 3 + 4 * 5 = $((2 * 3 + 4 * 5))"  # 26
```

**Comparison Operations:**

```bash
# Comparisons return 1 (true) or 0 (false)
if [ $((5 > 3)) -eq 1 ]; then echo "5 is greater than 3"; fi
if [ $((10 == 10)) -eq 1 ]; then echo "Equal"; fi
if [ $((7 != 5)) -eq 1 ]; then echo "Not equal"; fi

# Available comparison operators:
# ==  !=  <  <=  >  >=
```

**Bitwise and Logical Operations:**

```bash
# Bitwise operations
echo "5 & 3 = $((5 & 3))"    # 1 (binary AND)
echo "5 | 3 = $((5 | 3))"    # 7 (binary OR)
echo "5 ^ 3 = $((5 ^ 3))"    # 6 (binary XOR)

# Logical operations (non-zero = true)
echo "5 && 3 = $((5 && 3))"  # 1 (both true)
echo "5 && 0 = $((5 && 0))"  # 0 (second false)
echo "0 || 5 = $((0 || 5))"  # 1 (second true)
```

**Real-world Examples:**

```bash
# Calculate area of rectangle
length=10
width=5
area=$((length * width))
echo "Area: $area"

# Temperature conversion
celsius=25
fahrenheit=$((celsius * 9 / 5 + 32))
echo "$celsius°C = ${fahrenheit}°F"

# Array length calculation (simulated)
items=8
per_page=3
pages=$(((items + per_page - 1) / per_page))
echo "Pages needed: $pages"

# Conditional logic with arithmetic
count=15
if [ $((count % 2)) -eq 0 ]; then
    echo "Even number"
else
    echo "Odd number"
fi
```

**Error Handling:**

```bash
# Division by zero produces an error
echo "Division by zero: $((5 / 0))"

# Undefined variables cause errors
echo "Undefined var: $((undefined_var + 1))"
```

**Advanced Usage:**

```bash
# Complex mathematical expressions
radius=5
pi=3
area=$((pi * radius * radius))
echo "Circle area: $area"

# Multiple operations in one expression
result=$(( (10 + 5) * 3 / 2 ))
echo "Complex result: $result"

# Use in variable assignments
x=10
y=$((x + 5))  # y = 15
z=$((y * 2))  # z = 30
```

Arithmetic expansion integrates seamlessly with all other shell features and works in interactive mode, scripts, and command strings.

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

### Newly Documented Features (Previously Implemented)

Several features were fully implemented but not properly documented in previous versions:

**For Loops** - Complete implementation with `for variable in items; do commands; done` syntax, supporting variable assignment, multiple items, and integration with all shell features.

**While Loops** - Full implementation with `while condition; do commands; done` syntax, supporting complex conditions, nested loops, and proper exit code handling.

**Function System** - Comprehensive function implementation including:

- Function definition: `name() { commands; }`
- Function calls with arguments: `name arg1 arg2`
- Local variable scoping: `local var=value`
- Return statements: `return [value]`
- Recursion support with configurable depth limits
- Function introspection: `declare -f [function_name]`
- Integration with all shell features (variables, expansions, control structures)

**Arithmetic Implementation:**

- Uses the Shunting-yard algorithm for proper operator precedence and associativity
- Token-based parsing converts infix expressions to Reverse Polish Notation (RPN)
- Integrated with shell state for seamless variable access during evaluation
- Comprehensive error handling with graceful fallback to literal syntax on errors

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

### Signal Handling with Trap

Rush now provides comprehensive signal handling through the POSIX-compliant `trap` builtin, enabling scripts to respond to signals and perform cleanup operations:

- **Real-Time Signal Execution**: Traps execute immediately when signals are received during interactive sessions and script execution
- **Signal Handlers**: Set custom handlers for signals like INT, TERM, HUP, etc.
- **EXIT Trap**: Execute cleanup code when the shell exits
- **Signal Names and Numbers**: Support for both signal names (INT, TERM) and numbers (2, 15)
- **Multiple Signals**: Set the same handler for multiple signals at once
- **Trap Display**: View all active traps with `trap` command
- **Trap Reset**: Reset traps to default behavior with `trap - SIGNAL`
- **Signal Listing**: List all available signals with `trap -l`
- **Exit Code Preservation**: Trap handlers preserve the original `$?` exit code per POSIX requirements

**Basic Usage:**

```bash
# Set trap for SIGINT (Ctrl+C)
trap 'echo "Interrupted! Cleaning up..."; exit 1' INT

# Set EXIT trap for cleanup
trap 'rm -rf /tmp/mytemp; echo "Cleanup complete"' EXIT

# Set trap for multiple signals
trap 'echo "Signal received"' INT TERM HUP

# Display all traps
trap

# Display specific trap
trap -p INT

# Reset trap to default
trap - INT

# Ignore signal
trap '' HUP

# List all signals
trap -l
```

**Advanced Usage:**

```bash
# Cleanup trap for temporary files
TEMP_DIR="/tmp/script_$$"
mkdir -p "$TEMP_DIR"
trap 'rm -rf "$TEMP_DIR"; echo "Temp files removed"' EXIT

# Signal handling in long operations
trap 'echo "Operation cancelled"; exit 130' INT TERM
for i in {1..100}; do
    echo "Processing $i..."
    sleep 1
done

# Variable expansion in traps
SCRIPT_START=$(date)
trap 'echo "Script started at: $SCRIPT_START"' EXIT

# Multiple commands in trap
trap 'echo "Cleaning up..."; rm -f *.tmp; echo "Done"' EXIT
```

**Key Features:**

- **POSIX Compliance**: Full compatibility with POSIX trap specifications
- **32 Signals Supported**: All standard Unix signals including HUP, INT, QUIT, TERM, USR1, USR2, etc.
- **EXIT Trap**: Special trap that executes on shell exit (normal or error)
- **Signal Validation**: Rejects uncatchable signals (KILL, STOP)
- **Flexible Syntax**: Supports signal names, numbers, and SIG prefix
- **Error Handling**: Graceful handling of invalid signals and malformed commands

**Real-Time Execution:**

As of v0.5.0, Rush Shell supports real-time signal trap execution during interactive sessions and script execution:

- **Immediate Response**: Traps execute as soon as signals are received, not just at shell exit
- **Safe Execution Points**: Signals are processed at safe points in the REPL loop and script execution
- **Signal Queue**: Signals are queued and processed to prevent race conditions
- **Performance**: <5μs overhead per REPL iteration with no noticeable impact on interactive performance
- **Bounded Queue**: Maximum 100 queued signals to prevent memory issues

**Implementation Details:**

- Trap handlers are stored in thread-safe storage (`Arc<Mutex<HashMap>>`)
- Signal events are queued by a dedicated signal handler thread
- Main thread processes signals at safe points (before prompt, after commands, during script execution)
- EXIT traps execute at all shell exit points
- Trap commands preserve the original exit code ($?)
- Variable expansion occurs at trap execution time
- Trap handlers can access all shell features (functions, variables, etc.)

**Integration with Shell Features:**

```bash
# Trap with command substitution
trap 'echo "Files in directory: $(ls | wc -l)"' WINCH

# Trap with arithmetic expansion
COUNT=0
trap 'COUNT=$((COUNT + 1)); echo "Signal count: $COUNT"' USR1

# Trap with functions
cleanup() {
    echo "Cleanup function called"
    rm -rf /tmp/mytemp
}
trap cleanup EXIT

# Trap in scripts
#!/usr/bin/env rush-sh
trap 'echo "Script interrupted at line $LINENO"' INT
# ... rest of script
```

The trap builtin provides robust signal handling for production shell scripts while maintaining full backward compatibility with existing Rush shell functionality.

The test builtin is fully integrated with Rush's control structures, enabling complex conditional logic in scripts while maintaining POSIX compatibility.

### Here-Documents and Here-Strings

Rush provides full support for here-documents (`<<`) and here-strings (`<<<`), enabling flexible multi-line and single-line input redirection:

- **Here-Documents**: Multi-line input terminated by a delimiter
- **Here-Strings**: Single-line string input without needing echo or printf
- **Variable Expansion**: Variables are expanded in both here-docs and here-strings
- **Quoted Delimiters**: Support for quoted delimiters to prevent expansion
- **Pipeline Integration**: Works seamlessly with pipes and other redirections

**Here-Document Syntax:**

```bash
# Basic here-document
cat << EOF
This is line 1
This is line 2
This is line 3
EOF

# Here-document with variable expansion
NAME="Alice"
cat << EOF
Hello, $NAME!
Welcome to Rush shell.
Your home directory is: $HOME
EOF

# Here-document with quoted delimiter (no expansion)
cat << 'EOF'
Variables like $HOME are not expanded
They appear literally: $USER
EOF

# Here-document in a pipeline
cat << EOF | grep "important"
This line is important
This line is not
Another important line
EOF

# Here-document with command
grep "pattern" << EOF
line with pattern here
another line
pattern appears again
EOF
```

**Here-String Syntax:**

```bash
# Basic here-string
cat <<< "Hello, World!"

# Here-string with variable expansion
MESSAGE="Rush shell is awesome"
cat <<< "$MESSAGE"

# Here-string with grep
grep "rush" <<< "I love rush shell"

# Here-string in pipeline
cat <<< "test data" | tr 'a-z' 'A-Z'

# Here-string with multiple words
wc -w <<< "count these five words here"

# Here-string with special characters
cat <<< "Special chars: $HOME, $(date), `pwd`"
```

**Advanced Usage:**

```bash
# Here-document for multi-line variable assignment
read -r -d '' SCRIPT << 'EOF'
#!/usr/bin/env rush-sh
echo "This is a script"
echo "With multiple lines"
EOF

# Here-document with indentation (content preserves spacing)
cat << EOF
    Indented line 1
        More indented line 2
    Back to first indent
EOF

# Here-string for quick testing
while read -r line; do
    echo "Processing: $line"
done <<< "single line input"

# Here-document with arithmetic expansion
cat << EOF
Result: $((2 + 3))
Calculation: $((10 * 5))
EOF

# Here-string with command substitution
cat <<< "Current directory: $(pwd)"

# Multiple here-documents in sequence
cat << EOF1
First document
EOF1

cat << EOF2
Second document
EOF2
```

**Real-World Examples:**

```bash
# Generate configuration file
cat << EOF > config.yml
server:
  host: localhost
  port: 8080
database:
  name: mydb
  user: $DB_USER
EOF

# Create SQL script
mysql -u root << EOF
CREATE DATABASE IF NOT EXISTS testdb;
USE testdb;
CREATE TABLE users (id INT, name VARCHAR(50));
INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob');
EOF

# Send email with here-document
mail -s "Subject" user@example.com << EOF
Dear User,

This is the email body.
It can span multiple lines.

Best regards,
Rush Shell
EOF

# Quick data processing with here-string
# Count words in a string
wc -w <<< "The quick brown fox jumps"

# Convert to uppercase
tr 'a-z' 'A-Z' <<< "convert this text"

# Search for pattern
grep -o "word" <<< "find word in this string"

# Process JSON with here-string
jq '.name' <<< '{"name": "Rush", "type": "shell"}'
```

**Key Features:**

- **POSIX Compliance**: Full compatibility with standard here-document and here-string syntax
- **Variable Expansion**: Automatic expansion of variables, command substitutions, and arithmetic
- **Quoted Delimiters**: Use quoted delimiters to prevent variable expansion in here-documents
- **Pipeline Support**: Works seamlessly with pipes and other shell features
- **Error Handling**: Graceful handling of missing delimiters and malformed input
- **Performance**: Efficient implementation using Rust's pipe and I/O primitives

**Implementation Details:**

- Here-documents collect input until the delimiter is found on a line by itself
- Here-strings provide the content directly as stdin without requiring a delimiter
- Both support full variable expansion unless delimiters are quoted
- Content is passed to commands via stdin using efficient pipe mechanisms
- Works with both built-in and external commands

### File Descriptor Operations

Rush provides comprehensive POSIX-compliant file descriptor management, enabling advanced I/O redirection and control over file descriptors beyond the standard stdin (0), stdout (1), and stderr (2):

- **FD Output Redirection**: Redirect any file descriptor to a file
- **FD Input Redirection**: Open files for reading on custom file descriptors
- **FD Append**: Append output from any file descriptor to a file
- **FD Duplication**: Duplicate file descriptors to combine or swap streams
- **FD Closing**: Explicitly close file descriptors to suppress output
- **FD Read/Write**: Open files for both reading and writing
- **Multiple Redirections**: Apply multiple redirections to a single command
- **Advanced Patterns**: FD swapping, stream separation, and complex I/O routing

**Basic FD Redirection:**

```bash
# Redirect stderr to a file (FD 2)
ls /nonexistent 2>errors.log

# Redirect stdout to one file, stderr to another
command >output.log 2>errors.log

# Append stderr to a file
command 2>>errors.log

# Open file for reading on FD 3
cat 3<input.txt

# Open file for reading and writing on FD 3
cat 3<>file.txt
```

**FD Duplication:**

```bash
# Redirect stderr to stdout (combine streams)
command 2>&1

# Redirect stdout to stderr
echo "Error message" 1>&2

# Redirect FD 3 to stdout
command 3>&1

# Combine stderr into stdout, then pipe both
command 2>&1 | grep pattern
```

**FD Closing:**

```bash
# Close stderr to suppress error messages
command 2>&-

# Close FD 3
command 3>&-

# Discard all output
command >/dev/null 2>&1
```

**Advanced Patterns:**

```bash
# Separate stdout and stderr to different files
command >output.txt 2>errors.txt

# Combine streams and redirect to file
command >combined.log 2>&1

# Swap stdout and stderr
command 3>&1 1>&2 2>&3 3>&-

# Multiple custom file descriptors
command 3>custom1.txt 4>custom2.txt 5>custom3.txt

# Pipeline with error handling
command 2>&1 | tee output.log | grep ERROR

# Conditional error logging
if ! command 2>errors.log; then
    cat errors.log
    exit 1
fi
```

**Practical Use Cases:**

```bash
# Logging: Separate normal output from errors
./script.sh >output.log 2>errors.log

# Debugging: Capture stderr while displaying stdout
command 2>debug.log

# Silent execution: Discard all output
command >/dev/null 2>&1

# Error analysis: Capture and process errors
command 2>&1 | grep -i error | tee error_summary.txt

# Stream separation: Process stdout and stderr differently
(command | process_output) 2> >(process_errors)

# Backup with logging
tar czf backup.tar.gz /data 2>backup_errors.log

# Complex pipelines with error tracking
(command1 2>&1 | command2) 2>pipeline_errors.log
```

**Key Features:**

- **POSIX Compliance**: Full compatibility with POSIX file descriptor operations
- **FD Range**: Support for file descriptors 0-9
- **State Management**: Proper FD lifecycle management with save/restore capabilities
- **Error Handling**: Comprehensive error reporting for invalid operations
- **Integration**: Works seamlessly with pipes, redirections, and all shell features
- **Performance**: Efficient implementation using Rust's file descriptor primitives

**Implementation Details:**

- File descriptors are managed through a dedicated `FileDescriptorTable` in shell state
- FD operations are parsed during lexing and applied during command execution
- Supports duplication, closing, and read/write modes
- Proper cleanup and restoration of file descriptors after command execution
- Thread-safe implementation for concurrent operations
- Comprehensive test coverage with 20+ test cases

For a complete demonstration of file descriptor operations, see [`examples/fd_redirection_demo.sh`](examples/fd_redirection_demo.sh).

### Positional Parameters

Rush now provides comprehensive support for positional parameters, enabling scripts to access and manipulate command-line arguments with full POSIX compliance:

- **Individual Parameters**: Access script arguments using `$1`, `$2`, `$3`, etc.
- **All Parameters**: `$*` and `$@` provide access to all arguments as a single string
- **Parameter Count**: `$#` returns the number of positional parameters as a string
- **Parameter Shifting**: `shift [n]` builtin command to manipulate parameter positions
- **Script Integration**: Automatic argument passing when running scripts with `./rush-sh script.sh arg1 arg2`

**Basic Usage:**

```bash
# Create a script that uses positional parameters
cat > greet.sh << 'EOF'
#!/usr/bin/env rush-sh
echo "Hello $1!"
echo "You provided $# arguments"
echo "All arguments: $*"
EOF

# Make it executable and run with arguments
chmod +x greet.sh
./rush-sh greet.sh World
# Output: Hello World!
#         You provided 1 arguments
#         All arguments: World
```

**Advanced Usage:**

```bash
# Script with multiple arguments
cat > process.sh << 'EOF'
#!/usr/bin/env rush-sh
echo "Script name: $0"
echo "First arg: $1"
echo "Second arg: $2"
echo "Total args: $#"

# Shift parameters
echo "Shifting..."
shift
echo "New first arg: $1"
echo "New arg count: $#"
EOF

./rush-sh process.sh file1.txt file2.txt
# Output: Script name: process.sh
#         First arg: file1.txt
#         Second arg: file2.txt
#         Total args: 2
#         Shifting...
#         New first arg: file2.txt
#         New arg count: 1

**Running the Demo Script:**

To see all positional parameter features in action, run the demonstration script with multiple arguments:

```bash
./rush-sh examples/positional_parameters_demo.sh hello world test arguments
```

This will demonstrate:

- Individual parameter access (`$1`, `$2`, `$3`, `$4`)
- Parameter counting (`$#`)
- All parameters display (`$*`, `$@`)
- Parameter shifting with `shift` command
- Custom shift counts with `shift 2`

The script provides comprehensive output showing how each feature works with the provided arguments.

**Parameter Manipulation:**

```bash
# Using shift with custom count
cat > multi_shift.sh << 'EOF'
#!/usr/bin/env rush-sh
echo "Original args: $*"
echo "Count: $#"

# Shift by 2
shift 2
echo "After shift 2: $*"
echo "New count: $#"
EOF

./rush-sh multi_shift.sh a b c d e
# Output: Original args: a b c d e
#         Count: 5
#         After shift 2: c d e
#         New count: 3
```

**Key Features:**

- **POSIX Compliance**: Follows standard shell parameter expansion behavior
- **Variable Integration**: Works seamlessly with all other shell features
- **Error Handling**: Graceful handling of out-of-bounds parameter access
- **Multi-Mode Support**: Available in interactive mode, scripts, and command strings
- **Performance**: Efficient parameter storage and access

**Integration with Other Features:**

```bash
# Positional parameters with control structures
cat > check_args.sh << 'EOF'
#!/usr/bin/env rush-sh
if [ $# -eq 0 ]; then
    echo "No arguments provided"
    exit 1
fi

# Process each argument
for arg in $*; do
    if [ -f "$arg" ]; then
        echo "File: $arg"
    elif [ -d "$arg" ]; then
        echo "Directory: $arg"
    else
        echo "Other: $arg"
    fi
done
EOF

./rush-sh check_args.sh /tmp /etc/passwd nonexistent
# Output: Directory: /tmp
#         File: /etc/passwd
#         Other: nonexistent
```

**Implementation Details:**

- Parameters are stored efficiently in the shell state
- Variable expansion handles parameter access during lexing and execution
- Shift operations modify the parameter array in place
- All parameter operations maintain O(1) access time for individual parameters

### Parameter Expansion with Modifiers

Rush now supports comprehensive POSIX sh parameter expansion with modifiers, providing powerful string manipulation capabilities directly in shell commands and scripts:

- **Basic Expansion**: `${VAR}` - Simple variable expansion (equivalent to `$VAR`)
- **Default Values**: `${VAR:-default}` - Use default if VAR is unset or null
- **Assign Default**: `${VAR:=default}` - Assign default if VAR is unset or null
- **Alternative Values**: `${VAR:+replacement}` - Use replacement if VAR is set and not null
- **Error Handling**: `${VAR:?error}` - Display error if VAR is unset or null
- **Substring Operations**:
  - `${VAR:offset}` - Extract substring starting at offset
  - `${VAR:offset:length}` - Extract substring with specified length
- **Length Operations**: `${#VAR}` - Get length of variable content
- **Pattern Removal**:
  - `${VAR#pattern}` - Remove shortest match from beginning
  - `${VAR##pattern}` - Remove longest match from beginning
  - `${VAR%pattern}` - Remove shortest match from end
  - `${VAR%%pattern}` - Remove longest match from end
- **Pattern Substitution**:
  - `${VAR/pattern/replacement}` - Replace first occurrence
  - `${VAR//pattern/replacement}` - Replace all occurrences
- **Indirect Expansion** (bash extension):
  - `${!name}` - Indirect variable reference (value of variable named by name)
  - `${!prefix*}` and `${!prefix@}` - Names of variables starting with prefix

**Basic Usage:**

```bash
# Set a variable
MY_PATH="/usr/local/bin:/usr/bin:/bin"

# Default values
echo "Home: ${HOME:-/home/user}"
echo "Editor: ${EDITOR:-vim}"

# Assign default if unset
echo "Setting default..."
echo "Editor: ${EDITOR:=nano}"

# Alternative values
echo "Verbose: ${VERBOSE:+enabled}"

# Error handling
echo "Required var: ${REQUIRED_VAR:?This variable must be set}"
```

**Substring Operations:**

```bash
# Extract parts of strings
FILENAME="document.txt"
echo "Extension: ${FILENAME:9}"           # "txt"
echo "Name only: ${FILENAME:0:8}"         # "document"

# Length operations
echo "Length: ${#FILENAME}"               # "13"

# Pattern-based length
LONG_STRING="hello world"
echo "Length: ${#LONG_STRING}"            # "11"
```

**Pattern Removal:**

```bash
# Remove file extensions
FILENAME="document.txt"
echo "No extension: ${FILENAME%.txt}"     # "document"
echo "No extension: ${FILENAME%%.txt}"    # "document"

# Remove directory paths
FULL_PATH="/usr/bin/ls"
echo "Basename: ${FULL_PATH##*/}"         # "ls"
echo "Directory: ${FULL_PATH%/*}"         # "/usr/bin"

# Remove prefixes
PREFIXED="prefix_value"
echo "No prefix: ${PREFIXED#prefix_}"     # "value"
```

**Pattern Substitution:**

```bash
# Replace substrings
GREETING="hello world"
echo "Replace first: ${GREETING/world/universe}"    # "hello universe"
echo "Replace all: ${GREETING//l/L}"                # "heLLo worLd"

# Multiple replacements
PATH_LIST="/usr/bin:/bin:/usr/local/bin"
echo "Clean path: ${PATH_LIST//:/ }"               # "/usr/bin /bin /usr/local/bin"
```

**Advanced Usage:**

```bash
# Complex string manipulation
URL="https://example.com/path/to/resource"
echo "Domain: ${URL#*//}"                          # "example.com/path/to/resource"
echo "Domain: ${URL#*//}" | cut -d/ -f1            # "example.com"
echo "Path: ${URL#*/}"                             # "path/to/resource"

# Safe variable handling
CONFIG_FILE="/etc/app.conf"
echo "Config: ${CONFIG_FILE:-/etc/default.conf}"

# Indirect expansion - dynamic variable access
VAR_NAME="MESSAGE"
MESSAGE="Hello World"
echo "Indirect: ${!VAR_NAME}"                      # "Hello World"

# Indirect expansion - list matching variables
MY_VAR1="value1"
MY_VAR2="value2"
MY_VAR3="value3"
echo "All MY_ vars: ${!MY_*}"                      # "MY_VAR1 MY_VAR2 MY_VAR3"
```

**Indirect Expansion:**

```bash
# Basic indirect expansion - ${!name}
# Access variable whose name is stored in another variable
TARGET="HOME"
echo "Value: ${!TARGET}"                           # Expands to value of $HOME

# Practical use case - configuration selection
ENV="production"
production_db="prod.example.com"
development_db="dev.example.com"
DB_VAR="${ENV}_db"
echo "Database: ${!DB_VAR}"                        # "prod.example.com"

# Prefix-based indirect expansion - ${!prefix*}
# List all variables starting with a prefix
PATH_VAR1="/usr/bin"
PATH_VAR2="/usr/local/bin"
PATH_VAR3="/opt/bin"
echo "All PATH_ vars: ${!PATH_*}"                  # "PATH_VAR1 PATH_VAR2 PATH_VAR3"

# Works with both global and local variables
myfunc() {
    local LOCAL_VAR1="a"
    local LOCAL_VAR2="b"
    echo "Local vars: ${!LOCAL_*}"                 # "LOCAL_VAR1 LOCAL_VAR2"
}
```

**Integration with Other Features:**

```bash
# Parameter expansion in control structures
FILENAME="test.txt"
if [ -f "${FILENAME%.txt}.bak" ]; then
    echo "Backup exists for ${FILENAME%.txt}"
fi

# In arithmetic expressions
COUNT=42
echo "Count: $((COUNT + 1))"

# With command substitution
DIR_COUNT=$(find /tmp -type d | wc -l)
echo "Directories: ${DIR_COUNT:-0}"

# In case statements
case "${FILENAME##*.}" in
    txt) echo "Text file" ;;
    jpg|png) echo "Image file" ;;
    *) echo "Other type" ;;
esac
```

**Key Features:**

- **POSIX Compliance**: Full compatibility with standard parameter expansion syntax
- **Performance**: Efficient string operations with minimal overhead
- **Safety**: Robust error handling for edge cases and invalid operations
- **Integration**: Works seamlessly with all other shell features
- **Multi-Mode Support**: Available in interactive mode, scripts, and command strings
- **Error Resilience**: Graceful fallback for malformed expressions

**Implementation Details:**

- Parameter expansion is handled during the lexing phase for optimal performance
- Pattern matching uses simple string operations for reliability
- All operations maintain compatibility with existing variable expansion
- Comprehensive error handling prevents shell crashes from malformed expressions
- Memory efficient implementation suitable for large variable values

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

You'll see a prompt showing the condensed current working directory followed by `$` (e.g., `/h/d/p/r/rush-sh $`) where you can type commands. Type `exit` to quit.

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
- Execute positional parameters demo: `source examples/positional_parameters_demo.sh`
- Execute functions demo (comprehensive): `source examples/functions_demo.sh`
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
- Positional parameters:
  - Access arguments: `echo "First arg: $1, Second: $2"`
  - Argument count: `echo "You provided $# arguments"`
  - All arguments: `echo "All args: $*"`
  - Shift parameters: `shift; echo "New first: $1"`
  - Custom shift: `shift 2; echo "After shift 2: $*"`
- Subshells:
  - Basic subshell: `(cd /tmp && ls); pwd  # Still in original directory`
  - Variable isolation: `VAR=parent; (VAR=child; echo $VAR); echo $VAR  # Shows parent`
  - Exit code: `(exit 42); echo $?  # Shows 42`
  - Nested subshells: `(echo "Level 1"; (echo "Level 2"))`
  - With redirections: `(echo "output" > file.txt; cat file.txt) 2>errors.log`
- Use control structures:
  - If statement: `if true; then echo yes; else echo no; fi`
  - If-elif-else statement: `if false; then echo no; elif true; then echo yes; else echo maybe; fi`
  - Case statement with glob patterns:
    - Simple match: `case hello in hello) echo match ;; *) echo no match ;; esac`
    - Glob patterns: `case file.txt in *.txt) echo "Text file" ;; *.jpg) echo "Image" ;; *) echo "Other" ;; esac`
    - Multiple patterns: `case file in *.txt|*.md) echo "Document" ;; *.exe|*.bin) echo "Executable" ;; *) echo "Other" ;; esac`
    - Character classes: `case letter in [abc]) echo "A, B, or C" ;; *) echo "Other letter" ;; esac`
  - For loops: `for i in 1 2 3; do echo "Number: $i"; done`
  - While loops: `while [ $count -lt 5 ]; do echo "Count: $count"; count=$((count + 1)); done`
  - Functions:
    - Define function: `myfunc() { echo "Hello $1"; }`
    - Call function: `myfunc world`
    - Local variables: `local var="value"`
    - Return values: `return 42`
    - Function introspection: `declare -f myfunc`
- Test builtin for conditional logic:
  - String tests: `if test -z "$VAR"; then echo "Variable is empty"; fi`
  - File tests: `if [ -f "/etc/passwd" ]; then echo "File exists"; fi`
  - Combined conditions: `if test -n "$NAME" && [ -d "/tmp" ]; then echo "Ready"; fi`
  - Error handling: `test -x "invalid"; echo "Exit code: $?"`
- Command substitution:
  - Basic substitution: `echo "Current dir: $(pwd)"`
  - Backtick syntax: `echo "Files:`ls | wc -l`"`
  - Variable assignments: `PROJECT_DIR="$(pwd)/src"`
  - Complex commands: `echo "Rust version: $(rustc --version | cut -d' ' -f2)"`
  - Error handling: `RESULT="$(nonexistent_command 2>/dev/null || echo 'failed')"`
  - With pipes: `$(echo hello | grep ll) > output.txt`
  - Multiple commands: `echo "Output: $(echo 'First'; echo 'Second')"`
- Arithmetic expansion:
  - Basic arithmetic: `echo "Result: $((2 + 3 * 4))"`
  - Variable calculations: `result=$((x * y + z))`
  - Comparisons: `if [ $((count % 2)) -eq 0 ]; then echo "Even"; fi`
  - Complex expressions: `area=$((length * width))`
  - Temperature conversion: `fahrenheit=$((celsius * 9 / 5 + 32))`
- Parameter expansion with modifiers:
  - Default values: `echo "Home: ${HOME:-/home/user}"`
  - Substring extraction: `echo "Extension: ${FILENAME:9}"`
  - Pattern removal: `echo "Basename: ${FULL_PATH##*/}"`
  - Pattern substitution: `echo "Replaced: ${TEXT/old/new}"`
  - Length operations: `echo "Length: ${#VARIABLE}"`
  - Indirect expansion: `echo "Value: ${!VAR_NAME}"` and `echo "Vars: ${!PREFIX*}"`
- Here-documents and here-strings:
  - Here-document: `cat << EOF` (multi-line input)
  - Here-string: `grep pattern <<< "search text"` (single-line input)
- Brace expansion:
  - Simple lists: `echo {a,b,c}` → `a b c`
  - Numeric ranges: `echo {1..5}` → `1 2 3 4 5`
  - Alphabetic ranges: `echo {a..c}` → `a b c`
  - With prefix/suffix: `echo file{1,2,3}.txt` → `file1.txt file2.txt file3.txt`
  - Nested patterns: `echo {{a,b},{c,d}}` → `a b c d`
  - Multiple patterns: `echo {a,b}{1,2}` → `a1 a2 b1 b2`
  - Create directories: `mkdir -p project/{src,test,docs}`
  - Batch operations: `touch file{1..10}.txt`
- File descriptor operations:
  - FD output redirection: `command 2>errors.log`
  - FD input redirection: `cat 3<input.txt`
  - FD append: `command 2>>errors.log`
  - FD duplication: `command 2>&1` (combine stderr into stdout)
  - FD closing: `command 2>&-` (close stderr)
  - FD read/write: `cat 3<>file.txt`
  - Multiple redirections: `command >out.txt 2>err.txt 3>custom.txt`
  - Stream separation: `./script.sh >output.log 2>errors.log`
  - Discard output: `command >/dev/null 2>&1`
  - Demo script: `source examples/fd_redirection_demo.sh`
- Tab completion:
  - Complete commands: `cd` → `cd`, `env`, `exit`
  - Complete files: `cat f` → `cat file.txt`
  - Complete directories: `cd src/` → `cd src/main/`
  - Complete from PATH: `l` → `ls`, `g` → `grep`
  - Complete nested paths: `ls src/m` → `ls src/main/`

## Architecture

Rush consists of the following components:

- **Lexer**: Tokenizes input into commands, operators, and variables with support for variable expansion, parameter expansion with modifiers (`${VAR:-default}`, `${VAR#pattern}`, etc.), command substitution (`$(...)` and `` `...` `` syntax), arithmetic expansion (`$((...))`), brace expansion (`{a,b,c}`, `{1..5}`), and alias expansion.
- **Parser**: Builds an Abstract Syntax Tree (AST) from tokens, including support for complex control structures, case statements with glob patterns, and variable assignments.
- **Brace Expansion Engine**: A comprehensive brace expansion system implemented in [`src/brace_expansion.rs`](src/brace_expansion.rs) that supports:
  - **Pattern Detection**: Identifies brace patterns during lexing with support for nested braces
  - **Comma-Separated Lists**: Expands `{a,b,c}` into multiple alternatives
  - **Range Expansion**: Numeric (`{1..10}`) and alphabetic (`{a..z}`) range generation
  - **Prefix/Suffix Handling**: Combines braces with surrounding text for complex patterns
  - **Nested Patterns**: Recursive expansion of nested brace expressions
  - **Error Handling**: Graceful handling of malformed patterns with clear error messages
- **Executor**: Executes the AST, handling pipes, redirections, built-ins, glob pattern matching, environment variable inheritance, command substitution execution, arithmetic expression evaluation, and brace expansion.
- **Arithmetic Engine**: A comprehensive arithmetic expression evaluator implemented in [`src/arithmetic.rs`](src/arithmetic.rs) that supports:
  - **Token-based parsing**: Converts expressions to tokens and uses the Shunting-yard algorithm for proper operator precedence
  - **Variable integration**: Seamlessly accesses shell variables during evaluation
  - **Comprehensive operators**: Arithmetic, comparison, bitwise, and logical operations with correct precedence
  - **Error handling**: Robust error reporting for syntax errors, division by zero, and undefined variables
  - **Unary operators**: Support for both logical NOT (`!`) and bitwise NOT (`~`) operations
- **Parameter Expansion Engine**: A comprehensive parameter expansion system implemented in [`src/parameter_expansion.rs`](src/parameter_expansion.rs) that supports:
  - **Modifier parsing**: Sophisticated parsing of POSIX sh parameter expansion modifiers
  - **String operations**: Default values, substring extraction, pattern removal, and substitution
  - **Indirect expansion**: Dynamic variable access with `${!name}` and variable name listing with `${!prefix*}`
  - **Error handling**: Robust error reporting for malformed expressions and edge cases
  - **Performance**: Efficient string manipulation with minimal memory allocation
- **Shell State**: Comprehensive state management for environment variables, exported variables, special variables (`$?`, `$$`, `$0`), current directory, directory stack, and command aliases.
- **Built-in Commands**: Optimized detection and execution of built-in commands including variable management (`export`, `unset`, `env`) and alias management (`alias`, `unalias`).
- **Completion**: Provides intelligent tab-completion for commands, files, and directories.

## Dependencies

- `clap`: For command-line argument parsing with derive macros.
- `rustyline`: For interactive line editing and history with signal handling support.
- `signal-hook`: For robust signal handling (SIGINT, SIGTERM).
- `glob`: For pattern matching in case statements and wildcard expansion.
- `lazy_static`: For global state management in tab completion.

## Quality Assurance

### Comprehensive Test Suite

Rush includes an extensive test suite with **413+ test functions** ensuring reliability and correctness:

- **Unit Tests**: Individual component testing for lexer, parser, arithmetic engine, and parameter expansion
- **Integration Tests**: End-to-end command execution, pipelines, redirections, and control structures
- **Built-in Command Tests**: Comprehensive coverage of all built-in command functionality
- **Error Handling Tests**: Robust testing of edge cases, syntax errors, and failure scenarios
- **Feature-Specific Tests**: Dedicated test suites for arithmetic expansion, parameter expansion, and POSIX compliance

**Test Coverage Areas:**

- Command parsing and execution
- Variable expansion and parameter modifiers
- Arithmetic expression evaluation
- Control structures (if/elif/else, case statements)
- Built-in command functionality
- Pipeline and redirection handling
- Tab completion system
- Error conditions and edge cases

### Testing

Run the complete test suite with:

### Test Structure

- **Lexer Tests** Tokenization of commands, arguments, operators, quotes, variable expansion, command substitution, arithmetic expansion, and edge cases.
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

### Performance Benchmarking

Rush includes a comprehensive performance benchmark suite for measuring and tracking shell performance across all major components:

#### Running Benchmarks

Run the complete benchmark suite from the repository root:

```bash
cargo run -p rush-benchmarks
```

This will execute 20+ benchmark scenarios covering:

- **Lexer Performance**: Tokenization speed for basic and complex commands
- **Parser Performance**: AST construction for various command structures
- **Executor Performance**: Command execution speed for built-ins and external commands
- **Expansion Performance**: Variable, arithmetic, and command substitution performance
- **Control Structure Performance**: If statements, loops, and case statement execution
- **Pipeline Performance**: Simple and complex pipeline execution
- **Script Execution Performance**: Full script file execution and command-line mode

#### Benchmark Output

The benchmark suite generates:

- **Interactive Progress**: Real-time progress display during execution
- **Performance Metrics**: Detailed timing for each benchmark category
- **HTML Report**: Visual report saved to `target/benchmark_report.html`
- **JSON Results**: Machine-readable results in `target/benchmark_results.json`
- **Regression Detection**: Basic performance regression analysis

#### Example Output

```bash
🚀 Rush Shell Performance Benchmark Suite
==========================================

📝 Registering lexer benchmarks...
🔍 Registering parser benchmarks...
⚡ Registering executor benchmarks...
🔄 Registering expansion benchmarks...
🏗️  Registering control structure benchmarks...
🔗 Registering pipeline benchmarks...
📜 Registering script benchmarks...

📊 Running benchmarks...
  Running: lexer_basic_tokens (Basic tokenization (simple commands))
  Running: lexer_complex_tokens (Complex tokenization (quotes, variables, expansions))
  ...

📈 Generating report...

✅ Benchmark completed!
📊 Results summary:
   Total benchmarks: 21
   Total time: 2.34s

📋 Detailed report saved to: target/benchmark_report.html
📋 JSON results saved to: target/benchmark_results.json
```

#### Advanced Usage

Run benchmarks with custom iteration counts:

```bash
# Use the library API for custom configurations
# See benchmarks/src/main.rs for implementation details
```

View the generated HTML report in a browser:

```bash
# Open target/benchmark_report.html in your browser
# or serve it locally:
python3 -m http.server 8000 -d target/
# Then visit http://localhost:8000/benchmark_report.html
```

#### Benchmark Categories

The benchmark suite covers all major shell components:

1. **Lexer Benchmarks**: Tokenization performance for various command types
2. **Parser Benchmarks**: AST construction speed for complex structures
3. **Executor Benchmarks**: Command execution performance
4. **Expansion Benchmarks**: Variable and arithmetic expansion speed
5. **Control Structure Benchmarks**: If/for/while/case statement performance
6. **Pipeline Benchmarks**: Pipe and redirection performance
7. **Script Benchmarks**: Full script execution performance

#### Performance Monitoring

The benchmark suite is designed for:

- **Regression Detection**: Track performance changes over time
- **Optimization Validation**: Verify performance improvements
- **CI/CD Integration**: Automated performance testing in build pipelines
- **Historical Tracking**: JSON export enables trend analysis
- **Component Analysis**: Detailed breakdown of performance by shell component

This benchmark suite provides a foundation for maintaining optimal shell performance and identifying performance regressions during development.

### Test Coverage

The test suite provides extensive coverage of:

- Command parsing and execution
- Built-in command functionality (all 20 built-in commands including cd, pwd, env, exit, help, source, export, unset, shift, pushd, popd, dirs, alias, unalias, test, [, set_colors, set_color_scheme, set_condensed, declare, trap)
- **Subshells** (state isolation, exit code propagation, trap inheritance, depth limits, 60+ test cases)
- **File descriptor operations** (duplication, closing, read/write modes, 30+ test cases)
- Pipeline and redirection handling
- Control structures (if-elif-else statements, case statements with glob patterns, for loops, while loops)
- **Functions** (definition, calls, local variables, return statements, recursion, introspection)
- Command substitution (`$(...)` and `` `...` `` syntax, error handling, variable expansion)
- **Arithmetic expansion** (`$((...))` syntax, operator precedence, variable integration, error handling)
- **Positional parameters** (`$1`, `$2`, `$*`, `$@`, `$#`, `shift` command)
- **Parameter expansion with modifiers** (`${VAR:-default}`, `${VAR#pattern}`, `${VAR/pattern/replacement}`, etc.)
- **Here-documents and here-strings** (`<<` and `<<<` with proper expansion handling)
- Environment variable support (assignment, expansion, export, special variables)
- Variable scoping and inheritance
- Tab-completion for commands, files, and directories
- Path traversal and directory completion
- Error conditions and edge cases
- Signal handling integration with trap system

## Contributing

Contributions are welcome! Please fork the repository, make your changes, and submit a pull request.

## License

This project is licensed under the MIT License. See [LICENSE.txt](LICENSE.txt) for details.

## Project URL

[https://github.com/drewwalton19216801/rush-sh](https://github.com/drewwalton19216801/rush-sh)
