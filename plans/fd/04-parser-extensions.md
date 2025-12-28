# Parser Extensions for FD Operations

## Overview

This document describes the enhancements needed in the parser to support file descriptor (FD) operations. The parser must parse FD-specific tokens from the lexer and construct appropriate AST nodes.

## Current Parser State

### Existing ShellCommand Structure

The current `ShellCommand` struct (lines 54-63 in `src/parser.rs`) has basic redirection fields:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellCommand {
    pub args: Vec<String>,
    pub input: Option<String>,           // < file
    pub output: Option<String>,          // > file
    pub append: Option<String>,           // >> file
    pub here_doc_delimiter: Option<String>,  // << EOF
    pub here_doc_quoted: bool,           // <<'EOF' or <<"EOF"
    pub here_string_content: Option<String>, // <<< "content"
}
```

### Current Limitations

1. **No FD-specific fields**: Cannot specify which FD to redirect
2. **No FD duplication**: Cannot represent FD duplication operations
3. **No FD closing**: Cannot represent FD closing operations
4. **No FD here-documents**: Cannot specify FD for here-documents
5. **No FD here-strings**: Cannot specify FD for here-strings

## Proposed Data Structures

### FdRedirection Enum

```rust
/// Represents a file descriptor redirection operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FdRedirection {
    /// Redirect FD to file: [n]>word or [n]>>word
    RedirectToFile {
        fd: i32,
        path: String,
        append: bool,
    },
    
    /// Duplicate FD for writing: [n]>&[m]
    DuplicateFdWrite {
        dest_fd: i32,
        src_fd: i32,
    },
    
    /// Duplicate FD for reading: [n]<&[m]
    DuplicateFdRead {
        dest_fd: i32,
        src_fd: i32,
    },
    
    /// Close FD: [n]>&- or [n]<&-
    CloseFd {
        fd: i32,
    },
    
    /// Here-document for FD: [n]<<[-]word
    HereDoc {
        fd: i32,
        delimiter: String,
        strip_tabs: bool,  // <<-EOF
    },
    
    /// Here-string for FD: [n]<<<word
    HereString {
        fd: i32,
        content: String,
    },
}
```

### Enhanced ShellCommand

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellCommand {
    pub args: Vec<String>,
    
    // Basic redirections (unchanged for backward compatibility)
    pub input: Option<String>,           // < file (defaults to FD 0)
    pub output: Option<String>,          // > file (defaults to FD 1)
    pub append: Option<String>,           // >> file (defaults to FD 1)
    pub here_doc_delimiter: Option<String>, // << EOF (defaults to FD 0)
    pub here_doc_quoted: bool,           // <<'EOF' or <<"EOF"
    pub here_string_content: Option<String>, // <<< "content" (defaults to FD 0)
    
    // New FD-specific redirections
    pub fd_redirections: Vec<FdRedirection>,
}
```

## Parser Enhancements

### 1. Pipeline Parsing with FD Operations

