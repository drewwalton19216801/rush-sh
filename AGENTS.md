# Rush Shell - AI Agent Guide

## Project Overview

**Rush** is a comprehensive POSIX sh-compatible shell implementation written in Rust, currently at version 0.6.7. The project aims to provide a fully compliant POSIX shell while leveraging Rust's type safety, performance, and memory management capabilities.

### Project Goals

- **POSIX Compliance**: Achieve 100% compliance with IEEE Std 1003.1-2008 (POSIX sh)
- **Performance**: Leverage Rust's zero-cost abstractions for optimal execution speed
- **Reliability**: Extensive test coverage and robust error handling
- **Maintainability**: Clean, modular architecture with comprehensive documentation
- **Safety**: Memory safety and thread safety through Rust's ownership system

### Current Status

- **Compliance Level**: ~91% POSIX compliant
- **Test Coverage**: 413+ test functions across all components
- **Built-in Commands**: 22 implemented commands
- **Core Features**: Full variable expansion, arithmetic evaluation, control structures, functions with return
- **Architecture**: Modular design with separate lexer, parser, executor, and expansion engines

### Recently Implemented Features

- **Subshell Support**: Full POSIX-compliant subshells with state isolation, exit code propagation, trap inheritance, depth limit protection (max 100 levels), and 60+ test cases
- **File Descriptor Operations**: Complete FD table management, duplication (N>&M, N<&M), closing (N>&-, N<&-), read/write (N<>), with 30+ test cases
- **Here-documents**: Full implementation of `<<` and `<<<` (here-strings) with proper expansion handling
- **Enhanced Trap System**: Signal normalization, multiple handlers, trap display/reset, signal queue with overflow protection

## Architecture Overview

### Core Components

```text
src/
├── main.rs              # Entry point and REPL loop
├── lexer.rs             # Tokenization and alias expansion
├── parser.rs            # AST construction and parsing
├── executor.rs          # Command execution and evaluation
├── state.rs             # Shell state and variable management
├── arithmetic.rs        # Arithmetic expression evaluation
├── parameter_expansion.rs # Parameter expansion with modifiers
├── brace_expansion.rs   # Brace expansion engine
├── completion.rs        # Tab completion system
├── builtins/            # Built-in command implementations
│   ├── builtin_*.rs     # Individual builtin commands
└── lib.rs               # Library exports (if exists)
```

### Module Responsibilities

#### **Lexer** (`src/lexer.rs`)

- **Token Recognition**: Identifies commands, operators, keywords, and special tokens
- **Quote Handling**: Processes single/double quotes and escape sequences
- **Variable Detection**: Identifies variable patterns for deferred expansion
- **Alias Expansion**: Expands command aliases before parsing
- **Command Substitution**: Preserves `$(...)` and `` `...` `` syntax for runtime expansion
- **Here-document Tokenization**: Recognizes `<<` and `<<<` operators for here-documents and here-strings
- **FD Redirection Parsing**: Handles file descriptor redirection operators (N>&M, N<&M, N>&-, N<&-)

#### **Parser** (`src/parser.rs`)

- **AST Construction**: Builds Abstract Syntax Tree from token stream
- **Control Structures**: Handles if/elif/else, case, for, while, functions
- **Pipeline Construction**: Creates pipeline structures for command chaining
- **Redirection Parsing**: Processes I/O redirection operators
- **Subshell Parsing**: Parses subshell expressions with proper nesting
- **FD Redirection AST Nodes**: Constructs AST nodes for file descriptor operations

#### **Executor** (`src/executor.rs`)

- **Command Execution**: Runs external commands and built-in functions
- **Variable Expansion**: Runtime expansion of variables and parameters
- **Pipeline Management**: Coordinates data flow between pipeline stages
- **Redirection Handling**: Manages file descriptors and I/O redirection
- **Error Propagation**: Handles exit codes and error conditions
- **Subshell Execution**: Executes subshells with state isolation and trap inheritance
- **FD Table Management**: Manages file descriptor duplication, closing, and read/write operations
- **Here-document Processing**: Handles here-document and here-string expansion and execution

