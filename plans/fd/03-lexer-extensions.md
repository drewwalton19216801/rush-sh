# Lexer Extensions for FD Operations

## Overview

This document describes the enhancements needed in the lexer to support file descriptor (FD) operations. The lexer must recognize and tokenize FD-specific syntax patterns while maintaining backward compatibility with existing redirection syntax.

## Current Lexer State

### Existing Redirection Tokenization

The current lexer (lines 592-752 in `src/lexer.rs`) partially recognizes FD syntax:

```rust
// Current partial FD recognition (lines 592-667)
'>' if !in_double_quote && !in_single_quote => {
    // Check if this is a file descriptor redirection like 2>&1
    let is_fd_redirect = if !current.is_empty() {
        current.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false)
    } else {
        false
    };

    if is_fd_redirect {
        // This might be a file descriptor redirection like 2>&1
        chars.next(); // consume >
        if let Some(&'&') = chars.peek() {
            chars.next(); // consume &
            // Now collect the target fd or '-'
            let mut target = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_ascii_digit() || ch == '-' {
                    target.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }

            if !target.is_empty() {
                // This is a valid fd redirection like 2>&1 or 2>&-
                // Remove the trailing digit from current (the fd number)
                current.pop();

                // Push any remaining content as a token
                flush_current_token(&mut current, &mut tokens);

                // For now, we'll just skip the fd redirection (treat as no-op)
                // since we don't fully support it, but we won't treat it as an error
                continue;
            }
            // ... rest of handling
        }
    }
    // ... rest of handling
}
```

### Current Limitations

1. **Incomplete FD parsing**: Recognizes FD syntax but doesn't fully parse it
2. **No FD-specific tokens**: Doesn't create tokens for FD operations
3. **Limited FD number parsing**: Only checks for trailing digits
4. **No FD closing syntax**: Doesn't recognize `>&-` or `<&-`
5. **No FD duplication syntax**: Doesn't properly parse `>&` and `<&`

## Proposed Token Types

