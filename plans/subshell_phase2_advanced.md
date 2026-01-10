# Subshell Implementation - Phase 2: Advanced Features

## Overview

Phase 2 builds upon the basic subshell support from Phase 1 to add advanced features including nested subshells, pipeline integration, redirections, and integration with control structures and logical operators.

**Prerequisites:** Phase 1 must be fully implemented and tested.

**Phase 2 Goals:**
- Support nested subshells: `((echo nested))`
- Support subshells in pipelines: `(echo hello) | grep hello`
- Support subshells with redirections: `(echo hello) > output.txt`
- Support subshells with logical operators: `(cmd1) && (cmd2)`
- Support subshells in control structures: `if (test); then ...; fi`
- Support command substitution with subshells: `$((echo hello))` - **CONFLICT RESOLUTION NEEDED**

**Out of Scope for Phase 2:**
- Background subshells `(cmd) &` (requires job control)
- Performance optimizations (deferred to Phase 3)
- Process-based subshells vs in-process (deferred to Phase 3)

## Architecture Analysis

### Current Pipeline Structure

**File:** [`src/executor.rs`](src/executor.rs:1410-1578)

The `execute_pipeline()` function:
- Iterates through commands in the pipeline
- Creates pipes between commands
- Handles stdin/stdout redirection
- Supports both built-ins and external commands

**Key Insight:** Subshells in pipelines need to be treated as single "commands" that can participate in the pipeline.

### Current Redirection Handling

**File:** [`src/executor.rs`](src/executor.rs:609-664)

The `apply_redirections()` function:
- Processes redirections in left-to-right order (POSIX requirement)
- Handles both built-in and external commands
- Supports file descriptor operations

**Key Insight:** Subshells need their own redirection handling that applies to the entire subshell's output.

### Current Logical Operator Handling

**File:** [`src/executor.rs`](src/executor.rs:1169-1200)

The `And` and `Or` AST variants:
- Execute left side first
- Conditionally execute right side based on left's exit code
- Properly propagate exit codes

**Key Insight:** Subshells should work seamlessly with `&&` and `||` operators.

## Phase 2 Design

### 1. Nested Subshells

#### Parser Enhancement

**File:** [`src/parser.rs`](src/parser.rs)

**Current Issue:** The Phase 1 parser uses simple depth tracking that doesn't distinguish between different types of parentheses.

**Solution:** The existing depth tracking in `parse_subshell()` already handles nested subshells correctly. The recursive call to `parse_commands_sequentially()` will detect inner subshells.

**Example:**
```bash
((echo nested))
```

**Token Stream:**
```
LeftParen, LeftParen, Word("echo"), Word("nested"), RightParen, RightParen
```

**Parsing Flow:**
1. Outer `parse_commands_sequentially()` detects first `LeftParen`
2. Finds matching `RightParen` at position 4 (depth tracking handles inner parens)
3. Extracts body: `LeftParen, Word("echo"), Word("nested"), RightParen`
4. Recursively calls `parse_commands_sequentially()` on body
5. Inner call detects `LeftParen` and creates inner `Subshell` AST
6. Returns nested structure: `Subshell { body: Subshell { body: Pipeline(...) } }`

**No Code Changes Needed:** Phase 1 implementation already supports this!

#### Executor Enhancement

**File:** [`src/executor.rs`](src/executor.rs)

**Current Issue:** None - the recursive `execute()` call in `execute_subshell()` naturally handles nested subshells.

**Verification Test:**

```rust
#[test]
fn test_nested_subshells() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("VAR", "parent".to_string());
    
    // Nested subshell: ((VAR=inner; echo $VAR))
    let inner_subshell = Ast::Subshell {
        body: Box::new(Ast::Sequence(vec![
            Ast::Assignment {
                var: "VAR".to_string(),
                value: "inner".to_string(),
            },
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "$VAR".to_string()],
                redirections: Vec::new(),
            }]),
        ])),
    };
    
    let outer_subshell = Ast::Subshell {
        body: Box::new(inner_subshell),
    };
    
    let exit_code = execute(outer_subshell, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Parent VAR should still be "parent"
    assert_eq!(shell_state.get_var("VAR"), Some("parent".to_string()));
}

#[test]
fn test_nested_subshells_three_levels() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("LEVEL", "0".to_string());
    
    // (((LEVEL=3)))
    let level3 = Ast::Subshell {
        body: Box::new(Ast::Assignment {
            var: "LEVEL".to_string(),
            value: "3".to_string(),
        }),
    };
    
    let level2 = Ast::Subshell {
        body: Box::new(level3),
    };
    
    let level1 = Ast::Subshell {
        body: Box::new(level2),
    };
    
    let exit_code = execute(level1, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Parent LEVEL should still be "0"
    assert_eq!(shell_state.get_var("LEVEL"), Some("0".to_string()));
}
```

