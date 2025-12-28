# State Management Integration for FD Operations

## Overview

This document describes how the FdManager will be integrated into the ShellState module to provide persistent file descriptor state management across the shell's lifetime.

## Current ShellState Architecture

### Existing State Management

The current `src/state.rs` manages:

```rust
pub struct ShellState {
    /// Global variables
    pub variables: HashMap<String, String>,
    /// Local variable scopes (for functions)
    pub local_vars: Vec<HashMap<String, String>>,
    /// Exported variables
    pub exported_vars: HashSet<String>,
    /// Function definitions
    pub functions: HashMap<String, AST>,
    /// Aliases
    pub aliases: HashMap<String, String>,
    /// Directory stack for pushd/popd
    pub dir_stack: Vec<PathBuf>,
    /// Current working directory
    pub current_dir: PathBuf,
    /// Previous working directory
    pub previous_dir: PathBuf,
    /// Exit status of last command
    pub last_exit_status: i32,
    /// Background jobs
    pub jobs: Vec<Job>,
    /// Color scheme
    pub color_scheme: ColorScheme,
    /// Whether colors are enabled
    pub colors_enabled: bool,
    /// Whether condensed mode is enabled
    pub condensed_mode: bool,
}
```

### State Lifecycle

1. **Initialization**: Created when shell starts
2. **Modification**: Updated during command execution
3. **Persistence**: Maintained across commands
4. **Cleanup**: Destroyed when shell exits

## FdManager Integration Design

### Phase 1: Add FdManager to ShellState

#### 1.1 Update ShellState Structure

```rust
// src/state.rs
use crate::fd_manager::FdManager;

pub struct ShellState {
    // ... existing fields ...
    /// File descriptor manager
    fd_manager: FdManager,
}
```

#### 1.2 Initialize FdManager

```rust
// src/state.rs
impl ShellState {
    pub fn new() -> Self {
        Self {
            // ... existing initialization ...
            fd_manager: FdManager::new(),
        }
    }

    pub fn with_fd_manager(fd_manager: FdManager) -> Self {
        Self {
            // ... existing initialization ...
            fd_manager,
        }
    }
}
```

#### 1.3 Provide Accessor Methods

```rust
// src/state.rs
impl ShellState {
    /// Get reference to FdManager
    pub fn fd_manager(&self) -> &FdManager {
        &self.fd_manager
    }

    /// Get mutable reference to FdManager
    pub fn fd_manager_mut(&mut self) -> &mut FdManager {
        &mut self.fd_manager
    }

    /// Clone FD state (for subshells)
    pub fn clone_fd_state(&self) -> FdManager {
        self.fd_manager.clone()
    }

    /// Restore FD state (for subshells)
    pub fn restore_fd_state(&mut self, fd_manager: FdManager) {
        self.fd_manager = fd_manager;
    }
}
```

### Phase 2: FD State Persistence

#### 2.1 FD State Snapshot

```rust
// src/state.rs
impl ShellState {
    /// Create a snapshot of current FD state
    pub fn snapshot_fd_state(&self) -> FdStateSnapshot {
        FdStateSnapshot {
            fd_states: self.fd_manager.get_all_fd_states(),
            next_fd: self.fd_manager.get_next_fd(),
        }
    }

    /// Restore FD state from snapshot
    pub fn restore_fd_snapshot(&mut self, snapshot: FdStateSnapshot) -> Result<(), String> {
        self.fd_manager.restore_all_fd_states(snapshot.fd_states)?;
        self.fd_manager.set_next_fd(snapshot.next_fd);
        Ok(())
    }
}

/// Snapshot of FD state
#[derive(Clone, Debug)]
pub struct FdStateSnapshot {
    /// All FD states
    pub fd_states: HashMap<i32, FdState>,
    /// Next available FD
    pub next_fd: i32,
}
```

#### 2.2 FD State for Subshells