### New Token Variants

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    // ... existing tokens ...
    
    // FD-specific redirection tokens
    /// FD redirection to file: [n]>word or [n]>>word
    RedirFdOut(i32, bool),  // fd, append
    
    /// FD input redirection: [n]<word
    RedirFdIn(i32),
    
    /// FD duplication for writing: [n]>&[m]
    RedirFdDupWrite(i32, Option<i32>),  // dest_fd, src_fd (None means close)
    
    /// FD duplication for reading: [n]<&[m]
    RedirFdDupRead(i32, Option<i32>),  // dest_fd, src_fd (None means close)
    
    /// FD closing: [n]>&- or [n]<&-
    RedirFdClose(i32),
    
    /// FD here-document: [n]<<[-]word
    RedirFdHereDoc(i32, String, bool),  // fd, delimiter, strip_tabs
    
    /// FD here-string: [n]<<<word
    RedirFdHereString(i32, String),  // fd, content
}
```

## Lexer Enhancements

### 1. FD Number Parsing

```rust
/// Parse a file descriptor number from the current word buffer
/// 
/// Returns the FD number if the current buffer ends with a valid FD number,
/// and removes it from the buffer.
fn parse_fd_number(current: &mut String) -> Option<i32> {
    if current.is_empty() {
        return None;
    }
    
    // Check if current ends with a digit sequence
    let mut fd_str = String::new();
    let mut chars = current.chars().rev().peekable();
    
    // Collect trailing digits
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() {
            fd_str.push(ch);
            chars.next();
        } else {
            break;
        }
    }
    
    // Reverse to get correct order
    fd_str = fd_str.chars().rev().collect();
    
    // Parse as FD number
    if let Ok(fd) = fd_str.parse::<i32>() {
        // Validate FD number
        if fd >= 0 && fd < 1024 {
            // Remove FD number from current buffer
            let new_len = current.len() - fd_str.len();
            current.truncate(new_len);
            return Some(fd);
        }
    }
    
    None
}
```

### 2. Enhanced `>` Token Handling

```rust
'>' if !in_double_quote && !in_single_quote => {
    // Check if this is an FD redirection
    if let Some(fd) = parse_fd_number(&mut current) {
        // Flush any remaining content before the FD number
        flush_current_token(&mut current, &mut tokens);
        
        chars.next(); // consume >
        
        // Check for append mode (>>)
        if let Some(&'>') = chars.peek() {
            chars.next(); // consume second >
            tokens.push(Token::RedirFdOut(fd, true));  // append mode
        } else {
            tokens.push(Token::RedirFdOut(fd, false));  // truncate mode
        }
    } else {
        // Normal redirection (no FD specified, defaults to stdout)
        flush_current_token(&mut current, &mut tokens);
        chars.next(); // consume >
        
        if let Some(&next_ch) = chars.peek() {
            if next_ch == '>' {
                chars.next();
                tokens.push(Token::RedirAppend);
            } else {
                tokens.push(Token::RedirOut);
            }
        } else {
            tokens.push(Token::RedirOut);
        }
    }
}
```

### 3. Enhanced `<` Token Handling

```rust
'<' if !in_double_quote && !in_single_quote => {
    // Check if this is an FD redirection
    if let Some(fd) = parse_fd_number(&mut current) {
        // Flush any remaining content before the FD number
        flush_current_token(&mut current, &mut tokens);
        
        chars.next(); // consume <
        
        // Check for FD duplication syntax (<&)
        if let Some(&'&') = chars.peek() {
            chars.next(); // consume &
            
            // Check for closing syntax (<&-)
            if let Some(&'-') = chars.peek() {
                chars.next(); // consume -
                tokens.push(Token::RedirFdClose(fd));
            } else {
                // Parse source FD number
                let mut src_fd_str = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_digit() {
                        src_fd_str.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                
                if let Ok(src_fd) = src_fd_str.parse::<i32>() {
                    if src_fd >= 0 && src_fd < 1024 {
                        tokens.push(Token::RedirFdDupRead(fd, Some(src_fd)));
                    } else {
                        // Invalid FD, treat as error
                        return Err(format!("Invalid source FD: {}", src_fd));
                    }
                } else {
                    // No source FD specified, close the FD
                    tokens.push(Token::RedirFdClose(fd));
                }
            }
        } else {
            // Regular FD input redirection
            tokens.push(Token::RedirFdIn(fd));
        }
    } else {
        // Normal input redirection (no FD specified, defaults to stdin)
        flush_current_token(&mut current, &mut tokens);
        chars.next(); // consume <
        
        if let Some(&'<') = chars.peek() {
            chars.next(); // consume second <
            
            if let Some(&'<') = chars.peek() {
                chars.next(); // consume third <
                // Here-string: skip whitespace, then collect content
                skip_whitespace(&mut chars);
                
                let mut content = String::new();
                let mut in_quote = false;
                let mut quote_char = ' ';
                
                while let Some(&ch) = chars.peek() {
                    if ch == '\n' && !in_quote {
                        break;
                    }
                    if (ch == '"' || ch == '\'') && !in_quote {
                        in_quote = true;
                        quote_char = ch;
                        chars.next(); // consume quote but don't add to content
                    } else if in_quote && ch == quote_char {
                        in_quote = false;
                        chars.next(); // consume quote but don't add to content
                    } else if !in_quote && (ch == ' ' || ch == '\t') {
                        break;
                    } else {
                        content.push(ch);
                        chars.next();
                    }
                }
                
                if !content.is_empty() {
                    tokens.push(Token::RedirHereString(content));
                } else {
                    return Err("Invalid here-string syntax: expected content after <<<".to_string());
                }
            } else {
                // Here-document: skip whitespace, then collect delimiter
                skip_whitespace(&mut chars);
                
                let mut delimiter = String::new();
                let mut in_quote = false;
                let mut quote_char = ' ';
                let mut was_quoted = false;
                
                while let Some(&ch) = chars.peek() {
                    if ch == '\n' && !in_quote {
                        break;
                    }
                    if (ch == '"' || ch == '\'') && !in_quote {
                        in_quote = true;
                        quote_char = ch;
                        was_quoted = true;
                        chars.next(); // consume quote but don't add to delimiter
                    } else if in_quote && ch == quote_char {
                        in_quote = false;
                        chars.next(); // consume quote but don't add to delimiter
                    } else if !in_quote && (ch == ' ' || ch == '\t') {
                        break;
                    } else {
                        delimiter.push(ch);
                        chars.next();
                    }
                }
                
                if !delimiter.is_empty() {
                    tokens.push(Token::RedirHereDoc(delimiter, was_quoted));
                } else {
                    return Err("Invalid here-document syntax: expected delimiter after <<".to_string());
                }
            }
        } else {
            // Regular input redirection
            tokens.push(Token::RedirIn);
        }
    }
}
```

### 4. Enhanced `&` Token Handling (for FD duplication after `>`)

```rust
// This is handled within the '>' token processing above
// But we need to handle standalone '&' for FD duplication
// This is typically part of the '>' or '<' token processing
```

## Tokenization Examples

### Example 1: Basic FD Redirection

```bash
# Input: "echo hello 2>error.txt"
# Tokens:
#   Word("echo")
#   Word("hello")
#   RedirFdOut(2, false)  # FD 2 to file, truncate
#   Word("error.txt")
```

### Example 2: FD Duplication

```bash
# Input: "command 2>&1"
# Tokens:
#   Word("command")
#   RedirFdDupWrite(2, Some(1))  # Duplicate FD 1 to FD 2
```

### Example 3: FD Closing

```bash
# Input: "command 2>&-"
# Tokens:
#   Word("command")
#   RedirFdClose(2)  # Close FD 2
```

### Example 4: Multiple FD Operations

```bash
# Input: "command 1>out.txt 2>&1"
# Tokens:
#   Word("command")
#   RedirFdOut(1, false)  # FD 1 to out.txt
#   RedirFdDupWrite(2, Some(1))  # Duplicate FD 1 to FD 2
#   Word("out.txt")
```

### Example 5: FD Here-Document

```bash
# Input: "cat 3<<EOF"
# Tokens:
#   Word("cat")
#   RedirFdHereDoc(3, "EOF", false)  # FD 3 from here-document
```

### Example 6: FD Here-String

```bash
# Input: "grep 3<<<pattern"
# Tokens:
#   Word("grep")
#   RedirFdHereString(3, "pattern")  # FD 3 from here-string
```

### Example 7: Mixed Redirections

```bash
# Input: "command <input.txt 2>error.txt"
# Tokens:
#   Word("command")
#   RedirIn  # stdin from file (no FD specified)
#   Word("input.txt")
#   RedirFdOut(2, false)  # FD 2 to file
#   Word("error.txt")
```

## Edge Cases

### 1. FD Number at Word Boundary

```bash
# Input: "cmd2>file.txt"
# Should tokenize as:
#   Word("cmd2>file.txt")  # Not an FD redirection!
# 
# The lexer must distinguish between:
# - "cmd 2>file.txt" (FD redirection)
# - "cmd2>file.txt" (command name with > in it)
```

**Solution**: Only parse FD numbers when there's whitespace before the number.

### 2. Multiple FD Numbers

```bash
# Input: "command 12>file.txt"
# Should tokenize as:
#   Word("command")
#   RedirFdOut(12, false)  # FD 12 to file
#   Word("file.txt")
```

### 3. FD Number with Leading Zeros

```bash
# Input: "command 02>file.txt"
# Should tokenize as:
#   Word("command")
#   RedirFdOut(2, false)  # FD 2 (leading zeros ignored)
#   Word("file.txt")
```

### 4. Invalid FD Numbers

```bash
# Input: "command -1>file.txt"
# Should tokenize as:
#   Word("command")
#   Word("-1>file.txt")  # Not an FD redirection (negative FD)
```

### 5. FD Number in Quotes

```bash
# Input: "command '2'>file.txt"
# Should tokenize as:
#   Word("command")
#   Word("2")  # Not an FD redirection (quoted)
#   RedirOut
#   Word("file.txt")
```

## Testing Strategy

### Unit Tests

1. **FD Number Parsing Tests**
   - Test valid FD numbers (0, 1, 2, 3, 10, 100)
   - Test invalid FD numbers (-1, 1024, -10)
   - Test FD numbers with leading zeros (02, 003)
   - Test FD numbers at word boundaries

2. **FD Redirection Tokenization Tests**
   - Test `2>file.txt` → `RedirFdOut(2, false)`
   - Test `2>>file.txt` → `RedirFdOut(2, true)`
   - Test `3<file.txt` → `RedirFdIn(3)`

3. **FD Duplication Tokenization Tests**
   - Test `2>&1` → `RedirFdDupWrite(2, Some(1))`
   - Test `3<&4` → `RedirFdDupRead(3, Some(4))`
   - Test `2>&-` → `RedirFdClose(2)`
   - Test `3<&-` → `RedirFdClose(3)`

4. **FD Here-Document Tokenization Tests**
   - Test `3<<EOF` → `RedirFdHereDoc(3, "EOF", false)`
   - Test `3<<'EOF'` → `RedirFdHereDoc(3, "EOF", true)`
   - Test `3<<-EOF` → `RedirFdHereDoc(3, "EOF", true)`

5. **FD Here-String Tokenization Tests**
   - Test `3<<<pattern` → `RedirFdHereString(3, "pattern")`
   - Test `3<<<"pattern"` → `RedirFdHereString(3, "pattern")`

6. **Edge Case Tests**
   - Test `cmd2>file.txt` (not FD redirection)
   - Test `command '2'>file.txt` (quoted FD number)
   - Test `command -1>file.txt` (negative FD)
   - Test multiple FD operations in one command

### Integration Tests

1. **Command with FD Redirections**
   - Test tokenization of complete commands with FD operations
   - Verify token sequences match expected patterns

2. **Pipeline with FD Operations**
   - Test tokenization of pipelines with FD operations
   - Verify FD operations are correctly associated with commands

3. **Complex Commands**
   - Test tokenization of commands with multiple FD operations
   - Test FD operations combined with other redirections

## Backward Compatibility

### Existing Redirection Syntax

All existing redirection syntax must continue to work:

- `> file` → `RedirOut`
- `>> file` → `RedirAppend`
- `< file` → `RedirIn`
- `<< EOF` → `RedirHereDoc`
- `<<< "content"` → `RedirHereString`

### Default FD Behavior

When no FD is specified, use standard defaults:

- `> file` → FD 1 (stdout)
- `< file` → FD 0 (stdin)
- `>> file` → FD 1 (stdout)

## Implementation Notes

1. **Whitespace Sensitivity**: FD numbers must be preceded by whitespace to distinguish from command names
2. **FD Number Range**: Valid FDs are 0-1023 (system-dependent)
3. **Quote Handling**: FD numbers inside quotes are not parsed as FDs
4. **Error Handling**: Invalid FD syntax should produce clear error messages
5. **Performance**: FD parsing should be fast (simple digit collection)

## Dependencies

No new dependencies required. Uses existing Rust standard library.

## References

- POSIX Shell specification: IEEE Std 1003.1-2008
- Bash manual: Redirections section
- Current lexer implementation: `src/lexer.rs`
