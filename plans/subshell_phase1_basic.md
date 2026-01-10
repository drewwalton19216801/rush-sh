# Subshell Implementation - Phase 1: Basic Support

## Overview

This document outlines Phase 1 of implementing subshell support in Rush shell. A subshell is a command or group of commands enclosed in parentheses `(...)` that executes in an isolated copy of the shell state. This phase focuses on establishing the foundational infrastructure for basic subshell execution.

**Phase 1 Goals:**
- Add subshell AST node type
- Implement lexer recognition of subshell parentheses
- Implement parser support for basic subshell syntax
- Implement executor support with state isolation
- Handle simple subshell execution with proper exit codes

**Out of Scope for Phase 1:**
- Nested subshells (deferred to Phase 2)
- Subshells in pipelines (deferred to Phase 2)
- Complex redirection scenarios (deferred to Phase 2)
- Performance optimizations (deferred to Phase 3)

## Current Architecture Analysis

### AST Structure ([`src/parser.rs`](src/parser.rs:3-52))

The current AST enum defines these node types:
- `Pipeline(Vec<ShellCommand>)` - Command pipelines
- `Sequence(Vec<Ast>)` - Sequential command execution
- `Assignment`, `LocalAssignment` - Variable assignments
- `If`, `Case`, `For`, `While` - Control structures
- `FunctionDefinition`, `FunctionCall` - Function support
- `Return` - Function returns
- `And`, `Or` - Logical operators

**Key Observation:** The AST already supports nested structures through `Box<Ast>` in control structures, which provides a pattern for subshell implementation.

### Lexer Token Recognition ([`src/lexer.rs`](src/lexer.rs:8-46))

Current token types include:
- `LeftParen`, `RightParen` - Already exist for function definitions
- `LeftBrace`, `RightBrace` - Used for function bodies and brace expansion

**Key Observation:** The lexer already recognizes parentheses (lines 915-918, 905-909) but treats them as separate tokens. Currently used for:
1. Function definitions: `name() { ... }`
2. Case statement patterns: `pattern) commands ;;`

**Challenge:** Need to distinguish between:
- Function definition parentheses: `func() { ... }`
- Subshell parentheses: `(command)`
- Case pattern parentheses: `pattern) ...`
- Arithmetic expansion parentheses: `$((...))`
- Command substitution parentheses: `$(...)`

### Parser Logic ([`src/parser.rs`](src/parser.rs:157-256))

The parser has sophisticated logic for:
- Function definition detection (lines 159-241, 387-407)
- Control structure parsing with depth tracking
- Handling of nested structures

**Key Observation:** The parser already handles `RightParen` in pipelines (line 698-710) for function call patterns. This logic needs to be extended to handle subshells.

### Executor Model ([`src/executor.rs`](src/executor.rs:897-1202))

The executor uses pattern matching on AST nodes:
- Each AST variant has dedicated execution logic
- State is passed as `&mut ShellState`
- Exit codes are propagated correctly

**Key Observation:** The executor already has precedent for state isolation in function calls (lines 1097-1147) with `enter_function()` and `exit_function()`.

### State Management ([`src/state.rs`](src/state.rs:400-456))

`ShellState` contains:
- `variables: HashMap<String, String>` - Shell variables
- `exported: HashSet<String>` - Exported variables
- `local_vars: Vec<HashMap<String, String>>` - Local variable scopes
- `positional_params: Vec<String>` - Positional parameters
- `functions: HashMap<String, Ast>` - Function definitions
- `aliases: HashMap<String, String>` - Command aliases
- `dir_stack: Vec<String>` - Directory stack
- `fd_table: Rc<RefCell<FileDescriptorTable>>` - File descriptors
- Various flags and configuration

**Key Observation:** `ShellState` implements `Clone` (line 400), which is essential for subshell state isolation. However, some fields use `Rc` and `Arc` which need special handling.

## Phase 1 Design

### 1. AST Changes

#### Add Subshell Variant to Ast Enum

**File:** [`src/parser.rs`](src/parser.rs:3-52)