```rust
// src/state.rs
impl ShellState {
    /// Create a copy of ShellState for subshell execution
    pub fn create_subshell_state(&self) -> Self {
        Self {
            // Copy most fields
            variables: self.variables.clone(),
            local_vars: self.local_vars.clone(),
            exported_vars: self.exported_vars.clone(),
            functions: self.functions.clone(),
            aliases: self.aliases.clone(),
            dir_stack: self.dir_stack.clone(),
            current_dir: self.current_dir.clone(),
            previous_dir: self.previous_dir.clone(),
            last_exit_status: self.last_exit_status,
            jobs: Vec::new(), // Jobs are not inherited
            color_scheme: self.color_scheme.clone(),
            colors_enabled: self.colors_enabled,
            condensed_mode: self.condensed_mode,
            // Clone FD state
            fd_manager: self.fd_manager.clone(),
        }
    }

    /// Merge subshell state back into parent
    pub fn merge_subshell_state(&mut self, subshell_state: ShellState) {
        // Update exit status
        self.last_exit_status = subshell_state.last_exit_status;

        // Note: FD state is NOT merged - subshell FD changes are isolated
        // This is POSIX-compliant behavior
    }
}
```

### Phase 3: FD State and Variables

#### 3.1 FD-Related Variables

```rust
// src/state.rs
impl ShellState {
    /// Get FD-related variable
    pub fn get_fd_variable(&self, name: &str) -> Option<String> {
        match name {
            // Standard FD variables
            "0" | "1" | "2" => {
                let fd = name.parse::<i32>().ok()?;
                self.fd_manager.get_fd_path(fd)
            }
            // Custom FD variables
            _ if name.starts_with("FD_") => {
                let fd_str = name.strip_prefix("FD_")?;
                let fd = fd_str.parse::<i32>().ok()?;
                self.fd_manager.get_fd_path(fd)
            }
            _ => None,
        }
    }

    /// Set FD-related variable (read-only)
    pub fn set_fd_variable(&mut self, name: &str, value: String) -> Result<(), String> {
        match name {
            "0" | "1" | "2" => {
                Err(format!("Cannot set read-only FD variable: {}", name))
            }
            _ if name.starts_with("FD_") => {
                Err(format!("Cannot set read-only FD variable: {}", name))
            }
            _ => Ok(()),
        }
    }
}
```

#### 3.2 FD State Export

```rust
// src/state.rs
impl ShellState {
    /// Export FD state to environment
    pub fn export_fd_state(&mut self) {
        // Export standard FD paths
        for fd in 0..=2 {
            if let Some(path) = self.fd_manager.get_fd_path(fd) {
                let var_name = format!("FD_{}", fd);
                self.set_var(&var_name, path);
                self.exported_vars.insert(var_name);
            }
        }
    }

    /// Import FD state from environment
    pub fn import_fd_state(&mut self) {
        for fd in 0..=2 {
            let var_name = format!("FD_{}", fd);
            if let Some(path) = self.get_var(&var_name) {
                // Note: This is informational only
                // Actual FD state is managed by FdManager
            }
        }
    }
}
```

### Phase 4: FD State and Jobs

#### 4.1 Job FD State

```rust
// src/state.rs
#[derive(Clone, Debug)]
pub struct Job {
    pub job_id: usize,
    pub pgid: Option<nix::unistd::Pid>,
    pub command: String,
    pub processes: Vec<Process>,
    pub state: JobState,
    pub fd_snapshot: Option<FdStateSnapshot>,
}

#[derive(Clone, Debug)]
pub struct Process {
    pub pid: nix::unistd::Pid,
    pub command: String,
    pub fd_snapshot: Option<FdStateSnapshot>,
}

impl ShellState {
    /// Add job with FD state snapshot
    pub fn add_job_with_fd_state(
        &mut self,
        command: String,
        processes: Vec<Process>,
    ) -> usize {
        let job_id = self.jobs.len() + 1;
        let fd_snapshot = self.snapshot_fd_state();

        let job = Job {
            job_id,
            pgid: None,
            command,
            processes,
            state: JobState::Running,
            fd_snapshot: Some(fd_snapshot),
        };

        self.jobs.push(job);
        job_id
    }

    /// Restore FD state for job
    pub fn restore_job_fd_state(&mut self, job_id: usize) -> Result<(), String> {
        let job = self.jobs.iter()
            .find(|j| j.job_id == job_id)
            .ok_or_else(|| format!("Job {} not found", job_id))?;

        if let Some(ref snapshot) = job.fd_snapshot {
            self.restore_fd_snapshot(snapshot.clone())?;
        }

        Ok(())
    }
}
```