```rust
fn parse_pipeline(tokens: &[Token]) -> Result<Ast, String> {
    let mut commands = Vec::new();
    let mut current_cmd = ShellCommand::default();
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];
        match token {
            Token::Word(word) => {
                current_cmd.args.push(word.clone());
            }
            
            Token::Pipe => {
                if !current_cmd.args.is_empty() {
                    commands.push(current_cmd.clone());
                    current_cmd = ShellCommand::default();
                }
            }
            
            // Handle FD redirection tokens
            Token::RedirFdOut(fd, append) => {
                current_cmd.fd_redirections.push(FdRedirection::RedirectToFile {
                    fd: *fd,
                    path: extract_next_word(tokens, &mut i)?,
                    append: *append,
                });
            }
            
            Token::RedirFdIn(fd) => {
                current_cmd.fd_redirections.push(FdRedirection::RedirectToFile {
                    fd: *fd,
                    path: extract_next_word(tokens, &mut i)?,
                    append: false,
                });
            }
            
            Token::RedirFdDupWrite(dest_fd, src_fd) => {
                if let Some(src) = src_fd {
                    current_cmd.fd_redirections.push(FdRedirection::DuplicateFdWrite {
                        dest_fd: *dest_fd,
                        src_fd: src,
                    });
                } else {
                    // No source FD means close
                    current_cmd.fd_redirections.push(FdRedirection::CloseFd {
                        fd: *dest_fd,
                    });
                }
            }
            
            Token::RedirFdDupRead(dest_fd, src_fd) => {
                if let Some(src) = src_fd {
                    current_cmd.fd_redirections.push(FdRedirection::DuplicateFdRead {
                        dest_fd: *dest_fd,
                        src_fd: src,
                    });
                } else {
                    // No source FD means close
                    current_cmd.fd_redirections.push(FdRedirection::CloseFd {
                        fd: *dest_fd,
                    });
                }
            }
            
            Token::RedirFdClose(fd) => {
                current_cmd.fd_redirections.push(FdRedirection::CloseFd {
                    fd: *fd,
                });
            }
            
            Token::RedirFdHereDoc(fd, delimiter, strip_tabs) => {
                current_cmd.fd_redirections.push(FdRedirection::HereDoc {
                    fd: *fd,
                    delimiter: delimiter.clone(),
                    strip_tabs: *strip_tabs,
                });
            }
            
            Token::RedirFdHereString(fd, content) => {
                current_cmd.fd_redirections.push(FdRedirection::HereString {
                    fd: *fd,
                    content: content.clone(),
                });
            }
            
            // ... existing token handling ...
            
            _ => {
                // Handle other tokens
            }
        }
        
        i += 1;
    }

    if !current_cmd.args.is_empty() || !current_cmd.fd_redirections.is_empty() {
        commands.push(current_cmd);
    }

    if commands.is_empty() {
        return Err("No commands found".to_string());
    }

    Ok(Ast::Pipeline(commands))
}
```

### 2. Helper Functions

```rust
/// Extract the next word token from the token stream
/// 
/// This is used to get file paths for FD redirections.
/// 
/// # Arguments
/// * `tokens` - The token stream
/// * `i` - Current position in token stream (will be updated)
/// 
/// # Returns
/// * The next word token as a string
/// 
/// # Errors
/// * `Err` if no word token is found
fn extract_next_word(tokens: &[Token], i: &mut usize) -> Result<String, String> {
    // Skip any whitespace tokens (if we had them)
    while *i < tokens.len() {
        match &tokens[*i] {
            Token::Word(word) => {
                *i += 1;
                return Ok(word.clone());
            }
            _ => {
                *i += 1;
            }
        }
    }
    
    Err("Expected word token after FD redirection".to_string())
}
```

### 3. FD Redirection Validation

```rust
/// Validate FD redirections for conflicts and errors
/// 
/// # Arguments
/// * `redirections` - The FD redirections to validate
/// 
/// # Returns
/// * `Ok(())` if redirections are valid
/// * `Err` if redirections have conflicts or errors
fn validate_fd_redirections(redirections: &[FdRedirection]) -> Result<(), String> {
    // Check for duplicate FD operations on same FD
    let mut fd_targets = HashMap::new();
    
    for redir in redirections {
        match redir {
            FdRedirection::RedirectToFile { fd, .. } => {
                if fd_targets.contains_key(&fd) {
                    return Err(format!("FD {} is redirected multiple times", fd));
                }
                fd_targets.insert(fd, ());
            }
            
            FdRedirection::DuplicateFdWrite { dest_fd, .. } => {
                if fd_targets.contains_key(&dest_fd) {
                    return Err(format!("FD {} is redirected multiple times", dest_fd));
                }
                fd_targets.insert(dest_fd, ());
            }
            
            FdRedirection::DuplicateFdRead { dest_fd, .. } => {
                if fd_targets.contains_key(&dest_fd) {
                    return Err(format!("FD {} is redirected multiple times", dest_fd));
                }
                fd_targets.insert(dest_fd, ());
            }
            
            FdRedirection::CloseFd { fd } => {
                if fd_targets.contains_key(&fd) {
                    return Err(format!("FD {} is redirected and closed", fd));
                }
                fd_targets.insert(fd, ());
            }
            
            FdRedirection::HereDoc { fd, .. } => {
                if fd_targets.contains_key(&fd) {
                    return Err(format!("FD {} is redirected multiple times", fd));
                }
                fd_targets.insert(fd, ());
            }
            
            FdRedirection::HereString { fd, .. } => {
                if fd_targets.contains_key(&fd) {
                    return Err(format!("FD {} is redirected multiple times", fd));
                }
                fd_targets.insert(fd, ());
            }
        }
    }
    
    Ok(())
}
```