### 2. Subshells in Pipelines

#### Challenge: Subshells as Pipeline Components

**Current Pipeline Structure:**
```rust
Ast::Pipeline(Vec<ShellCommand>)
```

**Problem:** `ShellCommand` only contains `args` and `redirections`. It cannot represent a subshell.

**Solution:** Extend the pipeline model to support compound commands.

#### Design Option A: Extend ShellCommand (Recommended)

**File:** [`src/parser.rs`](src/parser.rs:81-86)

**Change ShellCommand to support both simple and compound commands:**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellCommand {
    pub args: Vec<String>,
    pub redirections: Vec<Redirection>,
    /// Optional compound command (subshell, command group, etc.)
    /// If present, this takes precedence over args
    pub compound: Option<Box<Ast>>,
}
```

**Rationale:**
- Minimal change to existing structure
- Backward compatible (compound is `Option`)
- Allows any AST node in pipeline position
- Redirections can apply to compound commands

**Parser Changes:**

**File:** [`src/parser.rs`](src/parser.rs:630-744)

**Location:** In `parse_pipeline()` function

```rust
fn parse_pipeline(tokens: &[Token]) -> Result<Ast, String> {
    let mut commands = Vec::new();
    let mut current_cmd = ShellCommand::default();
    let mut i = 0;
    
    while i < tokens.len() {
        let token = &tokens[i];
        match token {
            Token::LeftParen => {
                // Start of subshell in pipeline
                // Find matching RightParen
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
                    return Err("Unmatched parenthesis in pipeline".to_string());
                }
                
                // Parse subshell body
                let subshell_tokens = &tokens[i + 1..j - 1];
                if subshell_tokens.is_empty() {
                    return Err("Empty subshell in pipeline".to_string());
                }
                
                let body_ast = parse_commands_sequentially(subshell_tokens)?;
                
                // Create ShellCommand with compound subshell
                current_cmd.compound = Some(Box::new(Ast::Subshell {
                    body: Box::new(body_ast),
                }));
                
                i = j; // Move past closing paren
                
                // Check for redirections after subshell
                while i < tokens.len() {
                    match &tokens[i] {
                        Token::RedirOut | Token::RedirIn | Token::RedirAppend => {
                            // Handle redirection
                            // (existing redirection parsing logic)
                            break;
                        }
                        Token::Pipe => {
                            // End of this pipeline stage
                            break;
                        }
                        _ => break,
                    }
                }
                
                // Push the command with subshell
                commands.push(current_cmd.clone());
                current_cmd = ShellCommand::default();
                
                continue;
            }
            Token::Word(word) => {
                current_cmd.args.push(word.clone());
            }
            Token::Pipe => {
                if !current_cmd.args.is_empty() || current_cmd.compound.is_some() {
                    commands.push(current_cmd.clone());
                    current_cmd = ShellCommand::default();
                }
            }
            // ... existing token handling ...
        }
        i += 1;
    }
    
    // ... rest of function ...
}
```

#### Executor Changes for Pipeline Subshells

**File:** [`src/executor.rs`](src/executor.rs:1410-1578)

**Location:** In `execute_pipeline()` function

```rust
fn execute_pipeline(commands: &[ShellCommand], shell_state: &mut ShellState) -> i32 {
    let mut exit_code = 0;
    let mut previous_stdout = None;
    
    for (i, cmd) in commands.iter().enumerate() {
        let is_last = i == commands.len() - 1;
        
        // Check if this is a compound command (subshell)
        if let Some(ref compound_ast) = cmd.compound {
            // Execute compound command (subshell) in pipeline
            exit_code = execute_compound_in_pipeline(
                compound_ast,
                shell_state,
                previous_stdout.take(),
                is_last,
                &cmd.redirections,
            );
            
            // For now, compound commands in pipelines don't produce stdout
            // This will be enhanced in Phase 3 with proper pipe handling
            previous_stdout = None;
            continue;
        }
        
        // Existing logic for simple commands
        if cmd.args.is_empty() {
            continue;
        }
        
        // ... existing pipeline logic ...
    }
    
    exit_code
}
```

#### Add Compound Command Pipeline Executor

**File:** [`src/executor.rs`](src/executor.rs)

**Location:** After `execute_subshell()` function

```rust
/// Execute a compound command (subshell) as part of a pipeline
/// 
/// # Arguments
/// * `compound_ast` - The compound command AST (typically Subshell)
/// * `shell_state` - The parent shell state
/// * `stdin` - Optional stdin from previous pipeline stage
/// * `is_last` - Whether this is the last command in the pipeline
/// * `redirections` - Redirections to apply to the compound command
/// 
/// # Returns
/// * Exit code from the compound command
fn execute_compound_in_pipeline(
    compound_ast: &Ast,
    shell_state: &mut ShellState,
    stdin: Option<Stdio>,
    is_last: bool,
    redirections: &[Redirection],
) -> i32 {
    match compound_ast {
        Ast::Subshell { body } => {
            // Clone state for subshell
            let mut subshell_state = clone_shell_state_for_subshell(shell_state);
            
            // Handle stdin from previous pipeline stage
            // For Phase 2, we'll use a simplified approach:
            // - If stdin is provided, we can't easily redirect it to the subshell
            // - This is a known limitation that will be addressed in Phase 3
            // - For now, log a warning and proceed
            if stdin.is_some() {
                eprintln!("Warning: stdin redirection to subshells in pipelines not fully supported");
            }
            
            // Handle stdout capture for next pipeline stage
            if !is_last || shell_state.capture_output.is_some() {
                // Need to capture subshell output
                let capture_buffer = Rc::new(RefCell::new(Vec::new()));
                subshell_state.capture_output = Some(capture_buffer.clone());
                
                // Execute subshell
                let exit_code = execute(*body.clone(), &mut subshell_state);
                
                // Transfer captured output to parent's capture buffer
                if let Some(ref parent_capture) = shell_state.capture_output {
                    let captured = capture_buffer.borrow().clone();
                    parent_capture.borrow_mut().extend_from_slice(&captured);
                }
                
                // Update parent's last_exit_code
                shell_state.last_exit_code = exit_code;
                
                exit_code
            } else {
                // Last command, no capture needed
                let exit_code = execute(*body.clone(), &mut subshell_state);
                shell_state.last_exit_code = exit_code;
                exit_code
            }
        }
        _ => {
            // Other compound commands not yet supported
            eprintln!("Unsupported compound command in pipeline");
            1
        }
    }
}
```

**Known Limitation:** Stdin redirection to subshells in pipelines is not fully supported in Phase 2. This requires process-based subshells (Phase 3).

### 3. Subshells with Redirections

#### Parser Enhancement

**File:** [`src/parser.rs`](src/parser.rs)

**Location:** In subshell detection logic (added in Phase 1)

**Enhancement:** After parsing subshell body, check for redirections:

```rust
// After creating subshell_ast in parse_commands_sequentially()
let subshell_ast = Ast::Subshell {
    body: Box::new(body_ast),
};