#### 4.2 Background Job FD Handling

```rust
// src/state.rs
impl ShellState {
    /// Start background job with FD state
    pub fn start_background_job(
        &mut self,
        command: String,
        fd_operations: Vec<FdOperation>,
    ) -> Result<usize, String> {
        // Apply FD operations
        let mut fd_context = FdOperationContext::new();
        for operation in fd_operations {
            fd_context.add_operation(operation);
        }

        fd_context.apply(self.fd_manager_mut())?;

        // Create snapshot after applying operations
        let fd_snapshot = self.snapshot_fd_state();

        // Fork and execute
        match unsafe { nix::unistd::fork() } {
            Ok(nix::unistd::ForkResult::Parent { child }) => {
                // Restore original FD state in parent
                fd_context.restore(self.fd_manager_mut())?;

                // Add job
                let job_id = self.add_job_with_fd_state(command, vec![
                    Process {
                        pid: child,
                        command: command.clone(),
                        fd_snapshot: Some(fd_snapshot),
                    }
                ]);

                Ok(job_id)
            }
            Ok(nix::unistd::ForkResult::Child) => {
                // Child continues with modified FD state
                // Execute command
                let exit_code = execute_command(&command, &[], self);
                std::process::exit(exit_code);
            }
            Err(e) => {
                // Restore FD state on error
                fd_context.restore(self.fd_manager_mut())?;
                Err(format!("Fork failed: {}", e))
            }
        }
    }
}
```

### Phase 5: FD State and Functions

#### 5.1 Function FD State

```rust
// src/state.rs
impl ShellState {
    /// Call function with FD state isolation
    pub fn call_function_with_fd_state(
        &mut self,
        name: &str,
        args: Vec<String>,
        fd_operations: Vec<FdOperation>,
    ) -> Result<i32, String> {
        // Save current FD state
        let fd_snapshot = self.snapshot_fd_state();

        // Create new local variable scope
        self.local_vars.push(HashMap::new());

        // Set positional parameters
        for (i, arg) in args.iter().enumerate() {
            self.set_local_var(&format!("{}", i + 1), arg.clone());
        }
        self.set_local_var("0", name.to_string());
        self.set_local_var("#", args.len().to_string());

        // Apply FD operations
        let mut fd_context = FdOperationContext::new();
        for operation in fd_operations {
            fd_context.add_operation(operation);
        }

        fd_context.apply(self.fd_manager_mut())?;

        // Execute function body
        let function = self.functions.get(name)
            .ok_or_else(|| format!("Function not found: {}", name))?
            .clone();

        let exit_code = execute(function, self);

        // Restore FD state
        fd_context.restore(self.fd_manager_mut())?;

        // Remove local variable scope
        self.local_vars.pop();

        Ok(exit_code)
    }
}
```

#### 5.2 Function FD State Export

```rust
// src/state.rs
impl ShellState {
    /// Export FD state from function
    pub fn export_function_fd_state(&mut self, name: &str) -> Result<(), String> {
        let function = self.functions.get(name)
            .ok_or_else(|| format!("Function not found: {}", name))?;

        // Check if function exports FD state
        if let Some(AST::FunctionDef { body, .. }) = function {
            // Look for export statements
            // This is a simplified check
            // In practice, you'd need to parse the function body
        }

        Ok(())
    }
}
```

### Phase 6: FD State and Builtins

#### 6.1 Builtin FD State Access

```rust
// src/state.rs
impl ShellState {
    /// Get FD state for builtin commands
    pub fn get_builtin_fd_state(&self, fd: i32) -> Option<FdState> {
        self.fd_manager.get_fd_state(fd).ok()
    }

    /// Set FD state for builtin commands
    pub fn set_builtin_fd_state(&mut self, fd: i32, state: FdState) -> Result<(), String> {
        self.fd_manager.set_fd_state(fd, state)
    }

    /// Apply FD operation for builtin commands
    pub fn apply_builtin_fd_operation(&mut self, operation: FdOperation) -> Result<(), String> {
        self.fd_manager.apply_operation(operation)
    }
}
```

