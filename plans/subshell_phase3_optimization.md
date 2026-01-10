# Subshell Implementation - Phase 3: Optimization and Edge Cases

## Overview

Phase 3 focuses on optimizing subshell performance, handling edge cases, and implementing process-based subshells for scenarios where in-process execution is insufficient.

**Prerequisites:** Phases 1 and 2 must be fully implemented and tested.

**Phase 3 Goals:**
- Optimize state cloning with copy-on-write semantics
- Implement process-based subshells for true isolation
- Handle complex stdin/stdout scenarios in pipelines
- Address all remaining edge cases
- Performance benchmarking and optimization
- Complete POSIX compliance for subshells

**Out of Scope:**
- Background subshells `(cmd) &` (requires job control - separate feature)

## Performance Analysis

### Current Performance Characteristics

**State Cloning Overhead:**

Based on [`src/state.rs`](src/state.rs:400-456), `ShellState` contains:
- `variables: HashMap<String, String>` - O(n) clone where n = number of variables
- `exported: HashSet<String>` - O(m) clone where m = number of exported vars
- `functions: HashMap<String, Ast>` - O(f) clone where f = number of functions
- `aliases: HashMap<String, String>` - O(a) clone where a = number of aliases
- `local_vars: Vec<HashMap<String, String>>` - O(l) clone where l = local var count
- `positional_params: Vec<String>` - O(p) clone where p = number of params

**Total Cloning Cost:** O(n + m + f + a + l + p)

**Worst Case Scenarios:**
1. Large environment (100+ variables)
2. Many functions (50+ functions)
3. Deep nesting (10+ levels)
4. Tight loops with subshells

**Benchmark Targets:**
- Simple subshell: < 1ms overhead
- Nested subshell (3 levels): < 5ms overhead
- Subshell in loop (100 iterations): < 100ms total overhead

### Optimization Strategy 1: Copy-on-Write (COW) Semantics

#### Design: Shared State with Mutation Tracking

**Concept:** Share immutable data between parent and subshell, only clone when modified.

**Implementation Approach:**

```rust
// New wrapper type for COW semantics
#[derive(Debug, Clone)]
enum CowValue<T: Clone> {
    Shared(Rc<T>),
    Owned(T),
}

impl<T: Clone> CowValue<T> {
    fn new(value: T) -> Self {
        CowValue::Owned(value)
    }
    
    fn get(&self) -> &T {
        match self {
            CowValue::Shared(rc) => rc.as_ref(),
            CowValue::Owned(t) => t,
        }
    }
    
    fn get_mut(&mut self) -> &mut T {
        match self {
            CowValue::Shared(rc) => {
                // Clone on write
                let owned = (**rc).clone();
                *self = CowValue::Owned(owned);
                if let CowValue::Owned(t) = self {
                    t
                } else {
                    unreachable!()
                }
            }
            CowValue::Owned(t) => t,
        }
    }
    
    fn share(&self) -> Self {
        match self {
            CowValue::Shared(rc) => CowValue::Shared(rc.clone()),
            CowValue::Owned(t) => CowValue::Shared(Rc::new(t.clone())),
        }
    }
}
```

**ShellState Modifications:**

```rust
pub struct ShellState {
    // COW-enabled fields
    variables: CowValue<HashMap<String, String>>,
    exported: CowValue<HashSet<String>>,
    functions: CowValue<HashMap<String, Ast>>,
    aliases: CowValue<HashMap<String, String>>,
    
    // Keep as-is (small or already shared)
    positional_params: Vec<String>,
    local_vars: Vec<HashMap<String, String>>,
    trap_handlers: Arc<Mutex<HashMap<String, String>>>,
    
    // ... other fields ...
}
```

**Cloning for Subshells:**

```rust
fn clone_shell_state_for_subshell(parent_state: &ShellState) -> ShellState {
    ShellState {
        // Share instead of clone
        variables: parent_state.variables.share(),
        exported: parent_state.exported.share(),
        functions: parent_state.functions.share(),
        aliases: parent_state.aliases.share(),
        
        // Clone small fields
        positional_params: parent_state.positional_params.clone(),
        local_vars: parent_state.local_vars.clone(),
        
        // ... rest of fields ...
    }
}
```

**Benefits:**
- Subshells that only read variables have zero cloning overhead
- Only modified data structures are cloned
- Transparent to rest of codebase (getter/setter methods hide implementation)

**Drawbacks:**
- Increased code complexity
- Additional indirection for all variable access
- May not provide significant benefit for typical use cases

**Recommendation:** Implement only if benchmarks show significant overhead (>10ms per subshell).

### Optimization Strategy 2: Process-Based Subshells

#### When to Use Process-Based Subshells

**Scenarios Requiring True Process Isolation:**