// Check for redirections after the closing paren
let mut redirections = Vec::new();
while i < tokens.len() {
    match &tokens[i] {
        Token::RedirOut => {
            i += 1;
            if i < tokens.len() {
                if let Token::Word(file) = &tokens[i] {
                    redirections.push(Redirection::Output(file.clone()));
                    i += 1;
                }
            }
        }
        Token::RedirIn => {
            i += 1;
            if i < tokens.len() {
                if let Token::Word(file) = &tokens[i] {
                    redirections.push(Redirection::Input(file.clone()));
                    i += 1;
                }
            }
        }
        Token::RedirAppend => {
            i += 1;
            if i < tokens.len() {
                if let Token::Word(file) = &tokens[i] {
                    redirections.push(Redirection::Append(file.clone()));
                    i += 1;
                }
            }
        }
        // Handle all other redirection types...
        _ => break,
    }
}

// If redirections found, wrap subshell in a pipeline with redirections
let final_ast = if !redirections.is_empty() {
    // Create a ShellCommand that wraps the subshell
    Ast::Pipeline(vec![ShellCommand {
        args: Vec::new(),
        redirections,
        compound: Some(Box::new(subshell_ast)),
    }])
} else {
    subshell_ast
};

commands.push(final_ast);
```

**Alternative Design:** Add redirections field to Subshell AST variant:

```rust
Ast::Subshell {
    body: Box<Ast>,
    redirections: Vec<Redirection>,
}
```

**Recommendation:** Use the ShellCommand wrapper approach (first option) because:
- Reuses existing redirection infrastructure
- Consistent with how redirections work elsewhere
- No need to duplicate redirection handling logic

#### Executor Enhancement

**File:** [`src/executor.rs`](src/executor.rs)

**Location:** In `execute_subshell()` function

**Enhancement:** Handle redirections if subshell is wrapped in Pipeline:

```rust
fn execute_subshell(body: Ast, shell_state: &mut ShellState) -> i32 {
    // Clone the shell state for isolation
    let mut subshell_state = clone_shell_state_for_subshell(shell_state);
    
    // If capturing output, set up capture buffer
    if shell_state.capture_output.is_some() {
        let capture_buffer = Rc::new(RefCell::new(Vec::new()));
        subshell_state.capture_output = Some(capture_buffer.clone());
        
        // Execute the body
        let exit_code = execute(body, &mut subshell_state);
        
        // Transfer captured output to parent
        if let Some(ref parent_capture) = shell_state.capture_output {
            let captured = capture_buffer.borrow().clone();
            parent_capture.borrow_mut().extend_from_slice(&captured);
        }
        
        shell_state.last_exit_code = exit_code;
        exit_code
    } else {
        // Normal execution
        let exit_code = execute(body, &mut subshell_state);
        shell_state.last_exit_code = exit_code;
        exit_code
    }
}
```

**Note:** Redirections are handled by the Pipeline wrapper, not directly in `execute_subshell()`.

### 4. Subshells with Logical Operators

#### Parser Enhancement

**File:** [`src/parser.rs`](src/parser.rs:580-608)

**Current Logic:** The parser already handles `&&` and `||` after any command.

**Verification:** The existing logic in `parse_commands_sequentially()` should work:

```rust
// After parsing subshell
let ast = parse_slice(command_tokens)?;  // This could be a Subshell

