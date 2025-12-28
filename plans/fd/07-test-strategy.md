# Test Strategy for File Descriptor Operations

## Overview

This document outlines the comprehensive testing strategy for file descriptor operations in the Rush shell. The strategy covers unit tests, integration tests, edge cases, and POSIX compliance validation.

## Testing Philosophy

Following the Rush shell's testing philosophy:

- **Comprehensive Coverage**: Test all success and failure paths
- **Synchronization**: Use mutexes for tests modifying global state
- **Isolation**: Each test should be independent and reproducible
- **Clarity**: Tests should be self-documenting with clear assertions

## Test Organization

### Directory Structure

```
src/
├── fd_manager.rs              # FdManager implementation
└── tests/
    ├── fd_manager_tests.rs    # FdManager unit tests
    ├── fd_lexer_tests.rs      # Lexer FD operation tests
    ├── fd_parser_tests.rs     # Parser FD operation tests
    ├── fd_executor_tests.rs   # Executor FD operation tests
    └── fd_integration_tests.rs # End-to-end FD operation tests
```

## Unit Tests

### FdManager Tests (`fd_manager_tests.rs`)

#### Test Categories

1. **FD Allocation Tests**
   - Allocate new FDs
   - Allocate specific FDs
   - Handle FD conflicts
   - FD reuse after closure

2. **FD Duplication Tests**
   - Duplicate to available FD
   - Duplicate to occupied FD (with close)
   - Duplicate to invalid FD
   - Preserve original FD after duplication

3. **FD Closure Tests**
   - Close valid FDs
   - Close invalid FDs (error handling)
   - Close all FDs
   - Close FDs in specific range

4. **FD Query Tests**
   - Check if FD is open
   - Get FD metadata
   - List all open FDs
   - Get FD count

5. **FD Redirection Tests**
   - Redirect to file
   - Redirect to another FD
   - Redirect to /dev/null
   - Redirect with append mode

6. **Error Handling Tests**
   - Invalid FD numbers
   - Permission errors
   - File not found
   - Too many open files

#### Example Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    #[test]
    fn test_fd_allocation() {
        let mut manager = FdManager::new();
        
        // Allocate a new FD
        let fd = manager.allocate_fd(None).unwrap();
        assert!(fd >= 3); // Should be >= 3 (0,1,2 are stdin/stdout/stderr)
        
        // Verify FD is tracked
        assert!(manager.is_open(fd));
    }

    #[test]
    fn test_fd_duplication() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let mut manager = FdManager::new();
        let source_fd = 1; // stdout
        
        // Duplicate stdout to FD 3
        let new_fd = manager.dup_fd(source_fd, Some(3)).unwrap();
        assert_eq!(new_fd, 3);
        assert!(manager.is_open(3));
    }

    #[test]
    fn test_fd_closure() {
        let mut manager = FdManager::new();
        let fd = manager.allocate_fd(None).unwrap();
        
        // Close the FD
        manager.close_fd(fd).unwrap();
        assert!(!manager.is_open(fd));
    }

    #[test]
    fn test_redirect_to_file() {
        let _lock = DIR_CHANGE_LOCK.lock().unwrap();
        
        let mut manager = FdManager::new();
        let temp_file = "/tmp/rush_test_redirect.txt";
        
        // Create temp file
        let mut file = File::create(temp_file).unwrap();
        writeln!(file, "test content").unwrap();
        drop(file);
        
        // Redirect FD 3 to file
        manager.redirect_fd(3, temp_file, false).unwrap();
        
        // Verify redirection
        assert!(manager.is_open(3));
        
        // Cleanup
        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_invalid_fd_error() {
        let mut manager = FdManager::new();
        
        // Try to close invalid FD
        let result = manager.close_fd(9999);
        assert!(result.is_err());
    }
}
```

### Lexer Tests (`fd_lexer_tests.rs`)

#### Test Categories

1. **FD Number Tokenization**
   - Simple FD numbers: `0`, `1`, `2`, `3`
   - FD numbers before operators: `2>`, `1<`, `3>&1`
   - FD numbers in complex expressions

2. **FD Operation Operators**
   - `>` (output redirection)
   - `<` (input redirection)
   - `>>` (append redirection)
   - `>&` (FD duplication)
   - `<&` (FD duplication input)
   - `>&-` (close FD)
   - `<&-` (close FD input)

3. **Combined FD Operations**
   - Multiple FD operations in one command
   - FD operations with pipes
   - FD operations with subshells

#### Example Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;

    #[test]
    fn test_fd_number_tokenization() {
        let shell_state = ShellState::new();
        let tokens = lexer::lex("echo test 2>file.txt", &shell_state).unwrap();
        
        // Should contain FD number token
        assert!(tokens.iter().any(|t| matches!(t, Token::FdNumber(2))));
    }

    #[test]
    fn test_fd_duplication_operator() {
        let shell_state = ShellState::new();
        let tokens = lexer::lex("cmd 2>&1", &shell_state).unwrap();
        
        // Should contain FD duplication operator
        assert!(tokens.iter().any(|t| matches!(t, Token::FdDupOut)));
    }

    #[test]
    fn test_fd_close_operator() {
        let shell_state = ShellState::new();
        let tokens = lexer::lex("cmd 2>&-", &shell_state).unwrap();
        
        // Should contain FD close operator
        assert!(tokens.iter().any(|t| matches!(t, Token::FdClose)));
    }

    #[test]
    fn test_multiple_fd_operations() {
        let shell_state = ShellState::new();
        let tokens = lexer::lex("cmd <input.txt 2>err.txt >out.txt", &shell_state).unwrap();
        
        // Should contain multiple FD operations
        let fd_ops: Vec<_> = tokens.iter()
            .filter(|t| matches!(t, Token::FdNumber(_) | Token::RedirectOut | Token::RedirectIn))
            .collect();
        assert!(fd_ops.len() >= 3);
    }
}
```