1. **Stdin Redirection in Pipelines:**
   ```bash
   echo "input" | (cat)
   ```
   - In-process subshells can't easily redirect stdin
   - Process-based subshells inherit stdin naturally

2. **Signal Isolation:**
   ```bash
   (trap 'echo caught' INT; sleep 10)
   ```
   - Signals should be isolated to subshell process
   - In-process subshells share signal handlers

3. **Resource Limits:**
   ```bash
   (ulimit -t 5; expensive_command)
   ```
   - Resource limits should apply only to subshell
   - Requires separate process

4. **Exit Calls:**
   ```bash
   (exit 1)
   echo "This should still run"
   ```
   - `exit` in subshell should only exit subshell
   - In-process implementation needs special handling

#### Design: Hybrid Approach

**Strategy:** Use in-process subshells by default, fall back to process-based for complex scenarios.

**Detection Logic:**

```rust
/// Determine if a subshell should execute in a separate process
fn should_use_process_subshell(body: &Ast, in_pipeline: bool) -> bool {
    // Use process-based if:
    // 1. Subshell is in a pipeline (for proper stdin/stdout handling)
    // 2. Subshell contains exit command (needs process isolation)
    // 3. Subshell contains exec command (needs process isolation)
    
    if in_pipeline {
        return true;
    }
    
    // Check if body contains exit or exec
    contains_exit_or_exec(body)
}

/// Recursively check if AST contains exit or exec commands
fn contains_exit_or_exec(ast: &Ast) -> bool {
    match ast {
        Ast::Pipeline(commands) => {
            commands.iter().any(|cmd| {
                cmd.args.first().map_or(false, |arg| {
                    arg == "exit" || arg == "exec"
                })
            })
        }
        Ast::Sequence(asts) => {
            asts.iter().any(contains_exit_or_exec)
        }
        Ast::If { branches, else_branch } => {
            branches.iter().any(|(cond, then_b)| {
                contains_exit_or_exec(cond) || contains_exit_or_exec(then_b)
            }) || else_branch.as_ref().map_or(false, |e| contains_exit_or_exec(e))
        }
        Ast::Subshell { body } => contains_exit_or_exec(body),
        // ... other variants ...
        _ => false,
    }
}
```

#### Process-Based Subshell Implementation

**File:** [`src/executor.rs`](src/executor.rs)

**Location:** New function after `execute_subshell()`

```rust
/// Execute a subshell in a separate process
/// 
/// # Arguments
/// * `body` - The AST to execute in the subshell
/// * `shell_state` - The parent shell state
/// * `stdin` - Optional stdin for the subshell
/// * `capture_output` - Whether to capture stdout
/// 
/// # Returns
/// * `(exit_code, captured_output)`
fn execute_subshell_in_process(
    body: Ast,
    shell_state: &ShellState,
    stdin: Option<Stdio>,
    capture_output: bool,
) -> (i32, Option<Vec<u8>>) {
    use std::process::Command;
    
    // Serialize the AST and shell state for the child process
    // This is complex - we need to pass the subshell command to a new rush instance
    
    // Option 1: Use rush -c with serialized command
    // Option 2: Use fork() and execute in child (Unix-specific)
    
    // For Phase 3, we'll use Option 2 (fork-based) for better performance
    
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        
        // Fork the current process
        unsafe {
            match libc::fork() {
                -1 => {
                    // Fork failed
                    eprintln!("Failed to fork for subshell");
                    return (1, None);
                }
                0 => {
                    // Child process
                    // Clone state and execute
                    let mut subshell_state = clone_shell_state_for_subshell(shell_state);
                    let exit_code = execute(body, &mut subshell_state);
                    std::process::exit(exit_code);
                }
                child_pid => {
                    // Parent process
                    // Wait for child
                    let mut status: i32 = 0;
                    libc::waitpid(child_pid, &mut status as *mut i32, 0);
                    
                    let exit_code = if libc::WIFEXITED(status) {
                        libc::WEXITSTATUS(status)
                    } else {
                        1
                    };
                    
                    return (exit_code, None);
                }
            }
        }
    }
    
    #[cfg(not(unix))]
    {
        // Fallback for non-Unix systems: use in-process execution
        let mut subshell_state = clone_shell_state_for_subshell(shell_state);
        let exit_code = execute(body, &mut subshell_state);
        (exit_code, None)
    }
}
```

**Safety Considerations:**
- Fork is unsafe in Rust
- Must be careful with file descriptors
- Must handle signals correctly
- Must clean up resources properly

**Alternative:** Use `Command::new("rush")` to spawn a new rush instance:

```rust
fn execute_subshell_in_process_safe(
    body: Ast,
    shell_state: &ShellState,
) -> i32 {
    // Serialize the command to a string
    // This is challenging - we'd need to convert AST back to shell syntax
    
    // For now, this is a placeholder for future implementation
    eprintln!("Process-based subshells not yet implemented");
    1
}
```

