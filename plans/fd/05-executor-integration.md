# Executor Integration for FD Operations

## Overview

This document describes how the executor module will integrate with the FdManager to handle file descriptor operations during command execution.

## Current Executor Architecture

### Existing Redirection Handling

The current executor in `src/executor.rs` handles basic redirections:

```rust
// Current redirection handling (simplified)
pub fn execute(ast: AST, shell_state: &mut ShellState) -> i32 {
    match ast {
        AST::SimpleCommand { command, args, redirections } => {
            // Apply redirections
            for redirection in redirections {
                apply_redirection(&redirection)?;
            }
            // Execute command
            // Restore original FDs
        }
        // ... other AST variants
    }
}
```

### Limitations

1. **No FD Duplication**: Cannot duplicate FDs (e.g., `2>&1`)
2. **No FD Closures**: Cannot close FDs (e.g., `2>&-`)
3. **No FD Movement**: Cannot move FDs (e.g., `2>&1-`)
4. **No Here-Documents**: Limited support for here-documents
5. **No FD State Tracking**: No persistent FD state across commands

## Integration Strategy

### Phase 1: FdManager Integration

#### 1.1 Add FdManager to ShellState

```rust
// src/state.rs
use crate::fd_manager::FdManager;

pub struct ShellState {
    // ... existing fields ...
    fd_manager: FdManager,
}

impl ShellState {
    pub fn new() -> Self {
        Self {
            // ... existing initialization ...
            fd_manager: FdManager::new(),
        }
    }

    pub fn fd_manager(&self) -> &FdManager {
        &self.fd_manager
    }

    pub fn fd_manager_mut(&mut self) -> &mut FdManager {
        &mut self.fd_manager
    }
}
```

#### 1.2 Create FD Operation Context

```rust
// src/executor.rs
pub struct FdOperationContext {
    /// Original FD states for restoration
    original_states: HashMap<i32, FdState>,
    /// Applied operations
    operations: Vec<FdOperation>,
    /// Whether operations have been applied
    applied: bool,
}

impl FdOperationContext {
    pub fn new() -> Self {
        Self {
            original_states: HashMap::new(),
            operations: Vec::new(),
            applied: false,
        }
    }

    /// Add an operation to the context
    pub fn add_operation(&mut self, operation: FdOperation) {
        self.operations.push(operation);
    }

    /// Apply all operations in the context
    pub fn apply(&mut self, fd_manager: &mut FdManager) -> Result<(), String> {
        if self.applied {
            return Err("Operations already applied".to_string());
        }

        for operation in &self.operations {
            // Save original state before applying
            if let Some(fd) = operation.target_fd() {
                if let Ok(state) = fd_manager.get_fd_state(fd) {
                    self.original_states.insert(fd, state);
                }
            }

            fd_manager.apply_operation(operation.clone())?;
        }

        self.applied = true;
        Ok(())
    }

    /// Restore original FD states
    pub fn restore(&mut self, fd_manager: &mut FdManager) -> Result<(), String> {
        if !self.applied {
            return Ok(()); // Nothing to restore
        }

        for (fd, state) in &self.original_states {
            fd_manager.restore_fd_state(*fd, state.clone())?;
        }

        self.applied = false;
        self.original_states.clear();
        Ok(())
    }
}
```

### Phase 2: Command Execution with FD Operations

#### 2.1 Execute SimpleCommand with FD Operations

```rust
// src/executor.rs
pub fn execute_simple_command(
    command: String,
    args: Vec<String>,
    fd_operations: Vec<FdOperation>,
    shell_state: &mut ShellState,
) -> i32 {
    let mut fd_context = FdOperationContext::new();

    // Add all FD operations to context
    for operation in fd_operations {
        fd_context.add_operation(operation);
    }

    // Apply FD operations
    if let Err(e) = fd_context.apply(shell_state.fd_manager_mut()) {
        eprintln!("FD operation error: {}", e);
        return 1;
    }

    // Execute command
    let exit_code = match execute_command(&command, &args, shell_state) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Command execution error: {}", e);
            1
        }
    };

    // Restore original FD states
    if let Err(e) = fd_context.restore(shell_state.fd_manager_mut()) {
        eprintln!("FD restoration error: {}", e);
    }

    exit_code
}
```

#### 2.2 Execute Pipeline with FD Operations