### Parser Tests (`fd_parser_tests.rs`)

#### Test Categories

1. **FD Redirection Parsing**
   - Simple redirections: `2>file`
   - Append redirections: `2>>file`
   - Input redirections: `0<file`

2. **FD Duplication Parsing**
   - Output duplication: `2>&1`
   - Input duplication: `0<&3`
   - Close operations: `2>&-`

3. **Complex FD Operations**
   - Multiple FD operations
   - FD operations with pipelines
   - FD operations with subshells
   - FD operations with command groups

#### Example Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use crate::parser;

    #[test]
    fn test_parse_fd_redirection() {
        let shell_state = ShellState::new();
        let tokens = lexer::lex("echo test 2>err.txt", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        
        // Should contain FD redirection node
        assert!(matches!(ast, AstNode::Command(_)));
    }

    #[test]
    fn test_parse_fd_duplication() {
        let shell_state = ShellState::new();
        let tokens = lexer::lex("cmd 2>&1", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        
        // Should contain FD duplication node
        assert!(matches!(ast, AstNode::Command(_)));
    }

    #[test]
    fn test_parse_multiple_fd_operations() {
        let shell_state = ShellState::new();
        let tokens = lexer::lex("cmd <in.txt 2>err.txt >out.txt", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        
        // Should contain multiple FD operations
        assert!(matches!(ast, AstNode::Command(_)));
    }
}
```

### Executor Tests (`fd_executor_tests.rs`)

#### Test Categories

1. **FD Redirection Execution**
   - Redirect to file
   - Redirect with append
   - Redirect to /dev/null
   - Redirect to pipe

2. **FD Duplication Execution**
   - Duplicate FD
   - Duplicate with close
   - Preserve original FD

3. **FD Closure Execution**
   - Close specific FD
   - Close multiple FDs
   - Close all FDs

4. **Error Handling**
   - Invalid FD operations
   - Permission errors
   - File not found

#### Example Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use crate::parser;
    use crate::executor;
    use crate::state::ShellState;

    #[test]
    fn test_execute_fd_redirection() {
        let _lock = DIR_CHANGE_LOCK.lock().unwrap();
        
        let mut shell_state = ShellState::new();
        let temp_file = "/tmp/rush_test_out.txt";
        
        // Execute command with FD redirection
        let tokens = lexer::lex(&format!("echo test > {}", temp_file), &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        executor::execute(ast, &mut shell_state);
        
        // Verify file was created with content
        let content = std::fs::read_to_string(temp_file).unwrap();
        assert!(content.contains("test"));
        
        // Cleanup
        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_execute_fd_duplication() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let mut shell_state = ShellState::new();
        
        // Execute command with FD duplication
        let tokens = lexer::lex("echo test 2>&1", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        
        // Should succeed
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_fd_closure() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let mut shell_state = ShellState::new();
        
        // Execute command with FD closure
        let tokens = lexer::lex("cmd 2>&-", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        
        // Should succeed (stderr closed)
        assert_eq!(exit_code, 0);
    }
}
```

## Integration Tests

### End-to-End Tests (`fd_integration_tests.rs`)

#### Test Categories

1. **Standard FD Operations**
   - Redirect stdin: `cmd < input.txt`
   - Redirect stdout: `cmd > output.txt`
   - Redirect stderr: `cmd 2> error.txt`
   - Redirect both: `cmd > output.txt 2>&1`

2. **FD Duplication**
   - Duplicate stdout: `cmd 2>&1`
   - Duplicate stdin: `cmd 0<&3`
   - Chain duplications: `cmd 3>&2 2>&1`

3. **FD Closure**
   - Close stdout: `cmd 1>&-`
   - Close stderr: `cmd 2>&-`
   - Close custom FD: `cmd 3>&-`

4. **Complex Scenarios**
   - Multiple FD operations: `cmd <in.txt >out.txt 2>err.txt`
   - FD operations with pipes: `cmd1 2>&1 | cmd2`
   - FD operations with subshells: `(cmd 2>&1) | cmd2`
   - FD operations with command groups: `{ cmd 2>&1; } | cmd2`

5. **Here-Documents**
   - Basic here-doc: `cmd << EOF`
   - Here-doc with FD: `cmd 3<< EOF`
   - Quoted here-doc: `cmd << 'EOF'`

6. **Here-Strings**
   - Basic here-string: `cmd <<< "text"`
   - Here-string with FD: `cmd 3<<< "text"`

#### Example Integration Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use crate::parser;
    use crate::executor;
    use crate::state::ShellState;

    #[test]
    fn test_redirect_stdout_to_file() {
        let _lock = DIR_CHANGE_LOCK.lock().unwrap();
        
        let mut shell_state = ShellState::new();
        let temp_file = "/tmp/rush_test_stdout.txt";
        
        // Execute command with stdout redirection
        let tokens = lexer::lex(&format!("echo 'hello world' > {}", temp_file), &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        executor::execute(ast, &mut shell_state);
        
        // Verify output
        let content = std::fs::read_to_string(temp_file).unwrap();
        assert_eq!(content.trim(), "hello world");
        
        // Cleanup
        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_redirect_stderr_to_file() {
        let _lock = DIR_CHANGE_LOCK.lock().unwrap();
        
        let mut shell_state = ShellState::new();
        let temp_file = "/tmp/rush_test_stderr.txt";
        
        // Execute command that writes to stderr
        let tokens = lexer::lex(&format!("sh -c 'echo error >&2' 2> {}", temp_file), &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        executor::execute(ast, &mut shell_state);
        
        // Verify error was captured
        let content = std::fs::read_to_string(temp_file).unwrap();
        assert!(content.contains("error"));
        
        // Cleanup
        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_redirect_stdout_and_stderr_to_same_file() {
        let _lock = DIR_CHANGE_LOCK.lock().unwrap();
        
        let mut shell_state = ShellState::new();
        let temp_file = "/tmp/rush_test_both.txt";
        
        // Execute command with both stdout and stderr redirected
        let tokens = lexer::lex(&format!("sh -c 'echo out; echo err >&2' > {} 2>&1", temp_file), &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        executor::execute(ast, &mut shell_state);
        
        // Verify both outputs
        let content = std::fs::read_to_string(temp_file).unwrap();
        assert!(content.contains("out"));
        assert!(content.contains("err"));
        
        // Cleanup
        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_fd_duplication_stderr_to_stdout() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let mut shell_state = ShellState::new();
        
        // Execute command with stderr duplicated to stdout
        let tokens = lexer::lex("sh -c 'echo err >&2' 2>&1", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        
        // Should succeed
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_close_stderr() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let mut shell_state = ShellState::new();
        
        // Execute command with stderr closed
        let tokens = lexer::lex("sh -c 'echo err >&2' 2>&-", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        
        // Should succeed (stderr closed)
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_multiple_fd_operations() {
        let _lock = DIR_CHANGE_LOCK.lock().unwrap();
        
        let mut shell_state = ShellState::new();
        let in_file = "/tmp/rush_test_in.txt";
        let out_file = "/tmp/rush_test_out.txt";
        let err_file = "/tmp/rush_test_err.txt";
        
        // Create input file
        std::fs::write(in_file, "test input").unwrap();
        
        // Execute command with multiple FD operations
        let tokens = lexer::lex(
            &format!("cat < {} > {} 2> {}", in_file, out_file, err_file),
            &shell_state
        ).unwrap();
        let ast = parser::parse(tokens).unwrap();
        executor::execute(ast, &mut shell_state);
        
        // Verify output
        let content = std::fs::read_to_string(out_file).unwrap();
        assert_eq!(content.trim(), "test input");
        
        // Cleanup
        let _ = std::fs::remove_file(in_file);
        let _ = std::fs::remove_file(out_file);
        let _ = std::fs::remove_file(err_file);
    }

    #[test]
    fn test_fd_operations_with_pipe() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let mut shell_state = ShellState::new();
        
        // Execute command with FD operations and pipe
        let tokens = lexer::lex("sh -c 'echo out; echo err >&2' 2>&1 | cat", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        
        // Should succeed
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_fd_operations_with_subshell() {
        let _lock = ENV_LOCK.lock().unwrap();
        
        let mut shell_state = ShellState::new();
        
        // Execute command with FD operations in subshell
        let tokens = lexer::lex("(sh -c 'echo out; echo err >&2' 2>&1) | cat", &shell_state).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let exit_code = executor::execute(ast, &mut shell_state);
        
        // Should succeed
        assert_eq!(exit_code, 0);
    }
}
```

## Edge Cases and Error Handling

### Test Categories

1. **Invalid FD Numbers**
   - Negative FD numbers
   - FD numbers beyond system limits
   - Non-numeric FD specifications

2. **Permission Errors**
   - Redirect to read-only file
   - Redirect from non-existent file
   - Redirect to directory

3. **Resource Limits**
   - Too many open files
   - FD exhaustion
   - System FD limits

4. **Concurrent Operations**
   - Multiple commands accessing same FD
   - FD operations in pipelines
   - FD operations in background jobs

5. **Special Cases**
   - Redirecting to/from /dev/null
   - Redirecting to/from /dev/full
   - Redirecting to/from special devices

#### Example Edge Case Tests

```rust
#[test]
fn test_invalid_fd_number() {
    let mut shell_state = ShellState::new();
    
    // Try to use invalid FD number
    let tokens = lexer::lex("cmd 999>file.txt", &shell_state).unwrap();
    let ast = parser::parse(tokens).unwrap();
    let exit_code = executor::execute(ast, &mut shell_state);
    
    // Should fail with appropriate error
    assert!(exit_code != 0);
}

#[test]
fn test_redirect_to_readonly_file() {
    let _lock = DIR_CHANGE_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();
    let temp_file = "/tmp/rush_test_readonly.txt";
    
    // Create read-only file
    std::fs::write(temp_file, "readonly").unwrap();
    let mut perms = std::fs::metadata(temp_file).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(temp_file, perms).unwrap();
    
    // Try to redirect to read-only file
    let tokens = lexer::lex(&format!("echo test > {}", temp_file), &shell_state).unwrap();
    let ast = parser::parse(tokens).unwrap();
    let exit_code = executor::execute(ast, &mut shell_state);
    
    // Should fail
    assert!(exit_code != 0);
    
    // Cleanup
    perms.set_readonly(false);
    std::fs::set_permissions(temp_file, perms).unwrap();
    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_redirect_from_nonexistent_file() {
    let _lock = DIR_CHANGE_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();
    let nonexistent_file = "/tmp/rush_test_nonexistent_12345.txt";
    
    // Try to redirect from non-existent file
    let tokens = lexer::lex(&format!("cat < {}", nonexistent_file), &shell_state).unwrap();
    let ast = parser::parse(tokens).unwrap();
    let exit_code = executor::execute(ast, &mut shell_state);
    
    // Should fail
    assert!(exit_code != 0);
}
```

## POSIX Compliance Tests

### Test Categories

1. **Standard Redirections**
   - Verify behavior matches POSIX specification
   - Test all standard redirection operators
   - Verify error handling

2. **FD Operations**
   - Verify FD duplication behavior
   - Test FD closure semantics
   - Verify FD numbering rules

3. **Here-Documents**
   - Verify here-document parsing
   - Test here-document with FD
   - Verify quoted here-documents

4. **Here-Strings**
   - Verify here-string behavior
   - Test here-string with FD
   - Verify escaping rules

#### Example POSIX Compliance Tests

```rust
#[test]
fn test_posix_stdout_redirection() {
    // POSIX: Redirection of output shall cause the file to be opened
    // for writing on file descriptor 1
    let _lock = DIR_CHANGE_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();
    let temp_file = "/tmp/rush_test_posix.txt";
    
    let tokens = lexer::lex(&format!("echo test > {}", temp_file), &shell_state).unwrap();
    let ast = parser::parse(tokens).unwrap();
    executor::execute(ast, &mut shell_state);
    
    let content = std::fs::read_to_string(temp_file).unwrap();
    assert_eq!(content.trim(), "test");
    
    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_posix_stderr_duplication() {
    // POSIX: [n]>&word shall duplicate standard output to file descriptor n
    let _lock = ENV_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();
    
    let tokens = lexer::lex("echo test 2>&1", &shell_state).unwrap();
    let ast = parser::parse(tokens).unwrap();
    let exit_code = executor::execute(ast, &mut shell_state);
    
    assert_eq!(exit_code, 0);
}

#[test]
fn test_posix_fd_closure() {
    // POSIX: [n]>&- shall close file descriptor n
    let _lock = ENV_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();
    
    let tokens = lexer::lex("echo test 2>&-", &shell_state).unwrap();
    let ast = parser::parse(tokens).unwrap();
    let exit_code = executor::execute(ast, &mut shell_state);
    
    assert_eq!(exit_code, 0);
}
```

## Performance Tests

### Test Categories

1. **FD Allocation Performance**
   - Benchmark FD allocation speed
   - Test FD reuse performance
   - Measure memory usage

2. **FD Operation Performance**
   - Benchmark redirection speed
   - Test duplication performance
   - Measure closure overhead

3. **Concurrent Operations**
   - Test multiple simultaneous FD operations
   - Measure contention
   - Verify thread safety

#### Example Performance Tests

```rust
#[test]
#[ignore] // Run with: cargo test --release -- --ignored
fn benchmark_fd_allocation() {
    let mut manager = FdManager::new();
    let start = std::time::Instant::now();
    
    for _ in 0..1000 {
        let fd = manager.allocate_fd(None).unwrap();
        manager.close_fd(fd).unwrap();
    }
    
    let duration = start.elapsed();
    println!("FD allocation/closure 1000 iterations: {:?}", duration);
    assert!(duration.as_millis() < 100); // Should be fast
}
```

## Test Synchronization

### Mutex Usage

All tests that modify global state MUST use appropriate mutexes:

```rust
#[test]
fn test_with_environment_modification() {
    let _lock = ENV_LOCK.lock().unwrap();
    
    // Save original state
    let original_var = std::env::var("TEST_VAR").ok();
    
    // Modify environment
    unsafe {
        std::env::set_var("TEST_VAR", "test_value");
    }
    
    // ... test logic ...
    
    // Restore environment
    unsafe {
        if let Some(var) = original_var {
            std::env::set_var("TEST_VAR", var);
        } else {
            std::env::remove_var("TEST_VAR");
        }
    }
}

#[test]
fn test_with_directory_change() {
    let _lock = DIR_CHANGE_LOCK.lock().unwrap();
    
    // Save original directory
    let original_dir = std::env::current_dir().unwrap();
    
    // Change directory
    std::env::set_current_dir("/tmp").unwrap();
    
    // ... test logic ...
    
    // Restore directory
    std::env::set_current_dir(original_dir).unwrap();
}
```

## Test Coverage Goals

### Target Coverage

- **FdManager**: 95%+ code coverage
- **Lexer FD operations**: 90%+ code coverage
- **Parser FD operations**: 90%+ code coverage
- **Executor FD operations**: 85%+ code coverage
- **Integration tests**: All major use cases

### Coverage Metrics

```bash
# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage/

# View coverage
open coverage/index.html
```

## Test Execution

### Running Tests

```bash
# Run all FD operation tests
cargo test fd

# Run specific test module
cargo test fd_manager_tests
cargo test fd_lexer_tests
cargo test fd_parser_tests
cargo test fd_executor_tests
cargo test fd_integration_tests

# Run with output
cargo test fd -- --nocapture

# Run ignored (performance) tests
cargo test fd -- --ignored --release

# Run with specific filter
cargo test fd -- test_redirect_stdout
```

### Continuous Integration

All tests should pass in CI before merging:

```yaml
# Example CI configuration
test:
  script:
    - cargo test --all-features
    - cargo test --all-features --release
    - cargo clippy --all-features -- -D warnings
```

## Summary

This test strategy provides comprehensive coverage for file descriptor operations in the Rush shell:

- **Unit tests** for each component (FdManager, lexer, parser, executor)
- **Integration tests** for end-to-end scenarios
- **Edge case tests** for error handling
- **POSIX compliance tests** for specification adherence
- **Performance tests** for optimization validation
- **Proper synchronization** for tests modifying global state

Following this strategy will ensure robust, reliable, and POSIX-compliant file descriptor operations in the Rush shell.