// Check if the next token is && or ||
if i < tokens.len() && (tokens[i] == Token::And || tokens[i] == Token::Or) {
    let operator = tokens[i].clone();
    i += 1;
    
    // Parse the right side recursively
    let remaining_tokens = &tokens[i..];
    let right_ast = parse_commands_sequentially(remaining_tokens)?;
    
    // Create And or Or node
    let combined_ast = match operator {
        Token::And => Ast::And {
            left: Box::new(ast),
            right: Box::new(right_ast),
        },
        Token::Or => Ast::Or {
            left: Box::new(ast),
            right: Box::new(right_ast),
        },
        _ => unreachable!(),
    };
    
    commands.push(combined_ast);
    break;
}
```

**No Changes Needed:** The existing logic already supports subshells with logical operators!

#### Test Cases

```rust
#[test]
fn test_subshell_with_and_operator() {
    let mut shell_state = ShellState::new();
    
    // (true) && echo "success"
    let ast = Ast::And {
        left: Box::new(Ast::Subshell {
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["true".to_string()],
                redirections: Vec::new(),
                compound: None,
            }])),
        }),
        right: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["echo".to_string(), "success".to_string()],
            redirections: Vec::new(),
            compound: None,
        }])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_subshell_with_or_operator() {
    let mut shell_state = ShellState::new();
    
    // (false) || echo "fallback"
    let ast = Ast::Or {
        left: Box::new(Ast::Subshell {
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["false".to_string()],
                redirections: Vec::new(),
                compound: None,
            }])),
        }),
        right: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["echo".to_string(), "fallback".to_string()],
            redirections: Vec::new(),
            compound: None,
        }])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_chained_subshells_with_operators() {
    let mut shell_state = ShellState::new();
    
    // (true) && (echo middle) && (echo end)
    let inner_and = Ast::And {
        left: Box::new(Ast::Subshell {
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "middle".to_string()],
                redirections: Vec::new(),
                compound: None,
            }])),
        }),
        right: Box::new(Ast::Subshell {
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "end".to_string()],
                redirections: Vec::new(),
                compound: None,
            }])),
        }),
    };
    
    let outer_and = Ast::And {
        left: Box::new(Ast::Subshell {
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["true".to_string()],
                redirections: Vec::new(),
                compound: None,
            }])),
        }),
        right: Box::new(inner_and),
    };
    
    let exit_code = execute(outer_and, &mut shell_state);
    assert_eq!(exit_code, 0);
}
```

### 5. Subshells in Control Structures

#### If Statement Conditions

**Example:**
```bash
if (test -f file.txt); then
    echo "File exists"
fi
```

**Current Support:** The parser's `parse_if()` function (line 746-911) calls `parse_slice()` for conditions, which will automatically handle subshells.

**No Changes Needed:** Already supported by Phase 1 implementation!

#### While Loop Conditions

**Example:**
```bash
while (test $count -lt 10); do
    count=$((count + 1))