```rust
// src/executor.rs
pub fn execute_pipeline(
    commands: Vec<PipelineCommand>,
    shell_state: &mut ShellState,
) -> i32 {
    let mut children = Vec::new();
    let mut prev_stdout: Option<RawFd> = None;

    for (i, cmd) in commands.into_iter().enumerate() {
        let is_last = i == commands.len() - 1;

        // Create pipe if not last command
        let (pipe_read, pipe_write) = if !is_last {
            match nix::unistd::pipe() {
                Ok((r, w)) => (Some(r), Some(w)),
                Err(e) => {
                    eprintln!("Pipe creation failed: {}", e);
                    return 1;
                }
            }
        } else {
            (None, None)
        };

        // Fork and execute
        match unsafe { nix::unistd::fork() } {
            Ok(nix::unistd::ForkResult::Parent { child }) => {
                children.push(child);

                // Close parent's pipe ends
                if let Some(read) = pipe_read {
                    let _ = nix::unistd::close(read);
                }
                if let Some(write) = pipe_write {
                    let _ = nix::unistd::close(write);
                }

                prev_stdout = pipe_write;
            }
            Ok(nix::unistd::ForkResult::Child) => {
                // Apply FD operations for this command
                let mut fd_context = FdOperationContext::new();
                for operation in cmd.fd_operations {
                    fd_context.add_operation(operation);
                }

                if let Err(e) = fd_context.apply(shell_state.fd_manager_mut()) {
                    eprintln!("FD operation error: {}", e);
                    std::process::exit(1);
                }

                // Set up pipe connections
                if let Some(read) = prev_stdout {
                    if let Err(e) = nix::unistd::dup2(read, 0) {
                        eprintln!("dup2 failed: {}", e);
                        std::process::exit(1);
                    }
                    let _ = nix::unistd::close(read);
                }

                if let Some(write) = pipe_write {
                    if let Err(e) = nix::unistd::dup2(write, 1) {
                        eprintln!("dup2 failed: {}", e);
                        std::process::exit(1);
                    }
                    let _ = nix::unistd::close(write);
                }

                // Close all pipe ends in child
                if let Some(read) = pipe_read {
                    let _ = nix::unistd::close(read);
                }

                // Execute command
                let exit_code = execute_simple_command(
                    cmd.command,
                    cmd.args,
                    Vec::new(), // FD operations already applied
                    shell_state,
                );

                std::process::exit(exit_code);
            }
            Err(e) => {
                eprintln!("Fork failed: {}", e);
                return 1;
            }
        }
    }

    // Wait for all children
    let mut last_exit_code = 0;
    for child in children {
        match nix::sys::wait::waitpid(child, None) {
            Ok(nix::sys::wait::WaitStatus::Exited(_, code)) => {
                last_exit_code = code;
            }
            Ok(nix::sys::wait::WaitStatus::Signaled(_, sig, _)) => {
                last_exit_code = 128 + sig as i32;
            }
            _ => {}
        }
    }

    last_exit_code
}
```

### Phase 3: Here-Document Support

#### 3.1 Here-Document Processing

```rust
// src/executor.rs
pub fn process_here_document(
    delimiter: String,
    content: String,
    fd: i32,
    shell_state: &mut ShellState,
) -> Result<(), String> {
    // Create temporary file for here-document
    let temp_file = tempfile::NamedTempFile::new()
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    // Write content to temp file
    std::fs::write(temp_file.path(), content)
        .map_err(|e| format!("Failed to write here-document: {}", e))?;

    // Open file for reading
    let file = std::fs::File::open(temp_file.path())
        .map_err(|e| format!("Failed to open here-document: {}", e))?;

    // Get raw file descriptor
    let raw_fd = file.into_raw_fd();

    // Use FdManager to redirect
    let operation = FdOperation::Redirect {
        target_fd: fd,
        source_fd: Some(raw_fd),
        path: None,
        mode: RedirectMode::Read,
    };

    shell_state.fd_manager_mut().apply_operation(operation)?;

    // Keep temp file alive (don't drop it)
    std::mem::forget(temp_file);

    Ok(())
}
```

#### 3.2 Here-String Processing