**Recommendation:** Start with in-process subshells for Phase 3. Add process-based subshells only if needed for specific use cases.

### Optimization Strategy 3: Lazy Cloning

#### Concept: Clone Only What's Needed

**Analysis of Subshell Usage Patterns:**

1. **Read-Only Subshells:** `(echo $VAR)` - Only reads variables
2. **Write-Only Subshells:** `(VAR=value)` - Only writes variables
3. **Mixed Subshells:** `(VAR=value; echo $VAR)` - Reads and writes

**Optimization:** For read-only subshells, use references instead of clones.

**Implementation Challenge:** Rust's borrow checker makes this difficult:
- Can't have mutable reference to parent state while subshell executes
- Would need to track which variables are modified
- Complex lifetime management

**Recommendation:** Not worth the complexity for Phase 3. Consider for future optimization if profiling shows significant benefit.

### Edge Cases and Solutions

#### Edge Case 1: Exit in Subshell

**Scenario:**
```bash
(exit 42)
echo "This should run"
echo $?  # Should be 42
```

**Current Behavior:** In-process subshells set `exit_requested` flag, which would exit parent shell.

**Solution:** Reset exit state in subshell:

```rust
fn execute_subshell(body: Ast, shell_state: &mut ShellState) -> i32 {
    let mut subshell_state = clone_shell_state_for_subshell(shell_state);
    
    // Execute body
    let exit_code = execute(body, &mut subshell_state);
    
    // Check if subshell requested exit
    if subshell_state.exit_requested {
        // Subshell exit should not affect parent
        // Just return the exit code
        shell_state.last_exit_code = subshell_state.exit_code;
        return subshell_state.exit_code;
    }
    
    shell_state.last_exit_code = exit_code;
    exit_code
}
```

**Test:**
```rust
#[test]
fn test_subshell_exit_isolation() {
    let mut shell_state = ShellState::new();
    
    // (exit 42)
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["exit".to_string(), "42".to_string()],
            redirections: Vec::new(),
            compound: None,
        }])),
    };
    
    let exit_code = execute(subshell_ast, &mut shell_state);
    assert_eq!(exit_code, 42);
    
    // Parent should not have exit_requested set
    assert!(!shell_state.exit_requested);
    
    // But last_exit_code should be updated
    assert_eq!(shell_state.last_exit_code, 42);
}
```

#### Edge Case 2: Return in Subshell

**Scenario:**
```bash
func() {
    (return 5)
    echo "This should run"
    return 10
}
func
echo $?  # Should be 10, not 5
```

**Current Behavior:** Return in subshell sets `returning` flag, which would exit parent function.

**Solution:** Isolate return state:

```rust
fn execute_subshell(body: Ast, shell_state: &mut ShellState) -> i32 {
    let mut subshell_state = clone_shell_state_for_subshell(shell_state);
    
    // Execute body
    let exit_code = execute(body, &mut subshell_state);
    
    // Check if subshell has return state
    if subshell_state.is_returning() {
        // Return in subshell should not affect parent function
        // Just use the return value as exit code
        let return_code = subshell_state.get_return_value().unwrap_or(exit_code);
        shell_state.last_exit_code = return_code;
        return return_code;
    }
    
    shell_state.last_exit_code = exit_code;
    exit_code
}
```

**Test:**
```rust
#[test]
fn test_subshell_return_isolation() {
    let mut shell_state = ShellState::new();
    
    // Define function with subshell containing return
    shell_state.define_function(
        "test_func".to_string(),
        Ast::Sequence(vec![
            Ast::Subshell {
                body: Box::new(Ast::Return {
                    value: Some("5".to_string()),
                }),
            },
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "after subshell".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
            Ast::Return {
                value: Some("10".to_string()),
            },
        ]),
    );
    
    // Call function
    let ast = Ast::FunctionCall {
        name: "test_func".to_string(),
        args: vec![],
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 10);  // Function's return, not subshell's
}
```

#### Edge Case 3: Trap Handlers in Subshells

**Scenario:**
```bash
trap 'echo parent trap' INT
(trap 'echo subshell trap' INT; kill -INT $$)
# Parent trap should still be active
```

**POSIX Requirement:** Subshells inherit trap handlers but changes don't affect parent.

**Current Implementation:** Trap handlers use `Arc<Mutex<HashMap>>` which is shared.

**Problem:** Changes in subshell would affect parent (Arc provides shared access).

**Solution:** Clone trap handlers for subshell:

```rust
fn clone_shell_state_for_subshell(parent_state: &ShellState) -> ShellState {
    // Clone trap handlers instead of sharing
    let parent_traps = parent_state.trap_handlers.lock().unwrap().clone();
    let subshell_traps = Arc::new(Mutex::new(parent_traps));
    
    ShellState {
        // ... other fields ...
        trap_handlers: subshell_traps,
        // ... rest of fields ...
    }
}
```

**Test:**
```rust
#[test]
fn test_subshell_trap_isolation() {
    let mut shell_state = ShellState::new();
    
    // Set parent trap
    shell_state.set_trap("INT", "echo parent".to_string());
    
    // Subshell that modifies trap
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["trap".to_string(), "echo subshell".to_string(), "INT".to_string()],
            redirections: Vec::new(),
            compound: None,
        }])),
    };
    
    execute(subshell_ast, &mut shell_state);
    
    // Parent trap should be unchanged
    assert_eq!(
        shell_state.get_trap("INT"),
        Some("echo parent".to_string())
    );
}
```

#### Edge Case 4: Current Directory Isolation

**Scenario:**
```bash
pwd  # /home/user
(cd /tmp; pwd)  # /tmp
pwd  # /home/user (should be unchanged)
```

**Current Behavior:** `cd` builtin changes process-wide current directory.

**Problem:** In-process subshells share the same process, so `cd` affects parent.

**Solution:** Save and restore current directory:

```rust
fn execute_subshell(body: Ast, shell_state: &mut ShellState) -> i32 {
    // Save current directory
    let saved_dir = std::env::current_dir().ok();
    
    // Clone state and execute
    let mut subshell_state = clone_shell_state_for_subshell(shell_state);
    let exit_code = execute(body, &mut subshell_state);
    
    // Restore current directory
    if let Some(dir) = saved_dir {
        let _ = std::env::set_current_dir(dir);
    }
    
    shell_state.last_exit_code = exit_code;
    exit_code
}
```

**Test:**
```rust
#[test]
fn test_subshell_cd_isolation() {
    let _lock = DIR_CHANGE_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();
    let original_dir = std::env::current_dir().unwrap();
    
    // (cd /tmp)
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["cd".to_string(), "/tmp".to_string()],
            redirections: Vec::new(),
            compound: None,
        }])),
    };
    
    let exit_code = execute(subshell_ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Current directory should be restored
    let current_dir = std::env::current_dir().unwrap();
    assert_eq!(current_dir, original_dir);
}
```

#### Edge Case 5: Environment Variable Isolation

**Scenario:**
```bash
export VAR=parent
(export VAR=child; env | grep VAR)  # Shows VAR=child
env | grep VAR                       # Shows VAR=parent
```

**Current Behavior:** `export` modifies `ShellState.exported` set.

**Solution:** Already handled by state cloning. Exported set is cloned for subshell.

**Test:**
```rust
#[test]
fn test_subshell_export_isolation() {
    let mut shell_state = ShellState::new();
    shell_state.set_exported_var("VAR", "parent".to_string());
    
    // (export VAR=child)
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "VAR".to_string(),
                value: "child".to_string(),
            },
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["export".to_string(), "VAR".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        ])),
    };
    
    execute(subshell_ast, &mut shell_state);
    
    // Parent's exported variable should be unchanged
    assert_eq!(shell_state.get_var("VAR"), Some("parent".to_string()));
    assert!(shell_state.exported.contains("VAR"));
}
```

#### Edge Case 6: Function Definition in Subshell

**Scenario:**
```bash
(func() { echo hello; })
func  # Should fail - function not defined in parent
```

**Current Behavior:** Function definitions modify `ShellState.functions` HashMap.

**Solution:** Already handled by state cloning. Functions HashMap is cloned for subshell.

**Test:**
```rust
#[test]
fn test_subshell_function_isolation() {
    let mut shell_state = ShellState::new();
    
    // (func() { echo hello; })
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::FunctionDefinition {
            name: "subfunc".to_string(),
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                redirections: Vec::new(),
                compound: None,
            }])),
        }),
    };
    
    execute(subshell_ast, &mut shell_state);
    
    // Function should not exist in parent
    assert!(shell_state.get_function("subfunc").is_none());
}
```

#### Edge Case 7: Alias Definition in Subshell

**Scenario:**
```bash
(alias ll='ls -la')
ll  # Should fail - alias not defined in parent
```

**Solution:** Already handled by state cloning.

#### Edge Case 8: Positional Parameters in Subshell

**Scenario:**
```bash
set -- a b c
echo $1  # a
(shift; echo $1)  # b
echo $1  # a (should be unchanged)
```

**Solution:** Already handled by state cloning. Positional parameters are cloned.