done
```

**Current Support:** The parser's `parse_while()` function (line 1127-1224) calls `parse_slice()` for conditions.

**No Changes Needed:** Already supported by Phase 1 implementation!

#### For Loop Items

**Example:**
```bash
for item in $(echo a b c); do
    echo $item
done
```

**Note:** This is command substitution, not subshell. Already supported.

**Subshell in For Body:**
```bash
for item in a b c; do
    (echo "Processing: $item")
done
```

**Current Support:** The parser's `parse_for()` function (line 1013-1125) calls `parse_commands_sequentially()` for the body.

**No Changes Needed:** Already supported by Phase 1 implementation!

### 6. Command Substitution Conflict Resolution

#### Problem: Syntax Ambiguity

**Conflict:**
- Arithmetic expansion: `$((2 + 3))` - Currently supported
- Command substitution: `$(echo hello)` - Currently supported
- Subshell in command substitution: `$((echo hello))` - **AMBIGUOUS**

**POSIX Specification:**
- `$((...))` is arithmetic expansion
- `$(...)` is command substitution
- `$((cmd))` should be arithmetic expansion (two opening parens)

**Resolution Strategy:**

The lexer already handles this correctly (lines 531-558):
1. Sees `$(`
2. Checks next character
3. If next is `(`, treats as arithmetic: `$((...))` 
4. Otherwise, treats as command substitution: `$(...)`

**Therefore:** `$((echo hello))` is parsed as arithmetic expansion with content `(echo hello)`, which will fail arithmetic evaluation.

**Correct Syntax:**
- Subshell in command substitution: `$( (echo hello) )` - space after `$(`
- Arithmetic expansion: `$((2 + 3))` - no space

**Implementation:** No changes needed. Document this behavior clearly.

### 7. Subshell State Isolation Details

#### What Gets Isolated (Cloned)

1. **Variables** (`variables: HashMap`) - ✅ Cloned
2. **Exported Variables** (`exported: HashSet`) - ✅ Cloned
3. **Positional Parameters** (`positional_params: Vec`) - ✅ Cloned
4. **Functions** (`functions: HashMap`) - ✅ Cloned
5. **Aliases** (`aliases: HashMap`) - ✅ Cloned
6. **Directory Stack** (`dir_stack: Vec`) - ✅ Cloned
7. **Local Variable Scopes** (`local_vars: Vec`) - ✅ Cloned
8. **File Descriptor Table** - ✅ New instance created

#### What Gets Shared (Not Isolated)

1. **Trap Handlers** (`trap_handlers: Arc<Mutex<HashMap>>`) - ✅ Shared via Arc
   - POSIX: Subshells inherit trap handlers
   - Changes in subshell don't affect parent (Arc provides shared read)
   
2. **Capture Output Buffer** (`capture_output: Option<Rc<RefCell<Vec<u8>>>>`) - ✅ Shared via Rc
   - Allows command substitution: `$(( subshell ))`
   - Subshell output goes to parent's capture buffer

#### What Gets Reset

1. **Return State** (`returning: bool`, `return_value: Option<i32>`) - ✅ Reset to false/None
2. **Exit State** (`exit_requested: bool`, `exit_code: i32`) - ✅ Reset to false/0
3. **Heredoc State** (`pending_heredoc_content`, `collecting_heredoc`) - ✅ Reset to None

### 8. Integration Test Scenarios

#### Test File

**File:** [`src/executor.rs`](src/executor.rs) (test module)

```rust
#[test]
fn test_subshell_in_pipeline() {
    let mut shell_state = ShellState::new();
    
    // (echo hello) | grep hello
    let commands = vec![
        ShellCommand {
            args: Vec::new(),
            redirections: Vec::new(),
            compound: Some(Box::new(Ast::Subshell {
                body: Box::new(Ast::Pipeline(vec![ShellCommand {
                    args: vec!["echo".to_string(), "hello".to_string()],
                    redirections: Vec::new(),
                    compound: None,
                }])),
            })),
        },
        ShellCommand {
            args: vec!["grep".to_string(), "hello".to_string()],
            redirections: Vec::new(),
            compound: None,
        },
    ];
    
    let exit_code = execute_pipeline(&commands, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_subshell_with_output_redirection() {
    let _lock = ENV_LOCK.lock().unwrap();
    
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/rush_test_subshell_redir_{}.txt", timestamp);
    
    let mut shell_state = ShellState::new();
    
    // (echo hello) > output.txt
    let ast = Ast::Pipeline(vec![ShellCommand {
        args: Vec::new(),
        redirections: vec![Redirection::Output(temp_file.clone())],
        compound: Some(Box::new(Ast::Subshell {
            body: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                redirections: Vec::new(),
                compound: None,
            }])),
        })),
    }]);
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Verify output file
    let content = std::fs::read_to_string(&temp_file).unwrap();
    assert!(content.contains("hello"));
    
    // Cleanup
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_subshell_in_if_condition() {
    let mut shell_state = ShellState::new();
    
    // if (true); then echo yes; fi
    let ast = Ast::If {
        branches: vec![(
            Box::new(Ast::Subshell {
                body: Box::new(Ast::Pipeline(vec![ShellCommand {
                    args: vec!["true".to_string()],
                    redirections: Vec::new(),
                    compound: None,
                }])),
            }),
            Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "yes".to_string()],
                redirections: Vec::new(),
                compound: None,
            }])),
        )],
        else_branch: None,
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_subshell_with_cd_isolation() {
    let _lock = DIR_CHANGE_LOCK.lock().unwrap();
    
    let mut shell_state = ShellState::new();
    let original_dir = std::env::current_dir().unwrap();
    
    // (cd /tmp; pwd)
    // The cd should not affect parent's directory
    let subshell_ast = Ast::Subshell {
        body: Box::new(Ast::Sequence(vec![
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["cd".to_string(), "/tmp".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["pwd".to_string()],
                redirections: Vec::new(),
                compound: None,
            }]),
        ])),
    };
    
    let exit_code = execute(subshell_ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Parent directory should be unchanged
    let current_dir = std::env::current_dir().unwrap();
    assert_eq!(current_dir, original_dir);
}
```

### 9. ShellCommand Structure Changes

#### Update ShellCommand Definition

**File:** [`src/parser.rs`](src/parser.rs:81-86)

**Current:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellCommand {
    pub args: Vec<String>,
    pub redirections: Vec<Redirection>,
}
```