```rust
// src/executor.rs
pub fn process_here_string(
    content: String,
    fd: i32,
    shell_state: &mut ShellState,
) -> Result<(), String> {
    // Create pipe
    let (read_fd, write_fd) = nix::unistd::pipe()
        .map_err(|e| format!("Failed to create pipe: {}", e))?;

    // Write content to pipe
    use std::io::Write;
    let mut write_stream = unsafe { std::fs::File::from_raw_fd(write_fd) };
    write_stream.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write here-string: {}", e))?;
    write_stream.flush()
        .map_err(|e| format!("Failed to flush here-string: {}", e))?;

    // Redirect target FD to read end of pipe
    let operation = FdOperation::Redirect {
        target_fd: fd,
        source_fd: Some(read_fd),
        path: None,
        mode: RedirectMode::Read,
    };

    shell_state.fd_manager_mut().apply_operation(operation)?;

    Ok(())
}
```

### Phase 4: Subshell FD Operations

#### 4.1 Subshell FD Context

```rust
// src/executor.rs
pub fn execute_subshell(
    ast: AST,
    fd_operations: Vec<FdOperation>,
    shell_state: &mut ShellState,
) -> i32 {
    // Create FD context for subshell
    let mut fd_context = FdOperationContext::new();

    // Add all FD operations
    for operation in fd_operations {
        fd_context.add_operation(operation);
    }

    // Apply FD operations
    if let Err(e) = fd_context.apply(shell_state.fd_manager_mut()) {
        eprintln!("FD operation error: {}", e);
        return 1;
    }

    // Execute subshell AST
    let exit_code = execute(ast, shell_state);

    // Restore FD states
    if let Err(e) = fd_context.restore(shell_state.fd_manager_mut()) {
        eprintln!("FD restoration error: {}", e);
    }

    exit_code
}
```

#### 4.2 Command Substitution FD Handling

```rust
// src/executor.rs
pub fn execute_command_substitution(
    ast: AST,
    shell_state: &mut ShellState,
) -> Result<String, String> {
    // Create pipe for capturing output
    let (read_fd, write_fd) = nix::unistd::pipe()
        .map_err(|e| format!("Failed to create pipe: {}", e))?;

    match unsafe { nix::unistd::fork() } {
        Ok(nix::unistd::ForkResult::Parent { child }) => {
            // Close write end in parent
            let _ = nix::unistd::close(write_fd);

            // Read output from pipe
            let mut output = String::new();
            let mut reader = unsafe { std::fs::File::from_raw_fd(read_fd) };
            reader.read_to_string(&mut output)
                .map_err(|e| format!("Failed to read command substitution: {}", e))?;

            // Wait for child
            match nix::sys::wait::waitpid(child, None) {
                Ok(nix::sys::wait::WaitStatus::Exited(_, code)) => {
                    if code != 0 {
                        return Err(format!("Command substitution failed with exit code {}", code));
                    }
                }
                _ => {}
            }

            Ok(output)
        }
        Ok(nix::unistd::ForkResult::Child) => {
            // Redirect stdout to pipe
            if let Err(e) = nix::unistd::dup2(write_fd, 1) {
                eprintln!("dup2 failed: {}", e);
                std::process::exit(1);
            }
            let _ = nix::unistd::close(write_fd);
            let _ = nix::unistd::close(read_fd);

            // Execute command
            let exit_code = execute(ast, shell_state);
            std::process::exit(exit_code);
        }
        Err(e) => {
            let _ = nix::unistd::close(read_fd);
            let _ = nix::unistd::close(write_fd);
            Err(format!("Fork failed: {}", e))
        }
    }
}
```

## Error Handling

### FD Operation Errors

```rust
// src/executor.rs
pub enum FdExecutionError {
    /// Invalid file descriptor
    InvalidFd(i32),
    /// FD operation failed
    OperationFailed(String),
    /// FD restoration failed
    RestorationFailed(String),
    /// Here-document processing failed
    HereDocumentFailed(String),
    /// Pipe creation failed
    PipeCreationFailed(String),
}

impl std::fmt::Display for FdExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FdExecutionError::InvalidFd(fd) => write!(f, "Invalid file descriptor: {}", fd),
            FdExecutionError::OperationFailed(msg) => write!(f, "FD operation failed: {}", msg),
            FdExecutionError::RestorationFailed(msg) => write!(f, "FD restoration failed: {}", msg),
            FdExecutionError::HereDocumentFailed(msg) => write!(f, "Here-document failed: {}", msg),
            FdExecutionError::PipeCreationFailed(msg) => write!(f, "Pipe creation failed: {}", msg),
        }
    }
}

impl std::error::Error for FdExecutionError {}
```