**Location:** After line 51 (after `Or` variant)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ast {
    // ... existing variants ...
    Or {
        left: Box<Ast>,
        right: Box<Ast>,
    },
    /// Subshell execution: (commands)
    /// Commands execute in an isolated copy of the shell state
    Subshell {
        body: Box<Ast>,
    },
}
```

**Rationale:**
- Simple structure with single `body` field containing the AST to execute
- Uses `Box<Ast>` for heap allocation (consistent with other compound commands)
- The body can be any AST node (Pipeline, Sequence, control structures, etc.)

### 2. Lexer Changes

#### Context-Aware Parenthesis Recognition

**File:** [`src/lexer.rs`](src/lexer.rs:915-918)

**Current Behavior:**
```rust
'(' if !in_double_quote && !in_single_quote => {
    flush_current_token(&mut current, &mut tokens);
    tokens.push(Token::LeftParen);
    chars.next();
}
```

**Problem:** The lexer unconditionally emits `LeftParen` tokens without context. This works for function definitions but doesn't distinguish subshells.

**Solution:** Keep lexer unchanged for Phase 1. The lexer should continue to emit `LeftParen` and `RightParen` tokens. The parser will determine context based on surrounding tokens.

**Rationale:**
- Lexer's job is tokenization, not semantic analysis
- Parser already has context to distinguish usage patterns
- Maintains separation of concerns
- Simpler implementation with fewer edge cases

### 3. Parser Changes

#### Add Subshell Detection Logic

**File:** [`src/parser.rs`](src/parser.rs:416-628)

**Location:** In `parse_commands_sequentially()` function, before the compound command checks (around line 447)

**New Logic:**

```rust
// Check for subshell: LeftParen at start of command
if tokens[i] == Token::LeftParen {
    // This is a subshell - find the matching RightParen
    let mut paren_depth = 1;
    let mut j = i + 1;
    
    while j < tokens.len() && paren_depth > 0 {
        match tokens[j] {
            Token::LeftParen => paren_depth += 1,
            Token::RightParen => paren_depth -= 1,
            _ => {}
        }
        j += 1;
    }
    
    if paren_depth != 0 {
        return Err("Unmatched parenthesis in subshell".to_string());
    }
    
    // Extract subshell body (tokens between parens)
    let subshell_tokens = &tokens[i + 1..j - 1];
    
    if subshell_tokens.is_empty() {
        return Err("Empty subshell".to_string());
    }
    
    // Parse the subshell body recursively
    let body_ast = parse_commands_sequentially(subshell_tokens)?;
    
    let subshell_ast = Ast::Subshell {
        body: Box::new(body_ast),
    };
    
    commands.push(subshell_ast);
    i = j; // Move past the closing paren
    
    // Handle operators after subshell (&&, ||, ;, newline)
    if i < tokens.len() && (tokens[i] == Token::And || tokens[i] == Token::Or) {
        // Handle logical operators (existing logic)
        // ...
    }
    
    continue;
}
```

**Integration Points:**

1. **Function Definition Detection** (lines 513-530): Must check for function pattern BEFORE subshell check
   ```rust
   // Check for function definition first: Word LeftParen RightParen LeftBrace
   if i + 3 < tokens.len()
       && matches!(tokens[i], Token::Word(_))
       && tokens[i + 1] == Token::LeftParen
       && tokens[i + 2] == Token::RightParen
       && tokens[i + 3] == Token::LeftBrace
   {
       // Parse as function definition
   }
   // THEN check for subshell
   else if tokens[i] == Token::LeftParen {
       // Parse as subshell
   }
   ```

2. **Case Pattern Handling:** Case patterns use `RightParen` (line 951-970). This is handled separately in `parse_case()` and won't conflict.

#### Add Subshell Parsing Function

**File:** [`src/parser.rs`](src/parser.rs)

**Location:** After `parse_function_definition()` (after line 1347)

```rust
/// Parse a subshell command: (commands)
/// 
/// # Arguments
/// * `tokens` - Token slice starting with LeftParen and ending with RightParen
/// 
/// # Returns
/// * `Ok(Ast::Subshell)` on success
/// * `Err(String)` with error message on failure
fn parse_subshell(tokens: &[Token]) -> Result<Ast, String> {
    if tokens.is_empty() {
        return Err("Empty token slice for subshell".to_string());
    }
    
    if tokens[0] != Token::LeftParen {
        return Err("Subshell must start with (".to_string());
    }
    
    // Find matching closing paren
    let mut paren_depth = 1;
    let mut end_pos = 0;
    let mut i = 1;
    
    while i < tokens.len() {
        match tokens[i] {
            Token::LeftParen => paren_depth += 1,
            Token::RightParen => {
                paren_depth -= 1;
                if paren_depth == 0 {
                    end_pos = i;
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    
    if paren_depth != 0 {
        return Err("Unmatched parenthesis in subshell".to_string());
    }
    
    // Extract body tokens (between parens)
    let body_tokens = &tokens[1..end_pos];
    
    if body_tokens.is_empty() {
        return Err("Empty subshell body".to_string());
    }
    
    // Parse body recursively
    let body_ast = parse_commands_sequentially(body_tokens)?;
    
    Ok(Ast::Subshell {
        body: Box::new(body_ast),
    })
}
```

### 4. Executor Changes

#### Add Subshell Execution Logic

**File:** [`src/executor.rs`](src/executor.rs:897-1202)

**Location:** In `execute()` function, add new match arm after `Or` variant (after line 1200)

```rust
pub fn execute(ast: Ast, shell_state: &mut ShellState) -> i32 {
    match ast {
        // ... existing variants ...
        Ast::Or { left, right } => {
            // existing logic
        }
        Ast::Subshell { body } => {
            execute_subshell(*body, shell_state)
        }
    }
}
```

#### Add Subshell Execution Function

**File:** [`src/executor.rs`](src/executor.rs)

**Location:** After `execute_pipeline()` function (after line 1578)

```rust
/// Execute a subshell with isolated state
/// 
/// # Arguments
/// * `body` - The AST to execute in the subshell
/// * `shell_state` - The parent shell state (will be cloned)
/// 
/// # Returns
/// * Exit code from the subshell execution
/// 
/// # Behavior
/// - Clones the shell state for isolation
/// - Executes the body in the cloned state
/// - Returns the exit code without modifying parent state
/// - Preserves parent state completely (variables, functions, etc.)
fn execute_subshell(body: Ast, shell_state: &mut ShellState) -> i32 {
    // Clone the shell state for isolation
    let mut subshell_state = clone_shell_state_for_subshell(shell_state);
    
    // Execute the body in the isolated state
    let exit_code = execute(body, &mut subshell_state);
    
    // Update parent's last_exit_code to reflect subshell result
    shell_state.last_exit_code = exit_code;
    
    // Return the exit code
    exit_code
}
```

### 5. State Cloning for Subshells

#### Add State Cloning Helper

**File:** [`src/executor.rs`](src/executor.rs)

**Location:** After `execute_subshell()` function

```rust
/// Clone shell state for subshell execution
/// 
/// This creates a deep copy of the shell state with special handling for:
/// - Rc/Arc wrapped fields (need new instances)
/// - File descriptor table (needs isolation)
/// - Trap handlers (shared with parent)
/// 
/// # Arguments
/// * `parent_state` - The parent shell state to clone
/// 
/// # Returns
/// * A new ShellState instance isolated from the parent
fn clone_shell_state_for_subshell(parent_state: &ShellState) -> ShellState {
    // Create a new state with cloned data
    let mut subshell_state = ShellState {
        // Clone simple fields
        variables: parent_state.variables.clone(),
        exported: parent_state.exported.clone(),
        last_exit_code: parent_state.last_exit_code,
        shell_pid: parent_state.shell_pid,
        script_name: parent_state.script_name.clone(),
        dir_stack: parent_state.dir_stack.clone(),
        aliases: parent_state.aliases.clone(),
        colors_enabled: parent_state.colors_enabled,
        color_scheme: parent_state.color_scheme.clone(),
        positional_params: parent_state.positional_params.clone(),
        functions: parent_state.functions.clone(),
        
        // Clone local variable scopes
        local_vars: parent_state.local_vars.clone(),
        function_depth: parent_state.function_depth,
        max_recursion_depth: parent_state.max_recursion_depth,
        
        // Reset function return state (subshell starts fresh)
        returning: false,
        return_value: None,
        
        // Inherit capture_output state (for command substitution in subshells)
        capture_output: parent_state.capture_output.clone(),
        
        // Clone display settings
        condensed_cwd: parent_state.condensed_cwd,
        
        // Share trap handlers with parent (Arc allows shared access)
        trap_handlers: parent_state.trap_handlers.clone(),
        
        // Reset exit state
        exit_trap_executed: false,
        exit_requested: false,
        exit_code: 0,
        pending_signals: false,
        
        // Reset heredoc state
        pending_heredoc_content: None,
        collecting_heredoc: None,
        
        // Create new file descriptor table (isolated from parent)
        fd_table: Rc::new(RefCell::new(FileDescriptorTable::new())),
    };
    
    subshell_state
}
```

**Critical Design Decisions:**

1. **File Descriptor Table:** Create NEW instance for subshell
   - Subshells should not affect parent's file descriptors
   - Standard fds (0, 1, 2) are inherited from parent process
   - Custom fds (3-9) start fresh in subshell

2. **Trap Handlers:** SHARE with parent via `Arc::clone()`
   - Trap handlers are inherited by subshells per POSIX
   - Changes to traps in subshell don't affect parent (Arc provides shared read access)
   - This is correct behavior per POSIX specification

3. **Capture Output:** INHERIT from parent
   - Allows command substitution to work: `$(( subshell ))`
   - Subshell output is captured if parent is capturing

4. **Function Return State:** RESET in subshell
   - `return` in subshell exits the subshell, not parent function
   - Subshell starts with clean return state

### 6. Test Strategy

#### Test File Structure

**File:** [`src/parser.rs`](src/parser.rs:1349-2071) (add to existing test module)

**Location:** After existing parser tests (after line 2070)

```rust
#[test]
fn test_parse_simple_subshell() {
    let tokens = vec![
        Token::LeftParen,
        Token::Word("echo".to_string()),
        Token::Word("hello".to_string()),
        Token::RightParen,
    ];
    let result = parse(tokens).unwrap();
    
    if let Ast::Subshell { body } = result {
        if let Ast::Pipeline(cmds) = *body {
            assert_eq!(cmds[0].args, vec!["echo", "hello"]);
        } else {
            panic!("Subshell body should be a pipeline");
        }
    } else {
        panic!("Should be parsed as subshell");
    }
}

#[test]
fn test_parse_subshell_with_sequence() {
    let tokens = vec![
        Token::LeftParen,
        Token::Word("echo".to_string()),
        Token::Word("first".to_string()),
        Token::Semicolon,
        Token::Word("echo".to_string()),
        Token::Word("second".to_string()),
        Token::RightParen,
    ];
    let result = parse(tokens).unwrap();
    
    if let Ast::Subshell { body } = result {
        if let Ast::Sequence(cmds) = *body {
            assert_eq!(cmds.len(), 2);
        } else {
            panic!("Subshell body should be a sequence");
        }
    } else {
        panic!("Should be parsed as subshell");
    }
}

#[test]
fn test_parse_empty_subshell() {
    let tokens = vec![
        Token::LeftParen,
        Token::RightParen,
    ];
    let result = parse(tokens);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Empty subshell"));
}

#[test]
fn test_parse_unmatched_left_paren() {
    let tokens = vec![
        Token::LeftParen,
        Token::Word("echo".to_string()),
        Token::Word("hello".to_string()),
    ];
    let result = parse(tokens);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unmatched parenthesis"));
}
```

#### Executor Tests

**File:** [`src/executor.rs`](src/executor.rs:1580-2386) (add to existing test module)

**Location:** After existing executor tests (after line 2385)

```rust
#[test]
fn test_execute_simple_subshell() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("TEST_VAR", "parent_value".to_string());
    
    // Subshell that modifies a variable
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::Assignment {
            var: "TEST_VAR".to_string(),
            value: "subshell_value".to_string(),
        }),
    };
    
    let exit_code = execute(subshell_ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Parent variable should be unchanged
    assert_eq!(
        shell_state.get_var("TEST_VAR"),
        Some("parent_value".to_string())
    );
}

#[test]
fn test_execute_subshell_exit_code() {
    let mut shell_state = ShellState::new();
    
    // Subshell that returns non-zero exit code
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["false".to_string()],
            redirections: Vec::new(),
        }])),
    };
    
    let exit_code = execute(subshell_ast, &mut shell_state);
    assert_eq!(exit_code, 1);
    
    // Parent's last_exit_code should be updated
    assert_eq!(shell_state.last_exit_code, 1);
}