**New:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellCommand {
    pub args: Vec<String>,
    pub redirections: Vec<Redirection>,
    /// Optional compound command (subshell, command group, etc.)
    /// If present, this takes precedence over args
    pub compound: Option<Box<Ast>>,
}
```

**Impact Analysis:**
- All existing code that creates `ShellCommand` will continue to work (compound defaults to None)
- Pattern matching on `ShellCommand` needs to check `compound` field
- Executor needs to handle compound commands in pipelines

#### Update All ShellCommand Instantiations

**Files to Update:**
- [`src/parser.rs`](src/parser.rs) - All `ShellCommand` creations
- [`src/executor.rs`](src/executor.rs) - All `ShellCommand` creations
- Test files - All `ShellCommand` test instances

**Pattern:**
```rust
// Old
ShellCommand {
    args: vec!["echo".to_string()],
    redirections: Vec::new(),
}

// New
ShellCommand {
    args: vec!["echo".to_string()],
    redirections: Vec::new(),
    compound: None,
}
```

**Automation:** Use `Default::default()` where possible:
```rust
ShellCommand {
    args: vec!["echo".to_string()],
    ..Default::default()
}
```

### 10. Executor Pipeline Logic Updates

#### Update execute_single_command

**File:** [`src/executor.rs`](src/executor.rs:1204-1408)

**Location:** At the start of `execute_single_command()` function

```rust
fn execute_single_command(cmd: &ShellCommand, shell_state: &mut ShellState) -> i32 {
    // Check if this is a compound command
    if let Some(ref compound_ast) = cmd.compound {
        // Execute compound command with redirections
        return execute_compound_with_redirections(
            compound_ast,
            shell_state,
            &cmd.redirections,
        );
    }
    
    // Existing logic for simple commands
    if cmd.args.is_empty() {
        // ... existing logic ...
    }
    
    // ... rest of function ...
}
```

#### Add Compound Command Executor with Redirections

**File:** [`src/executor.rs`](src/executor.rs)

**Location:** After `execute_compound_in_pipeline()` function

```rust
/// Execute a compound command with redirections
/// 
/// # Arguments
/// * `compound_ast` - The compound command AST
/// * `shell_state` - The shell state
/// * `redirections` - Redirections to apply
/// 
/// # Returns
/// * Exit code from the compound command
fn execute_compound_with_redirections(
    compound_ast: &Ast,
    shell_state: &mut ShellState,
    redirections: &[Redirection],
) -> i32 {
    match compound_ast {
        Ast::Subshell { body } => {
            // For subshells with redirections, we need to:
            // 1. Set up output capture if there are output redirections
            // 2. Execute the subshell
            // 3. Apply the redirections to the captured output
            
            // Check if we have output redirections
            let has_output_redir = redirections.iter().any(|r| {
                matches!(
                    r,
                    Redirection::Output(_)
                        | Redirection::Append(_)
                        | Redirection::FdOutput(_, _)
                        | Redirection::FdAppend(_, _)
                )
            });
            
            if has_output_redir {
                // Clone state for subshell
                let mut subshell_state = clone_shell_state_for_subshell(shell_state);
                
                // Set up output capture
                let capture_buffer = Rc::new(RefCell::new(Vec::new()));
                subshell_state.capture_output = Some(capture_buffer.clone());
                
                // Execute subshell
                let exit_code = execute(*body.clone(), &mut subshell_state);
                
                // Get captured output
                let output = capture_buffer.borrow().clone();
                
                // Apply redirections to output
                // For Phase 2, we'll write the output to the redirected files
                for redir in redirections {
                    match redir {
                        Redirection::Output(file) => {
                            let expanded_file = expand_variables_in_string(file, shell_state);
                            if let Err(e) = std::fs::write(&expanded_file, &output) {
                                if shell_state.colors_enabled {
                                    eprintln!(
                                        "{}Redirection error: {}\x1b[0m",
                                        shell_state.color_scheme.error, e
                                    );
                                } else {
                                    eprintln!("Redirection error: {}", e);
                                }
                                return 1;
                            }
                        }
                        Redirection::Append(file) => {
                            let expanded_file = expand_variables_in_string(file, shell_state);
                            use std::fs::OpenOptions;
                            let mut file_handle = match OpenOptions::new()
                                .append(true)
                                .create(true)
                                .open(&expanded_file)
                            {
                                Ok(f) => f,
                                Err(e) => {
                                    if shell_state.colors_enabled {
                                        eprintln!(
                                            "{}Redirection error: {}\x1b[0m",
                                            shell_state.color_scheme.error, e
                                        );
                                    } else {
                                        eprintln!("Redirection error: {}", e);
                                    }
                                    return 1;
                                }
                            };
                            if let Err(e) = file_handle.write_all(&output) {
                                if shell_state.colors_enabled {
                                    eprintln!(
                                        "{}Redirection error: {}\x1b[0m",
                                        shell_state.color_scheme.error, e
                                    );
                                } else {
                                    eprintln!("Redirection error: {}", e);
                                }
                                return 1;
                            }
                        }
                        // Handle other redirection types...
                        _ => {
                            // For Phase 2, only support basic output redirections
                            eprintln!("Unsupported redirection type for subshell");
                            return 1;
                        }
                    }
                }
                
                shell_state.last_exit_code = exit_code;
                exit_code
            } else {
                // No output redirections, execute normally
                execute_subshell(*body.clone(), shell_state)
            }
        }
        _ => {
            eprintln!("Unsupported compound command type");
            1
        }
    }
}
```

### 11. Phase 2 Implementation Checklist

#### Parser Changes
- [ ] Add `compound: Option<Box<Ast>>` field to `ShellCommand` struct
- [ ] Update all `ShellCommand` instantiations to include `compound: None`
- [ ] Add subshell detection in `parse_pipeline()` function
- [ ] Add redirection parsing after subshell closing paren
- [ ] Add tests for subshells in pipelines
- [ ] Add tests for subshells with redirections
- [ ] Add tests for nested subshells
- [ ] Verify logical operators work with subshells (should already work)
- [ ] Verify control structures work with subshells (should already work)

#### Executor Changes
- [ ] Update `execute_single_command()` to check for compound commands
- [ ] Implement `execute_compound_in_pipeline()` function
- [ ] Implement `execute_compound_with_redirections()` function
- [ ] Update `execute_pipeline()` to handle compound commands
- [ ] Add tests for subshell pipeline integration
- [ ] Add tests for subshell redirections
- [ ] Add tests for nested subshell execution
- [ ] Add tests for subshells with cd isolation

#### Integration Testing
- [ ] Test subshells in if conditions
- [ ] Test subshells in while conditions
- [ ] Test subshells in for loop bodies
- [ ] Test subshells with && and || operators
- [ ] Test complex scenarios combining multiple features

### 12. Known Limitations in Phase 2

1. **Stdin Redirection in Pipelines:** `echo hello | (cat)` - stdin not properly connected
   - Requires process-based subshells (Phase 3)
   - Workaround: Use command substitution instead

2. **Complex Redirection Scenarios:** `(cmd) 2>&1 | grep error`
   - File descriptor duplication with subshells not fully supported
   - Will be addressed in Phase 3

3. **Background Subshells:** `(sleep 10) &`
   - Requires job control implementation
   - Out of scope for subshell feature

4. **Performance:** In-process subshells may have performance implications
   - State cloning overhead
   - Will be optimized in Phase 3

### 13. Success Criteria

Phase 2 is complete when:

1. ✅ Nested subshells work correctly: `((echo nested))`
2. ✅ Subshells work in pipelines: `(echo hello) | grep hello`
3. ✅ Subshells work with output redirections: `(echo hello) > file`
4. ✅ Subshells work with logical operators: `(true) && echo yes`
5. ✅ Subshells work in control structure conditions
6. ✅ All Phase 2 tests pass
7. ✅ No regressions in Phase 1 or existing functionality
8. ✅ Documentation updated with examples and limitations

### 14. Example Usage After Phase 2

```bash
# Nested subshells
((echo "Level 2 nested"))
(((echo "Level 3 nested")))