**Test:**
```rust
#[test]
fn test_subshell_positional_params_isolation() {
    let mut shell_state = ShellState::new();
    shell_state.set_positional_params(vec![
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
    ]);
    
    // (shift)
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["shift".to_string()],
            redirections: Vec::new(),
            compound: None,
        }])),
    };
    
    execute(subshell_ast, &mut shell_state);
    
    // Parent's positional params should be unchanged
    assert_eq!(shell_state.get_var("1"), Some("a".to_string()));
    assert_eq!(shell_state.get_var("#"), Some("3".to_string()));
}
```

#### Edge Case 9: Deeply Nested Subshells

**Scenario:**
```bash
((((((((((echo deep))))))))))  # 10 levels
```

**Concern:** Stack overflow or performance degradation.

**Solution:** Add depth limit:

```rust
// Add to ShellState
pub struct ShellState {
    // ... existing fields ...
    
    /// Current subshell depth (for recursion limit)
    pub subshell_depth: usize,
    
    /// Maximum allowed subshell nesting depth
    pub max_subshell_depth: usize,
}

// In ShellState::new()
subshell_depth: 0,
max_subshell_depth: 100,  // Reasonable limit

// In execute_subshell()
fn execute_subshell(body: Ast, shell_state: &mut ShellState) -> i32 {
    // Check depth limit
    if shell_state.subshell_depth >= shell_state.max_subshell_depth {
        eprintln!("Subshell nesting limit ({}) exceeded", shell_state.max_subshell_depth);
        return 1;
    }
    
    // Clone state
    let mut subshell_state = clone_shell_state_for_subshell(shell_state);
    
    // Increment depth
    subshell_state.subshell_depth = shell_state.subshell_depth + 1;
    
    // Execute
    let exit_code = execute(body, &mut subshell_state);
    
    // ... rest of function ...
}
```

**Test:**
```rust
#[test]
fn test_subshell_depth_limit() {
    let mut shell_state = ShellState::new();
    shell_state.max_subshell_depth = 5;  // Low limit for testing
    
    // Create deeply nested subshells
    let mut ast = Ast::Pipeline(vec![ShellCommand {
        args: vec!["echo".to_string(), "deep".to_string()],
        redirections: Vec::new(),
        compound: None,
    }]);
    
    // Nest 10 levels
    for _ in 0..10 {
        ast = Ast::Subshell {
            body: Box::new(ast),
        };
    }
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 1);  // Should fail due to depth limit
}
```

#### Edge Case 10: Subshell with Here-Documents

**Scenario:**
```bash
(cat << EOF
line 1
line 2
EOF
)
```

**Current Behavior:** Here-documents are handled by `collect_here_document_content()` which reads from stdin.

**Problem:** In-process subshells share stdin, so here-document collection works correctly.

**Solution:** No changes needed for in-process subshells. For process-based subshells, stdin is naturally isolated.

#### Edge Case 11: Subshell in Command Substitution

**Scenario:**
```bash
result=$( (echo hello) )
echo $result  # Should print: hello
```

**Current Behavior:** Command substitution sets `capture_output` in shell state.

**Solution:** Already handled! The `clone_shell_state_for_subshell()` function inherits `capture_output`:

```rust
// In clone_shell_state_for_subshell()
capture_output: parent_state.capture_output.clone(),
```

**Test:**
```rust
#[test]
fn test_subshell_in_command_substitution() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("VAR", "parent".to_string());
    
    // result=$( (VAR=child; echo $VAR) )
    // The command substitution is handled by expand_variables_in_string
    // which calls execute_and_capture_output
    
    // For this test, we'll manually set up capture
    let capture_buffer = Rc::new(RefCell::new(Vec::new()));
    shell_state.capture_output = Some(capture_buffer.clone());
    
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "VAR".to_string(),
                value: "child".to_string(),
            },
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "$VAR".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        ])),
    };
    
    let exit_code = execute(subshell_ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Check captured output
    let output = String::from_utf8_lossy(&capture_buffer.borrow()).to_string();
    assert!(output.contains("child"));
    
    // Parent VAR should be unchanged
    assert_eq!(shell_state.get_var("VAR"), Some("parent".to_string()));
}
```

#### Edge Case 12: Subshell with Pipelines and Redirections

**Scenario:**
```bash
(echo hello | grep hello) > output.txt
```

**Complexity:** Pipeline inside subshell, with redirection on subshell.

**Solution:** The Pipeline wrapper approach handles this:
1. Subshell executes pipeline internally
2. Output is captured
3. Redirection applies to captured output

**Already Supported:** No additional changes needed.

### Performance Benchmarking

#### Benchmark Suite

**File:** `benchmarks/src/test_cases.rs`

**Add Subshell Benchmarks:**