#### **State Management** (`src/state.rs`)

- **Variable Scoping**: Global and local variable management with proper scoping
- **Environment Integration**: Coordination with system environment variables
- **Function Context**: Function call stack and local variable scoping
- **Directory Stack**: pushd/popd/dirs functionality
- **Alias Management**: Command alias storage and expansion
- **FD Table**: File descriptor table with save/restore capabilities for subshells
- **Trap Management**: Signal trap handlers with inheritance and queue management

#### **Expansion Engines**

- **Arithmetic** (`src/arithmetic.rs`): `$((...))` evaluation using Shunting-yard algorithm
- **Parameter** (`src/parameter_expansion.rs`): `${VAR:-default}` and modifier processing
- **Brace** (`src/brace_expansion.rs`): `{a,b,c}` and range expansion (`{1..5}`)

## Code Quality Standards

### Testing Philosophy

The project maintains **comprehensive test coverage** with 413+ test functions:

```rust
// Example test structure
#[cfg(test)]
mod tests {
    #[test]
    fn test_feature_comprehensive_coverage() {
        // Arrange
        let mut shell_state = ShellState::new();
        shell_state.set_var("TEST_VAR", "test_value".to_string());

        // Act
        let tokens = lex("echo $TEST_VAR", &shell_state).unwrap();
        let result = expand_tokens(tokens, &mut shell_state);

        // Assert
        assert_eq!(result, vec![
            Token::Word("echo".to_string()),
            Token::Word("test_value".to_string())
        ]);
    }
}
```

### Test Synchronization Guidelines

**CRITICAL**: Tests that modify global state (environment variables, current directory, etc.) **MUST** use synchronization mutexes to prevent race conditions when running in parallel.

#### Available Test Mutexes

The test suite provides the following mutexes in `src/main.rs`:

```rust
// Mutex to serialize tests that change the current directory
static DIR_CHANGE_LOCK: Mutex<()> = Mutex::new(());

// Mutex to serialize tests that modify environment variables
static ENV_LOCK: Mutex<()> = Mutex::new(());
```

#### When to Use Test Mutexes

**You MUST use `ENV_LOCK`** when your test:

- Modifies environment variables using `std::env::set_var()` or `std::env::remove_var()`
- Reads environment variables that other tests might modify (e.g., `HOME`, `RUSH_CONDENSED`, `RUSH_COLORS`)
- Creates `ShellState` instances that depend on environment variables

**You MUST use `DIR_CHANGE_LOCK`** when your test:

- Changes the current working directory using `std::env::set_current_dir()`
- Depends on the current working directory being in a specific location

#### Proper Test Structure with Mutexes

```rust
#[test]
fn test_with_environment_modification() {
    // ALWAYS acquire the lock at the start of the test
    let _lock = ENV_LOCK.lock().unwrap();
    
    // Save original environment state
    let original_home = std::env::var("HOME").ok();
    let original_custom_var = std::env::var("CUSTOM_VAR").ok();
    
    // Perform test operations
    unsafe {
        std::env::set_var("HOME", "/tmp/test");
        std::env::set_var("CUSTOM_VAR", "test_value");
    }
    
    // ... test logic here ...
    
    // ALWAYS restore original environment state
    unsafe {
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
        
        if let Some(custom_var) = original_custom_var {
            std::env::set_var("CUSTOM_VAR", custom_var);
        } else {
            std::env::remove_var("CUSTOM_VAR");
        }
    }
    
    // Lock is automatically released when _lock goes out of scope
}
```

#### Common Pitfalls to Avoid

1. **Forgetting to acquire the lock**: This causes flaky tests that fail intermittently
2. **Not restoring environment state**: This can cause subsequent tests to fail
3. **Acquiring the lock too late**: The lock must be acquired before any environment reads/writes
4. **Not using unique temporary directories**: Always use timestamps or process IDs in temp paths

#### Example: Complete Test with Proper Synchronization