#[test]
fn test_execute_subshell_with_sequence() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("COUNTER", "0".to_string());
    
    // Subshell with multiple commands
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "COUNTER".to_string(),
                value: "1".to_string(),
            },
            Ast::Assignment {
                var: "COUNTER".to_string(),
                value: "2".to_string(),
            },
        ])),
    };
    
    let exit_code = execute(subshell_ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Parent COUNTER should still be 0
    assert_eq!(shell_state.get_var("COUNTER"), Some("0".to_string()));
}

#[test]
fn test_subshell_inherits_variables() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("PARENT_VAR", "parent_value".to_string());
    
    // Subshell that reads parent variable
    // We'll need to capture output to verify this
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["echo".to_string(), "$PARENT_VAR".to_string()],
            redirections: Vec::new(),
        }])),
    };
    
    let exit_code = execute(subshell_ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Parent variable unchanged
    assert_eq!(
        shell_state.get_var("PARENT_VAR"),
        Some("parent_value".to_string())
    );
}

#[test]
fn test_subshell_exported_variables() {
    let mut shell_state = ShellState::new();
    shell_state.set_exported_var("EXPORTED_VAR", "exported_value".to_string());
    
    // Subshell should inherit exported variables
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::Assignment {
            var: "EXPORTED_VAR".to_string(),
            value: "modified_in_subshell".to_string(),
        }),
    };
    
    let exit_code = execute(subshell_ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Parent's exported variable should be unchanged
    assert_eq!(
        shell_state.get_var("EXPORTED_VAR"),
        Some("exported_value".to_string())
    );
    assert!(shell_state.exported.contains("EXPORTED_VAR"));
}
```

### 7. Edge Cases for Phase 1

#### Handled in Phase 1:

1. **Empty Subshells:** `()` - Parser error
2. **Unmatched Parentheses:** `(echo hello` - Parser error
3. **Variable Isolation:** Changes in subshell don't affect parent
4. **Exit Code Propagation:** Subshell exit code updates parent's `$?`
5. **Variable Inheritance:** Subshell inherits parent variables
6. **Function Inheritance:** Subshell inherits parent functions
7. **Alias Inheritance:** Subshell inherits parent aliases

#### Deferred to Phase 2:

1. **Nested Subshells:** `((echo nested))`
2. **Subshells in Pipelines:** `(echo hello) | grep hello`
3. **Subshells with Redirections:** `(echo hello) > output.txt`
4. **Subshells in Control Structures:** `if (test); then ...; fi`
5. **Command Substitution with Subshells:** `$((echo hello))`
6. **Background Subshells:** `(sleep 10) &`

### 8. Implementation Checklist

#### Lexer Changes
- [ ] No changes needed (already emits LeftParen/RightParen tokens)
- [ ] Verify existing parenthesis tokenization works correctly

#### Parser Changes
- [ ] Add `Subshell` variant to `Ast` enum in [`src/parser.rs`](src/parser.rs:3-52)
- [ ] Add subshell detection logic in `parse_commands_sequentially()` (before line 447)
- [ ] Ensure function definition detection takes precedence over subshell
- [ ] Add `parse_subshell()` helper function
- [ ] Add parser tests for basic subshell syntax

#### Executor Changes
- [ ] Add `Subshell` match arm in `execute()` function
- [ ] Implement `execute_subshell()` function
- [ ] Implement `clone_shell_state_for_subshell()` helper
- [ ] Add executor tests for subshell execution and isolation

#### State Management
- [ ] Verify `ShellState::clone()` works correctly
- [ ] Test that `Rc` and `Arc` fields clone properly
- [ ] Ensure file descriptor table isolation

#### Integration Testing
- [ ] Test subshell variable isolation
- [ ] Test subshell exit code propagation
- [ ] Test subshell inheritance of parent state
- [ ] Test interaction with existing features (assignments, control structures)

### 9. POSIX Compliance Notes

**POSIX Requirements for Subshells (IEEE Std 1003.1-2008, Section 2.9.4.1):**

1. ✅ Subshell executes in a separate environment
2. ✅ Changes to variables don't affect parent
3. ✅ Subshell inherits parent's variables
4. ✅ Exit status of subshell is exit status of last command
5. ⚠️ Subshell can be used in pipelines (Phase 2)
6. ⚠️ Subshell can have redirections (Phase 2)
7. ⚠️ Trap handlers are inherited (implemented via Arc sharing)

### 10. Testing Synchronization

**CRITICAL:** Tests that create subshells may need synchronization if they:
- Modify environment variables (use `ENV_LOCK`)
- Change current directory (use `DIR_CHANGE_LOCK`)
- Create temporary files (use unique timestamps)

**Example Test with Synchronization:**

```rust
#[test]
fn test_subshell_with_environment() {
    let _lock = ENV_LOCK.lock().unwrap();
    
    // Save original environment
    let original_home = std::env::var("HOME").ok();
    
    // Test logic here
    
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

### 11. Known Limitations in Phase 1

1. **No Nested Subshells:** `((echo nested))` will fail or behave incorrectly
2. **No Pipeline Integration:** `(echo hello) | grep hello` not supported
3. **No Redirection Support:** `(echo hello) > file` not supported
4. **No Background Execution:** `(sleep 10) &` not supported
5. **No Command Substitution:** `$((echo hello))` conflicts with arithmetic expansion
6. **No Logical Operators:** `(cmd1) && (cmd2)` may not work correctly

These limitations are acceptable for Phase 1 and will be addressed in subsequent phases.

### 12. Success Criteria

Phase 1 is complete when:

1. ✅ Parser recognizes basic subshell syntax: `(command)`
2. ✅ Parser correctly distinguishes subshells from function definitions
3. ✅ Executor creates isolated state for subshell execution
4. ✅ Variable changes in subshell don't affect parent
5. ✅ Subshell inherits parent variables, functions, and aliases
6. ✅ Subshell exit code propagates to parent's `$?`
7. ✅ All Phase 1 tests pass
8. ✅ No regressions in existing functionality

### 13. Example Usage After Phase 1

```bash
# Basic subshell
(echo "This runs in a subshell")

# Variable isolation
VAR=parent
(VAR=child; echo $VAR)  # Prints: child
echo $VAR               # Prints: parent

# Exit code propagation
(false)
echo $?                 # Prints: 1

# Multiple commands in subshell
(echo first; echo second; echo third)

# Subshell with control structures
(if true; then echo yes; fi)

# Subshell with variable inheritance
PARENT_VAR=value
(echo $PARENT_VAR)      # Prints: value
```

### 14. Migration Path

**Backward Compatibility:** Phase 1 changes are additive and don't break existing functionality:
- Function definitions still work (parser checks function pattern first)
- Case statements still work (handled in separate `parse_case()` function)
- Arithmetic expansion still works (handled in lexer/executor)
- Command substitution still works (handled in lexer/executor)

**Risk Mitigation:**
- Comprehensive test coverage before and after changes
- Parser precedence ensures function definitions take priority
- Isolated changes to specific functions minimize risk

### 15. Next Steps

After Phase 1 completion:
1. Review and test basic subshell functionality
2. Gather feedback on implementation approach
3. Proceed to Phase 2: Advanced Features (nested subshells, pipelines, redirections)
4. Proceed to Phase 3: Optimization and Edge Cases