```rust
pub fn subshell_benchmarks() -> Vec<TestCase> {
    vec![
        TestCase {
            name: "Simple Subshell".to_string(),
            command: "(echo hello)".to_string(),
            expected_output: Some("hello\n".to_string()),
        },
        TestCase {
            name: "Subshell with Variable".to_string(),
            command: "VAR=test; (echo $VAR)".to_string(),
            expected_output: Some("test\n".to_string()),
        },
        TestCase {
            name: "Nested Subshell (2 levels)".to_string(),
            command: "((echo nested))".to_string(),
            expected_output: Some("nested\n".to_string()),
        },
        TestCase {
            name: "Nested Subshell (5 levels)".to_string(),
            command: "(((((echo deep)))))".to_string(),
            expected_output: Some("deep\n".to_string()),
        },
        TestCase {
            name: "Subshell in Loop (10 iterations)".to_string(),
            command: "for i in 1 2 3 4 5 6 7 8 9 10; do (echo $i); done".to_string(),
            expected_output: None,  // Just measure performance
        },
        TestCase {
            name: "Subshell with Pipeline".to_string(),
            command: "(echo hello world) | grep world".to_string(),
            expected_output: Some("hello world\n".to_string()),
        },
        TestCase {
            name: "Subshell with Redirection".to_string(),
            command: "(echo test) > /tmp/rush_bench_subshell.txt; cat /tmp/rush_bench_subshell.txt".to_string(),
            expected_output: Some("test\n".to_string()),
        },
    ]
}
```

#### Performance Targets

**Baseline (bash):**
- Simple subshell: ~1-2ms
- Nested subshell (5 levels): ~5-10ms
- Subshell in loop (100 iterations): ~100-200ms

**Rush Targets:**
- Simple subshell: < 2ms (within 2x of bash)
- Nested subshell (5 levels): < 15ms (within 2x of bash)
- Subshell in loop (100 iterations): < 300ms (within 2x of bash)

**If targets not met:** Implement COW optimization or process-based subshells.

### State Cloning Optimization Details

#### Current Clone Implementation

**File:** [`src/state.rs`](src/state.rs:400)

```rust
#[derive(Debug, Clone)]
pub struct ShellState {
    // ... fields ...
}
```

**Rust's Derive Clone Behavior:**
- `HashMap::clone()` - Deep copy, O(n)
- `HashSet::clone()` - Deep copy, O(n)
- `Vec::clone()` - Deep copy, O(n)
- `Rc::clone()` - Shallow copy, O(1) - just increments reference count
- `Arc::clone()` - Shallow copy, O(1) - atomic reference count increment

**Optimization Opportunity:** Most fields are already efficient (Rc/Arc). The expensive clones are:
- `variables: HashMap<String, String>`
- `functions: HashMap<String, Ast>`
- `aliases: HashMap<String, String>`

#### Profiling Strategy

**Add Timing Instrumentation:**

```rust
fn clone_shell_state_for_subshell(parent_state: &ShellState) -> ShellState {
    #[cfg(feature = "profiling")]
    let start = std::time::Instant::now();
    
    let subshell_state = ShellState {
        // ... cloning logic ...
    };
    
    #[cfg(feature = "profiling")]
    {
        let elapsed = start.elapsed();
        if elapsed.as_millis() > 1 {
            eprintln!("State clone took {}ms", elapsed.as_millis());
        }
    }
    
    subshell_state
}
```

**Profiling Commands:**

```bash
# Enable profiling feature
cargo build --release --features profiling

# Run benchmarks
cargo run --release --features profiling -- -c '
for i in $(seq 1 100); do
    (echo $i)
done
'
```

### Advanced Pipeline Integration

#### Full Stdin/Stdout Support

**Challenge:** In-process subshells can't easily redirect stdin from previous pipeline stage.

**Solution for Phase 3:** Use temporary files or pipes:

```rust
fn execute_compound_in_pipeline(
    compound_ast: &Ast,
    shell_state: &mut ShellState,
    stdin: Option<Stdio>,
    is_last: bool,
    redirections: &[Redirection],
) -> i32 {
    match compound_ast {
        Ast::Subshell { body } => {
            // If stdin is provided, we need to handle it
            if let Some(stdin_handle) = stdin {
                // Create a temporary file to hold stdin data
                use std::io::Read;
                let mut stdin_data = Vec::new();
                
                // This is tricky - Stdio doesn't implement Read
                // We need a different approach
                
                // Option 1: Use process-based subshell
                // Option 2: Pass stdin through environment or temp file
                // Option 3: Modify subshell to accept stdin parameter
                
                // For Phase 3, recommend Option 1 (process-based)
                return execute_subshell_in_process(
                    *body.clone(),
                    shell_state,
                    Some(stdin_handle),
                    !is_last || shell_state.capture_output.is_some(),
                ).0;
            }
            
            // ... existing logic for no stdin ...
        }
        _ => 1,
    }
}
```