# Subshells in pipelines
(echo "hello world") | grep hello

# Subshells with redirections
(echo "output"; echo "error" >&2) > output.txt 2> error.txt

# Subshells with logical operators
(test -f file.txt) && echo "File exists" || echo "File not found"

# Subshells in control structures
if (test -d /tmp); then
    echo "Directory exists"
fi

while (test $count -lt 10); do
    count=$((count + 1))
    (echo "Count: $count")
done

# Complex combinations
(cd /tmp; ls -la) | grep "\.txt$" > /tmp/txt_files.list

# Variable isolation with pipelines
VAR=parent
(VAR=child; echo $VAR) | cat  # Outputs: child
echo $VAR                      # Outputs: parent
```

### 15. Migration from Phase 1 to Phase 2

**Breaking Changes:** None - Phase 2 is purely additive.

**New Capabilities:**
- Subshells can now participate in pipelines
- Subshells can have redirections
- Nested subshells work correctly
- All combinations with existing features work

**Testing Strategy:**
1. Run all Phase 1 tests to ensure no regressions
2. Add Phase 2 tests incrementally
3. Test each new feature in isolation before combining
4. Add integration tests for complex scenarios

### 16. Performance Considerations

**State Cloning Overhead:**
- Each subshell clones the entire `ShellState`
- For large variable sets or many functions, this could be expensive
- Nested subshells multiply the overhead

**Mitigation Strategies (for Phase 3):**
- Copy-on-write (COW) semantics for variables
- Shared immutable data structures
- Process-based subshells for expensive operations

**Measurement:**
- Add benchmarks for subshell execution
- Compare with bash performance
- Identify optimization opportunities

### 17. Documentation Updates

**Files to Update:**
- [`README.md`](README.md) - Add subshell feature to feature list
- [`TODO.md`](TODO.md) - Mark subshells as implemented
- [`AGENTS.md`](AGENTS.md) - Add subshell architecture notes
- [`docs/features.html`](docs/features.html) - Add subshell documentation

**Example Documentation:**

```markdown
## Subshells

Rush supports POSIX-compliant subshells using parentheses syntax:

### Basic Usage
\`\`\`bash
(command)
\`\`\`

### Features
- Variable isolation: Changes in subshell don't affect parent
- Inheritance: Subshells inherit parent variables, functions, and aliases
- Exit codes: Subshell exit code updates parent's $?
- Nesting: Subshells can be nested to any depth
- Pipelines: Subshells can participate in pipelines
- Redirections: Subshell output can be redirected

### Examples
\`\`\`bash
# Variable isolation
VAR=parent
(VAR=child; echo $VAR)  # Prints: child
echo $VAR               # Prints: parent

# Directory isolation
(cd /tmp; pwd)          # Prints: /tmp
pwd                     # Prints: original directory

# In pipelines
(echo hello; echo world) | grep world

# With redirections
(echo output) > file.txt
\`\`\`

### Limitations
- Background subshells require job control (not yet implemented)
- Some complex redirection scenarios may not work as expected
\`\`\`
```

### 18. Next Steps

After Phase 2 completion:
1. Comprehensive testing of all advanced features
2. Performance profiling and benchmarking
3. User feedback and bug fixes
4. Proceed to Phase 3: Optimization and Edge Cases