#### 6.2 Builtin FD State Restoration

```rust
// src/state.rs
impl ShellState {
    /// Execute builtin with FD state isolation
    pub fn execute_builtin_with_fd_state(
        &mut self,
        name: &str,
        args: Vec<String>,
        fd_operations: Vec<FdOperation>,
    ) -> Result<i32, String> {
        // Save current FD state
        let fd_snapshot = self.snapshot_fd_state();

        // Apply FD operations
        let mut fd_context = FdOperationContext::new();
        for operation in fd_operations {
            fd_context.add_operation(operation);
        }

        fd_context.apply(self.fd_manager_mut())?;

        // Execute builtin
        let exit_code = match execute_builtin(name, args, self) {
            Ok(code) => code,
            Err(e) => {
                // Restore FD state on error
                fd_context.restore(self.fd_manager_mut())?;
                return Err(e);
            }
        };

        // Restore FD state
        fd_context.restore(self.fd_manager_mut())?;

        Ok(exit_code)
    }
}
```

### Phase 7: FD State Persistence

#### 7.1 FD State Serialization

```rust
// src/state.rs
impl ShellState {
    /// Serialize FD state to string
    pub fn serialize_fd_state(&self) -> Result<String, String> {
        let snapshot = self.snapshot_fd_state();
        serde_json::to_string(&snapshot)
            .map_err(|e| format!("Failed to serialize FD state: {}", e))
    }

    /// Deserialize FD state from string
    pub fn deserialize_fd_state(&mut self, data: &str) -> Result<(), String> {
        let snapshot: FdStateSnapshot = serde_json::from_str(data)
            .map_err(|e| format!("Failed to deserialize FD state: {}", e))?;

        self.restore_fd_snapshot(snapshot)
    }
}
```

#### 7.2 FD State Save/Load

```rust
// src/state.rs
impl ShellState {
    /// Save FD state to file
    pub fn save_fd_state(&self, path: &Path) -> Result<(), String> {
        let data = self.serialize_fd_state()?;
        std::fs::write(path, data)
            .map_err(|e| format!("Failed to save FD state: {}", e))
    }

    /// Load FD state from file
    pub fn load_fd_state(&mut self, path: &Path) -> Result<(), String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to load FD state: {}", e))?;

        self.deserialize_fd_state(&data)
    }
}
```

## Error Handling

### FD State Errors