## Parsing Examples

### Example 1: Basic FD Redirection

```bash
# Input: "echo hello 2>error.txt"
# Tokens:
#   Word("echo")
#   Word("hello")
#   RedirFdOut(2, false)
#   Word("error.txt")
# 
# AST:
#   Pipeline([
#     ShellCommand {
#         args: ["echo", "hello"],
#         fd_redirections: [
#             RedirectToFile { fd: 2, path: "error.txt", append: false }
#         ]
#     }
#   ])
```

### Example 2: FD Duplication

```bash
# Input: "command 2>&1"
# Tokens:
#   Word("command")
#   RedirFdDupWrite(2, Some(1))
# 
# AST:
#   Pipeline([
#     ShellCommand {
#         args: ["command"],
#         fd_redirections: [
#             DuplicateFdWrite { dest_fd: 2, src_fd: 1 }
#         ]
#     }
#   ])
```

### Example 3: FD Closing

```bash
# Input: "command 2>&-"
# Tokens:
#   Word("command")
#   RedirFdClose(2)
# 
# AST:
#   Pipeline([
#     ShellCommand {
#         args: ["command"],
#         fd_redirections: [
#             CloseFd { fd: 2 }
#         ]
#     }
#   ])
```

### Example 4: Multiple FD Operations

```bash
# Input: "command 1>out.txt 2>&1"
# Tokens:
#   Word("command")
#   RedirFdOut(1, false)
#   Word("out.txt")
#   RedirFdDupWrite(2, Some(1))
# 
# AST:
#   Pipeline([
#     ShellCommand {
#         args: ["command"],
#         fd_redirections: [
#             RedirectToFile { fd: 1, path: "out.txt", append: false },
#             DuplicateFdWrite { dest_fd: 2, src_fd: 1 }
#         ]
#     }
#   ])
```

### Example 5: FD Here-Document

```bash
# Input: "cat 3<<EOF"
# Tokens:
#   Word("cat")
#   RedirFdHereDoc(3, "EOF", false)
# 
# AST:
#   Pipeline([
#     ShellCommand {
#         args: ["cat"],
#         fd_redirections: [
#             HereDoc { fd: 3, delimiter: "EOF", strip_tabs: false }
#         ]
#     }
#   ])
```

### Example 6: FD Here-String

```bash
# Input: "grep 3<<<pattern"
# Tokens:
#   Word("grep")
#   RedirFdHereString(3, "pattern")
# 
# AST:
#   Pipeline([
#     ShellCommand {
#         args: ["grep"],
#         fd_redirections: [
#             HereString { fd: 3, content: "pattern" }
#         ]
#     }
#   ])
```

### Example 7: Mixed Redirections

```bash
# Input: "command <input.txt 2>error.txt"
# Tokens:
#   Word("command")
#   RedirIn  # stdin from file (no FD specified)
#   Word("input.txt")
#   RedirFdOut(2, false)
#   Word("error.txt")
# 
# AST:
#   Pipeline([
#     ShellCommand {
#         args: ["command"],
#         input: Some("input.txt"),  # Basic redirection (FD 0)
#         fd_redirections: [
#             RedirectToFile { fd: 2, path: "error.txt", append: false }
#         ]
#     }
#   ])
```

## Edge Cases

### 1. Conflicting FD Operations

```bash
# Invalid: "command 1>file.txt 1>other.txt"
# Error: FD 1 is redirected multiple times
```

### 2. FD Duplication Chain

```bash
# Valid: "command 3>&2 2>&1"
# Tokens:
#   Word("command")
#   RedirFdDupWrite(3, Some(2))
#   RedirFdDupWrite(2, Some(1))
# 
# AST:
#   Pipeline([
#     ShellCommand {
#         args: ["command"],
#         fd_redirections: [
#             DuplicateFdWrite { dest_fd: 3, src_fd: 2 },
#             DuplicateFdWrite { dest_fd: 2, src_fd: 1 }
#         ]
#     }
#   ])
```