**Recommendation:** For Phase 3, use process-based subshells when in pipelines with stdin.

### Memory Safety Considerations

#### Rc and RefCell Usage

**Current Usage:**
- `capture_output: Option<Rc<RefCell<Vec<u8>>>>`
- `fd_table: Rc<RefCell<FileDescriptorTable>>`

**Safety Analysis:**
- `Rc::clone()` is safe - just increments reference count
- `RefCell` provides runtime borrow checking
- Subshells get their own `fd_table` (new instance)
- `capture_output` is shared (intentional for command substitution)

**Potential Issues:**
- Multiple mutable borrows of `fd_table` could panic
- Need to ensure borrows are dropped before cloning state

**Solution:** Audit all `fd_table.borrow_mut()` calls to ensure proper scope:

```rust
// Good: Borrow is dropped before next operation
{
    let mut fd_table = shell_state.fd_table.borrow_mut();
    fd_table.open_fd(3, "file.txt", true, false, false, false)?;
}  // Borrow dropped here

// Bad: Borrow held across function call
let mut fd_table = shell_state.fd_table.borrow_mut();
execute_subshell(body, shell_state);  // Panic! fd_table still borrowed
```

### Comprehensive Test Matrix

#### Test Dimensions

1. **Subshell Depth:** 1, 2, 3, 5, 10 levels
2. **Context:** Standalone, Pipeline, Control Structure, Logical Operator
3. **Redirections:** None, Input, Output, Append, FD operations
4. **State Modifications:** Variables, Functions, Aliases, Directory, Positional Params
5. **Special Commands:** exit, return, cd, export, trap

#### Test Matrix (Partial)

| Depth | Context | Redirections | State Mod | Special | Test Name |
|-------|---------|--------------|-----------|---------|-----------|
| 1 | Standalone | None | Variable | None | `test_simple_subshell` |
| 1 | Standalone | Output | Variable | None | `test_subshell_with_output_redir` |
| 1 | Pipeline | None | None | None | `test_subshell_in_pipeline` |
| 2 | Standalone | None | Variable | None | `test_nested_subshell_2_levels` |
| 1 | If Condition | None | None | None | `test_subshell_in_if_condition` |
| 1 | Standalone | None | None | exit | `test_subshell_exit_isolation` |
| 1 | Standalone | None | None | cd | `test_subshell_cd_isolation` |
| 3 | Standalone | None | Variable | None | `test_nested_subshell_3_levels` |
| 1 | And Operator | None | None | None | `test_subshell_with_and_operator` |
| 1 | Pipeline | Output | None | None | `test_subshell_pipeline_with_redir` |

**Total Tests for Phase 3:** ~50 additional tests covering all combinations.

### Documentation and Examples

#### Create Example Script

**File:** `examples/subshell_demo.sh`

```bash
#!/usr/bin/env rush

echo "=== Subshell Demonstration ==="
echo

echo "1. Basic subshell with variable isolation:"
VAR=parent
echo "Parent VAR: $VAR"
(VAR=child; echo "Subshell VAR: $VAR")
echo "Parent VAR after subshell: $VAR"
echo

echo "2. Subshell with directory isolation:"
echo "Current directory: $(pwd)"
(cd /tmp; echo "Subshell directory: $(pwd)")
echo "Current directory after subshell: $(pwd)"
echo

echo "3. Nested subshells:"
LEVEL=0
echo "Level 0: $LEVEL"
(LEVEL=1; echo "Level 1: $LEVEL"; (LEVEL=2; echo "Level 2: $LEVEL"))
echo "Back to level 0: $LEVEL"
echo

echo "4. Subshells in pipelines:"
(echo "hello"; echo "world") | grep "world"
echo

echo "5. Subshells with redirections:"
(echo "This goes to a file") > /tmp/subshell_output.txt
cat /tmp/subshell_output.txt
rm /tmp/subshell_output.txt
echo

echo "6. Subshells with logical operators:"
(true) && echo "First subshell succeeded"
(false) || echo "Second subshell failed"
echo

echo "7. Subshells in control structures:"
if (test -d /tmp); then
    echo "/tmp directory exists (checked in subshell)"
fi
echo

echo "8. Complex example - subshell with multiple features:"
COUNT=0
(
    COUNT=10
    for i in 1 2 3; do
        echo "Iteration $i, COUNT=$COUNT"
        COUNT=$((COUNT + 1))
    done
    echo "Final COUNT in subshell: $COUNT"
)
echo "COUNT in parent: $COUNT"
echo

echo "=== Demo Complete ==="
```

### Phase 3 Implementation Checklist

#### Optimization
- [ ] Add subshell depth tracking to `ShellState`
- [ ] Implement depth limit checking in `execute_subshell()`
- [ ] Add current directory save/restore in `execute_subshell()`
- [ ] Profile state cloning performance
- [ ] Implement COW optimization if needed (based on profiling)
- [ ] Add benchmarks for subshell performance