```rust
#[test]
fn test_source_rushrc_functionality() {
    // Lock to prevent parallel tests from interfering
    let _lock = ENV_LOCK.lock().unwrap();
    
    // Create unique temporary directory
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = format!("/tmp/rush_test_{}", timestamp);
    
    // Save original state
    let original_home = std::env::var("HOME").ok();
    
    // Create test files
    std::fs::create_dir_all(&temp_dir).unwrap();
    std::fs::write(
        format!("{}/.rushrc", temp_dir),
        "TEST_VAR=value\nexport TEST_VAR"
    ).unwrap();
    
    // Small delay to ensure filesystem consistency
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    // Modify environment
    unsafe {
        std::env::set_var("HOME", &temp_dir);
    }
    
    // Run test
    let mut shell_state = ShellState::new();
    source_rushrc(&mut shell_state);
    assert_eq!(shell_state.get_var("TEST_VAR"), Some("value".to_string()));
    
    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    // Restore environment
    unsafe {
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }
}
```

#### Adding New Test Mutexes

If you need to add a new mutex for a different type of global state:

1. Add it to the test module in `src/main.rs`:

```rust
static NEW_RESOURCE_LOCK: Mutex<()> = Mutex::new(());
```

1. Document it in this guide
2. Use it consistently in all tests that access that resource

### Error Handling Patterns

**Graceful Degradation**: The shell follows a "fail gracefully" philosophy:

```rust
// Example: Command substitution fallback
match execute_and_capture_output(ast, shell_state) {
    Ok(output) => output,
    Err(_) => {
        // Fall back to literal syntax preservation
        original_command.to_string()
    }
}
```

**Comprehensive Error Context**:

```rust
// Detailed error reporting with context
if shell_state.colors_enabled {
    eprintln!("{}Parse error: {}\x1b[0m",
              shell_state.color_scheme.error, error_msg);
} else {
    eprintln!("Parse error: {}", error_msg);
}
```

### Code Organization Principles

#### **Single Responsibility**

Each module has a clearly defined purpose:

- `lexer.rs`: Only handles tokenization and lexical analysis
- `arithmetic.rs`: Only handles mathematical expression evaluation
- `state.rs`: Only manages shell state and variables

#### **Immutable by Default**

```rust
// Functions prefer immutable references
pub fn get_var(&self, name: &str) -> Option<String> {
    // Implementation avoids mutation
}
```

#### **Explicit Error Propagation**

```rust
// Clear Result types for error handling
pub fn lex(input: &str, shell_state: &ShellState) -> Result<Vec<Token>, String>
```

## Development Guidelines

### Coding Standards

#### **Rust Best Practices**

- Use `clippy` linting with strict settings
- Prefer `&str` over `String` where possible
- Use `Cow<'_, str>` for efficient string handling
- Leverage Rust's type system for safety

#### **Documentation Requirements**

```rust
/// Comprehensive function documentation
///
/// # Examples
///
/// ```
/// use rush_sh::lexer;
/// let tokens = lexer::lex("echo hello", &shell_state).unwrap();
/// ```
///
/// # Errors
///
/// Returns `Err` if input cannot be tokenized
pub fn lex(input: &str, shell_state: &ShellState) -> Result<Vec<Token>, String>
```

### Contribution Workflow

#### **Before Making Changes**

1. **Understand the Architecture**: Review the module responsibilities above
2. **Check Test Coverage**: Ensure existing tests pass
3. **Identify Integration Points**: Understand how changes affect other modules
4. **Consider Error Cases**: Plan for comprehensive error handling

#### **Implementation Steps**

1. **Add Tests First**: Write failing tests for new functionality
2. **Implement Core Logic**: Follow existing patterns and error handling
3. **Update Documentation**: Maintain comprehensive documentation
4. **Run Full Test Suite**: Ensure no regressions

#### **Code Review Checklist**

- [ ] Comprehensive error handling implemented
- [ ] Tests cover both success and failure cases
- [ ] Documentation is complete and accurate
- [ ] Follows existing code patterns and style
- [ ] No performance regressions introduced
- [ ] Integration with existing modules verified

### Key Implementation Patterns

#### **Token Processing Pipeline**

```rust
// Standard flow for command processing
pub fn execute_line(line: &str, shell_state: &mut ShellState) {
    match lexer::lex(line, shell_state) {
        Ok(tokens) => {
            match lexer::expand_aliases(tokens, shell_state, &mut HashSet::new()) {
                Ok(expanded_tokens) => {
                    match brace_expansion::expand_braces(expanded_tokens) {
                        Ok(brace_expanded_tokens) => {
                            match parser::parse(brace_expanded_tokens) {
                                Ok(ast) => {
                                    let exit_code = executor::execute(ast, shell_state);
                                    // Handle execution results
                                }
                                Err(e) => { /* Handle parse errors */ }
                            }
                        }
                        Err(e) => { /* Handle brace expansion errors */ }
                    }
                }
                Err(e) => { /* Handle alias expansion errors */ }
            }
        }
        Err(e) => { /* Handle lex errors */ }
    }
}
```

#### **State Management**

```rust
// Proper variable scoping implementation
impl ShellState {
    /// Get variable with proper scoping rules
    pub fn get_var(&self, name: &str) -> Option<String> {
        // Check local scopes first (innermost to outermost)
        for scope in self.local_vars.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }

        // Fall back to global variables
        if let Some(value) = self.variables.get(name) {
            Some(value.clone())
        } else {
            // Finally check environment
            env::var(name).ok()
        }
    }
}
```

## Current Development Priorities

### High Priority (Core POSIX Features)

1. **Missing Built-ins**: `set`, `eval`, `exec`, `readonly`
2. **Job Control**: Background jobs (`&`), job management (`bg`, `fg`, `jobs`)

### Medium Priority

1. **Advanced Expansions**: Extended globbing patterns
2. **History Features**: History expansion (`!!`), improved line editing
3. **Performance Optimization**: Further optimize hot paths and memory usage

### Testing Priorities

- **Edge Case Coverage**: Ensure all error conditions are tested
- **Integration Testing**: End-to-end workflow validation
- **Performance Testing**: Verify no regressions in execution speed
- **Regression Testing**: Maintain comprehensive test coverage for all implemented features

## Performance Considerations

### **Memory Management**

- Reuse `String` instances where possible
- Avoid unnecessary allocations in hot paths
- Use `Rc<RefCell<_>>` for shared mutable state carefully

### **Execution Optimization**

- Built-in commands execute without process spawning
- Command substitution uses in-process execution for built-ins
- Efficient token processing with minimal copying

### **Caching Strategy**

- Alias expansion results are not cached (by design for freshness)
- Variable lookups are O(1) with HashMap implementation
- Function definitions stored efficiently in HashMap

## Debugging and Development Tools

### **Testing Commands**

```bash
# Run full test suite
cargo test

# Run specific test modules
cargo test lexer
cargo test parser
cargo test executor
cargo test builtins
cargo test state
cargo test completion
cargo test integration

# Run with output capture
cargo test -- --nocapture

# Benchmark tests
cargo test --release
```

### **Development Workflow**

```bash
# Build in debug mode
cargo build

# Build optimized version
cargo build --release

# Run interactive shell
cargo run

# Execute specific command
cargo run -- -c "echo hello"

# Run script
cargo run -- script.sh arg1 arg2
```

## Integration Points

### **Module Dependencies**

- `lexer` → `parser` → `executor` (primary flow)
- `state` ↔ All modules (shared state management)
- `expansion engines` → `executor` (runtime expansion)
- `builtins` → `executor` (command implementation)

### **External Dependencies**

- `rustyline`: Interactive line editing and history with signal handling support
- `signal-hook`: Robust signal handling (SIGINT, SIGTERM)
- `glob`: Pattern matching for case statements and wildcard expansion
- `clap`: Command-line argument parsing with derive macros
- `lazy_static`: Global state management in tab completion

This guide serves as a comprehensive reference for AI agents working on the Rush shell project. The modular architecture and extensive test coverage make it an excellent platform for learning systems programming in Rust while contributing to a real-world POSIX shell implementation.