### Error Recovery Strategy

```rust
// src/executor.rs
pub fn execute_with_fd_recovery(
    ast: AST,
    shell_state: &mut ShellState,
) -> i32 {
    let mut fd_context = FdOperationContext::new();

    // Extract FD operations from AST
    let fd_operations = extract_fd_operations(&ast);

    // Add operations to context
    for operation in fd_operations {
        fd_context.add_operation(operation);
    }

    // Apply FD operations with error recovery
    let apply_result = fd_context.apply(shell_state.fd_manager_mut());

    let exit_code = match apply_result {
        Ok(_) => {
            // Execute command
            execute(ast, shell_state)
        }
        Err(e) => {
            eprintln!("FD operation error: {}", e);
            1
        }
    };

    // Always attempt to restore FD states
    if let Err(e) = fd_context.restore(shell_state.fd_manager_mut()) {
        eprintln!("FD restoration error: {}", e);
    }

    exit_code
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fd_context_apply_and_restore() {
        let mut fd_manager = FdManager::new();
        let mut context = FdOperationContext::new();

        let operation = FdOperation::Redirect {
            target_fd: 2,
            source_fd: Some(1),
            path: None,
            mode: RedirectMode::Write,
        };

        context.add_operation(operation);
        context.apply(&mut fd_manager).unwrap();
        context.restore(&mut fd_manager).unwrap();
    }

    #[test]
    fn test_here_document_processing() {
        let mut shell_state = ShellState::new();
        let result = process_here_document(
            "EOF".to_string(),
            "test content\n".to_string(),
            0,
            &mut shell_state,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_here_string_processing() {
        let mut shell_state = ShellState::new();
        let result = process_here_string(
            "test string".to_string(),
            0,
            &mut shell_state,
        );
        assert!(result.is_ok());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_command_with_fd_duplication() {
    let mut shell_state = ShellState::new();
    let ast = parse("echo error 2>&1").unwrap();
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_command_with_fd_closure() {
    let mut shell_state = ShellState::new();
    let ast = parse("echo output 2>&-").unwrap();
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_pipeline_with_fd_operations() {
    let mut shell_state = ShellState::new();
    let ast = parse("cat file.txt 2>&1 | grep pattern").unwrap();
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}
```

## Performance Considerations

### FD Operation Optimization

1. **Batch Operations**: Apply all FD operations in a single context
2. **Lazy Restoration**: Only restore FDs that were actually modified
3. **State Caching**: Cache FD states to avoid redundant syscalls
4. **Minimal Forking**: Reduce fork overhead in pipelines

### Memory Management

1. **RAII Pattern**: Use RAII for automatic FD cleanup
2. **Smart Pointers**: Use `Arc` for shared FD state
3. **Buffer Reuse**: Reuse buffers for here-document processing

## Migration Path

### Backward Compatibility

1. **Gradual Migration**: Introduce FdManager alongside existing code
2. **Feature Flags**: Use feature flags to enable new FD operations
3. **Deprecation Warnings**: Warn about deprecated redirection syntax

### Rollout Strategy

1. **Phase 1**: Add FdManager to ShellState (no behavior change)
2. **Phase 2**: Implement FD operation context (no behavior change)
3. **Phase 3**: Enable FD duplication (2>&1)
4. **Phase 4**: Enable FD closure (2>&-)
5. **Phase 5**: Enable FD movement (2>&1-)
6. **Phase 6**: Enable here-documents
7. **Phase 7**: Enable here-strings

## Summary

The executor integration for FD operations involves:

1. **FdManager Integration**: Add FdManager to ShellState
2. **FD Operation Context**: Create context for applying and restoring FD operations
3. **Command Execution**: Execute commands with FD operations
4. **Pipeline Support**: Handle FD operations in pipelines
5. **Here-Document Support**: Process here-documents and here-strings
6. **Subshell Support**: Handle FD operations in subshells
7. **Error Handling**: Comprehensive error handling and recovery
8. **Testing**: Unit and integration tests for all FD operations

This integration provides a robust foundation for implementing POSIX-compliant file descriptor operations in the Rush shell.