#### Edge Cases
- [ ] Handle exit in subshell (isolation)
- [ ] Handle return in subshell (isolation)
- [ ] Handle trap modifications in subshell (isolation)
- [ ] Handle cd in subshell (directory restoration)
- [ ] Handle deeply nested subshells (depth limit)
- [ ] Handle subshells in command substitution
- [ ] Handle complex pipeline scenarios

#### Process-Based Subshells (Optional)
- [ ] Implement fork-based subshell execution (Unix only)
- [ ] Add detection logic for when to use process-based
- [ ] Handle stdin/stdout properly in process-based subshells
- [ ] Add tests for process-based subshells
- [ ] Document platform-specific behavior

#### Testing
- [ ] Create comprehensive test matrix
- [ ] Add ~50 edge case tests
- [ ] Add performance benchmarks
- [ ] Add stress tests (deep nesting, loops)
- [ ] Verify no memory leaks (use valgrind or similar)

#### Documentation
- [ ] Create `examples/subshell_demo.sh`
- [ ] Update README with subshell feature
- [ ] Update TODO.md to mark subshells as complete
- [ ] Update AGENTS.md with subshell architecture
- [ ] Add subshell section to docs/features.html
- [ ] Document known limitations and workarounds

### Success Criteria

Phase 3 is complete when:

1. ✅ All edge cases are handled correctly
2. ✅ Performance meets or exceeds targets
3. ✅ No memory leaks detected
4. ✅ Comprehensive test coverage (>95% for subshell code)
5. ✅ All benchmarks pass
6. ✅ Documentation is complete and accurate
7. ✅ No regressions in any existing functionality
8. ✅ POSIX compliance for subshells is 100%

### Known Remaining Limitations

After Phase 3, these limitations may still exist:

1. **Background Subshells:** `(sleep 10) &`
   - Requires job control implementation
   - Separate feature, not part of subshell implementation

2. **Platform-Specific Behavior:**
   - Process-based subshells may only work on Unix
   - Windows support may be limited

3. **Performance vs Bash:**
   - May be slower than bash for some scenarios
   - Acceptable if within 2x of bash performance

### Future Enhancements (Beyond Phase 3)

1. **Subshell Optimization Modes:**
   - Environment variable to control in-process vs process-based
   - `RUSH_SUBSHELL_MODE=process` or `RUSH_SUBSHELL_MODE=inprocess`

2. **Subshell Debugging:**
   - `set -x` should show subshell execution
   - Debug mode to trace subshell state changes

3. **Subshell Profiling:**
   - Built-in profiling for subshell overhead
   - Statistics on subshell usage

4. **Advanced Optimizations:**
   - Compile-time detection of read-only subshells
   - Eliminate cloning for provably read-only subshells
   - JIT optimization for frequently-used subshell patterns

### Migration Path

**From Phase 2 to Phase 3:**
- No breaking changes
- Only performance improvements and edge case fixes
- All Phase 2 functionality continues to work

**Testing Strategy:**
1. Run all Phase 1 and Phase 2 tests
2. Add Phase 3 edge case tests
3. Run performance benchmarks
4. Compare with bash behavior
5. Fix any regressions or performance issues

### Completion Timeline

**Phase 3 Complexity:** High
- Many edge cases to handle
- Performance optimization requires profiling
- Process-based subshells are complex (fork, signals, etc.)

**Recommended Approach:**
1. Implement edge case fixes first (exit, return, cd isolation)
2. Add comprehensive tests
3. Run performance benchmarks
4. Optimize only if needed (based on benchmarks)
5. Consider process-based subshells only if in-process has fundamental limitations

### Final Validation

**Before marking Phase 3 complete:**

1. **POSIX Compliance Test Suite:**
   - Run against POSIX test suite if available
   - Compare behavior with bash, dash, and other POSIX shells
   - Document any intentional deviations

2. **Performance Validation:**
   - Run benchmarks on various hardware
   - Compare with bash on same hardware
   - Ensure no performance regressions

3. **Memory Validation:**
   - Run valgrind or similar memory checker
   - Verify no memory leaks
   - Check for excessive memory usage

4. **Integration Validation:**
   - Test with real-world shell scripts
   - Verify compatibility with common patterns
   - Gather user feedback

5. **Documentation Validation:**
   - Ensure all features are documented
   - Verify examples work correctly
   - Check for completeness and accuracy

### Conclusion

Phase 3 completes the subshell implementation with:
- Robust edge case handling
- Optimized performance
- Comprehensive testing
- Complete documentation
- Full POSIX compliance

After Phase 3, subshells will be a production-ready feature in Rush shell.