```rust
// src/state.rs
pub enum FdStateError {
    /// FD state not found
    FdStateNotFound(i32),
    /// FD state restoration failed
    RestorationFailed(String),
    /// FD state serialization failed
    SerializationFailed(String),
    /// FD state deserialization failed
    DeserializationFailed(String),
    /// FD state snapshot failed
    SnapshotFailed(String),
}

impl std::fmt::Display for FdStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FdStateError::FdStateNotFound(fd) => {
                write!(f, "FD state not found: {}", fd)
            }
            FdStateError::RestorationFailed(msg) => {
                write!(f, "FD state restoration failed: {}", msg)
            }
            FdStateError::SerializationFailed(msg) => {
                write!(f, "FD state serialization failed: {}", msg)
            }
            FdStateError::DeserializationFailed(msg) => {
                write!(f, "FD state deserialization failed: {}", msg)
            }
            FdStateError::SnapshotFailed(msg) => {
                write!(f, "FD state snapshot failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for FdStateError {}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fd_manager_initialization() {
        let state = ShellState::new();
        assert!(state.fd_manager().get_fd_state(0).is_ok());
        assert!(state.fd_manager().get_fd_state(1).is_ok());
        assert!(state.fd_manager().get_fd_state(2).is_ok());
    }

    #[test]
    fn test_fd_state_snapshot() {
        let mut state = ShellState::new();
        let snapshot = state.snapshot_fd_state();
        assert!(snapshot.fd_states.contains_key(&0));
        assert!(snapshot.fd_states.contains_key(&1));
        assert!(snapshot.fd_states.contains_key(&2));
    }

    #[test]
    fn test_fd_state_restore() {
        let mut state = ShellState::new();
        let snapshot = state.snapshot_fd_state();

        // Modify FD state
        state.fd_manager_mut().apply_operation(FdOperation::Redirect {
            target_fd: 2,
            source_fd: Some(1),
            path: None,
            mode: RedirectMode::Write,
        }).unwrap();

        // Restore snapshot
        state.restore_fd_snapshot(snapshot).unwrap();

        // Verify restoration
        let fd2_state = state.fd_manager().get_fd_state(2).unwrap();
        assert_eq!(fd2_state.original_fd, Some(2));
    }

    #[test]
    fn test_subshell_fd_isolation() {
        let mut parent_state = ShellState::new();
        let mut child_state = parent_state.create_subshell_state();

        // Modify child FD state
        child_state.fd_manager_mut().apply_operation(FdOperation::Redirect {
            target_fd: 2,
            source_fd: Some(1),
            path: None,
            mode: RedirectMode::Write,
        }).unwrap();

        // Verify parent FD state is unchanged
        let parent_fd2 = parent_state.fd_manager().get_fd_state(2).unwrap();
        let child_fd2 = child_state.fd_manager().get_fd_state(2).unwrap();

        assert_ne!(parent_fd2, child_fd2);
    }

    #[test]
    fn test_fd_variable_access() {
        let state = ShellState::new();
        assert!(state.get_fd_variable("0").is_some());
        assert!(state.get_fd_variable("1").is_some());
        assert!(state.get_fd_variable("2").is_some());
    }

    #[test]
    fn test_fd_variable_readonly() {
        let mut state = ShellState::new();
        let result = state.set_fd_variable("0", "/dev/null".to_string());
        assert!(result.is_err());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_function_fd_isolation() {
    let mut state = ShellState::new();

    // Define function
    let function_ast = parse("test_func() { echo test 2>&1; }").unwrap();
    state.functions.insert("test_func".to_string(), function_ast);

    // Call function
    let result = state.call_function_with_fd_state(
        "test_func",
        vec![],
        vec![],
    );

    assert!(result.is_ok());
}

#[test]
fn test_job_fd_state() {
    let mut state = ShellState::new();

    // Add job
    let job_id = state.add_job_with_fd_state(
        "sleep 10".to_string(),
        vec![],
    );

    // Restore job FD state
    let result = state.restore_job_fd_state(job_id);
    assert!(result.is_ok());
}
```

## Performance Considerations

### FD State Optimization

1. **Lazy Snapshot**: Only create snapshots when needed
2. **Incremental Updates**: Track only changed FDs
3. **State Compression**: Compress FD state for storage
4. **Caching**: Cache frequently accessed FD states

### Memory Management

1. **Arc Sharing**: Use `Arc` for shared FD state
2. **Weak References**: Use weak references for job FD state
3. **Pool Reuse**: Reuse FD state objects

## Migration Path

### Backward Compatibility

1. **Optional FdManager**: Make FdManager optional initially
2. **Feature Flags**: Use feature flags to enable FD state management
3. **Graceful Degradation**: Fall back to existing behavior on errors

### Rollout Strategy

1. **Phase 1**: Add FdManager to ShellState (no behavior change)
2. **Phase 2**: Implement FD state snapshot/restore
3. **Phase 3**: Enable FD state for subshells
4. **Phase 4**: Enable FD state for functions
5. **Phase 5**: Enable FD state for jobs
6. **Phase 6**: Enable FD state serialization
7. **Phase 7**: Enable FD state for builtins

## Summary

The state management integration for FD operations involves:

1. **FdManager Integration**: Add FdManager to ShellState
2. **FD State Persistence**: Implement snapshot/restore functionality
3. **Subshell Isolation**: Isolate FD state in subshells
4. **Function Integration**: Manage FD state in function calls
5. **Job Management**: Track FD state for background jobs
6. **Builtin Integration**: Provide FD state access for builtins
7. **Serialization**: Support FD state save/load
8. **Testing**: Comprehensive unit and integration tests

This integration provides a robust foundation for managing file descriptor state across the shell's lifetime while maintaining POSIX compliance.