### 3. FD Closing After Redirection

```bash
# Invalid: "command 1>file.txt 1>&-"
# Error: FD 1 is redirected and closed
```

### 4. Here-Document with FD

```bash
# Valid: "cat 3<<EOF"
# Tokens:
#   Word("cat")
#   RedirFdHereDoc(3, "EOF", false)
# 
# AST:
#   Pipeline([
#     ShellCommand {
#         args: ["cat"],
#         fd_redirections: [
#             HereDoc { fd: 3, delimiter: "EOF", strip_tabs: false }
#         ]
#     }
#   ])
```

### 5. Here-String with FD

```bash
# Valid: "grep 3<<<pattern"
# Tokens:
#   Word("grep")
#   RedirFdHereString(3, "pattern")
# 
# AST:
#   Pipeline([
#     ShellCommand {
#         args: ["grep"],
#         fd_redirections: [
#             HereString { fd: 3, content: "pattern" }
#         ]
#     }
#   ])
```

## Backward Compatibility

### Existing Redirection Syntax

All existing redirection syntax must continue to work:

- `> file` → `RedirOut` (FD 1 implied)
- `>> file` → `RedirAppend` (FD 1 implied)
- `< file` → `RedirIn` (FD 0 implied)
- `<< EOF` → `RedirHereDoc` (FD 0 implied)
- `<<< "content"` → `RedirHereString` (FD 0 implied)

### Default FD Behavior

When no FD is specified in new FD-specific tokens, use standard defaults:

- `RedirFdOut(fd, append)` → Explicit FD
- `RedirFdIn(fd)` → Explicit FD
- `RedirFdHereDoc(fd, ...)` → Explicit FD
- `RedirFdHereString(fd, ...)` → Explicit FD

## Testing Strategy

### Unit Tests

1. **FD Redirection Parsing Tests**
   - Test `2>file.txt` → `RedirectToFile { fd: 2, ... }`
   - Test `2>>file.txt` → `RedirectToFile { fd: 2, append: true }`
   - Test `3<file.txt` → `RedirectToFile { fd: 3, ... }`

2. **FD Duplication Parsing Tests**
   - Test `2>&1` → `DuplicateFdWrite { dest_fd: 2, src_fd: 1 }`
   - Test `3<&4` → `DuplicateFdRead { dest_fd: 3, src_fd: 4 }`
   - Test `2>&-` → `CloseFd { fd: 2 }`

3. **FD Here-Document Parsing Tests**
   - Test `3<<EOF` → `HereDoc { fd: 3, ... }`
   - Test `3<<'EOF'` → `HereDoc { fd: 3, ... }`
   - Test `3<<-EOF` → `HereDoc { fd: 3, ... }`

4. **FD Here-String Parsing Tests**
   - Test `3<<<pattern` → `HereString { fd: 3, ... }`

5. **Multiple FD Operations Tests**
   - Test `1>out.txt 2>&1` → Multiple redirections
   - Test `3<&4 2>&1` → Chain of operations

6. **Validation Tests**
   - Test conflicting FD operations (should error)
   - Test FD closing after redirection (should error)
   - Test duplicate FD operations (should work)

### Integration Tests

1. **Command Parsing Tests**
   - Test complete commands with FD operations
   - Verify AST structure matches expectations

2. **Pipeline Parsing Tests**
   - Test pipelines with FD operations
   - Verify FD operations are correctly associated with commands

3. **Complex Command Tests**
   - Test commands with multiple FD operations
   - Test FD operations combined with other redirections

## Implementation Notes

1. **Token Order**: FD redirection tokens must be parsed in order they appear
2. **Word Extraction**: File paths must be extracted from following word tokens
3. **Validation**: FD redirections must be validated for conflicts
4. **Error Handling**: Clear error messages for invalid FD operations
5. **Default FDs**: When no FD specified, use standard defaults (0, 1, 2)

## Dependencies

No new dependencies required. Uses existing parser infrastructure.

## References

- POSIX Shell specification: IEEE Std 1003.1-2008
- Bash manual: Redirections section
- Current parser implementation: `src/parser.rs`
