# File Descriptor (FD) Implementation Design Document

## Executive Summary

This document provides a comprehensive design for implementing full POSIX-compliant file descriptor support in Rush shell. The current implementation supports basic redirections (`<`, `>`, `>>`) and here-documents/here-strings, but lacks support for advanced fd operations like fd duplication, explicit fd redirection, and fd closing.

## Current Implementation Analysis

### 1. Lexer Implementation ([`src/lexer.rs`](src/lexer.rs))

**Current Tokens:**
- `RedirOut` - Output redirection `>`
- `RedirIn` - Input redirection `<`
- `RedirAppend` - Append redirection `>>`
- `RedirHereDoc(String, bool)` - Here-document `<<DELIMITER`
- `RedirHereString(String)` - Here-string `<<<content`

**Current FD Handling (Lines 592-667):**
```rust
'>' if !in_double_quote && !in_single_quote => {
    // Check if this is a file descriptor redirection like 2>&1
    let is_fd_redirect = if !current.is_empty() {
        current.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false)
    } else {
        false
    };
    
    if is_fd_redirect {
        // Detects 2>&1 pattern but currently skips it (lines 605-631)
        // This is a NO-OP - the fd redirection is silently ignored
    }
}
```

**Key Findings:**
- ✅ Basic redirections are tokenized correctly
- ✅ Here-documents and here-strings are fully supported
- ⚠️ FD redirections like `2>&1` are **detected but silently ignored** (line 629: "treat as no-op")
- ❌ No support for `<&`, `>&`, `<>`, `>&-`, `<&-`
- ❌ No support for explicit fd numbers on basic redirections (`2>file`, `3<input`)

### 2. Parser Implementation ([`src/parser.rs`](src/parser.rs))

**Current AST Structure (Lines 54-63):**
```rust
pub struct ShellCommand {
    pub args: Vec<String>,
    pub input: Option<String>,           // < file
    pub output: Option<String>,          // > file
    pub append: Option<String>,          // >> file
    pub here_doc_delimiter: Option<String>,
    pub here_doc_quoted: bool,
    pub here_string_content: Option<String>,
}
```

**Key Findings:**
- ✅ Simple redirections are parsed into dedicated fields
- ❌ **No fd number tracking** - all redirections assume default fds (0 for input, 1 for output)
- ❌ **No support for multiple redirections** - only one input and one output
- ❌ **No fd duplication representation** in AST
- ❌ **No fd closing representation** in AST

### 3. Executor Implementation ([`src/executor.rs`](src/executor.rs))

**Current Redirection Handling:**

**Input Redirection (Lines 1104-1145):**
```rust
if let Some(ref input_file) = cmd.input {
    let expanded_input = expand_variables_in_string(input_file, shell_state);
    match File::open(&expanded_input) {
        Ok(file) => {
            command.stdin(Stdio::from(file));
        }
        // Error handling...
    }
}
```

**Output Redirection (Lines 1230-1280):**
```rust
if let Some(ref output_file) = cmd.output {
    let expanded_output = expand_variables_in_string(output_file, shell_state);
    match File::create(&expanded_output) {
        Ok(file) => {
            command.stdout(Stdio::from(file));
        }
        // Error handling...
    }
}
```

**Key Findings:**
- ✅ Basic file redirections work correctly
- ✅ Here-documents and here-strings are fully implemented
- ✅ Variable expansion in redirection targets works
- ❌ **Only stdin/stdout are managed** - no stderr or custom fd support
- ❌ **No fd duplication** (`2>&1`, `3>&2`)
- ❌ **No fd closing** (`2>&-`)
- ❌ **No fd opening for read/write** (`<>`)

### 4. State Management ([`src/state.rs`](src/state.rs))

**Key Findings:**
- ❌ **No fd tracking** in ShellState
- ❌ **No fd table** or fd management structures
- ❌ **No saved fd state** for restoration after command execution

## POSIX File Descriptor Requirements

### Standard File Descriptors
- **0** - Standard input (stdin)
- **1** - Standard output (stdout)
- **2** - Standard error (stderr)
- **3-9** - User-defined file descriptors

### Required Redirection Operators

#### 1. Input Redirections
| Operator | Description | Example | Status |
|----------|-------------|---------|--------|
| `<` | Redirect stdin from file | `cmd < file` | ✅ Implemented |
| `n<` | Redirect fd n from file | `3< file` | ❌ Missing |
| `<&n` | Duplicate input fd | `0<&3` | ❌ Missing |
| `<&-` | Close input fd | `0<&-` | ❌ Missing |
| `<<` | Here-document | `cmd << EOF` | ✅ Implemented |
| `<<-` | Here-document (strip tabs) | `cmd <<- EOF` | ❌ Missing |
| `<<<` | Here-string | `cmd <<< "text"` | ✅ Implemented |
| `<>` | Open for read/write | `3<> file` | ❌ Missing |

#### 2. Output Redirections
| Operator | Description | Example | Status |
|----------|-------------|---------|--------|
| `>` | Redirect stdout to file | `cmd > file` | ✅ Implemented |
| `n>` | Redirect fd n to file | `2> file` | ❌ Missing |
| `>>` | Append stdout to file | `cmd >> file` | ✅ Implemented |
| `n>>` | Append fd n to file | `2>> file` | ❌ Missing |
| `>&n` | Duplicate output fd | `2>&1` | ❌ Missing |
| `>&-` | Close output fd | `2>&-` | ❌ Missing |
| `>|` | Clobber (override noclobber) | `cmd >| file` | ❌ Missing |

### POSIX Redirection Semantics

#### Order of Evaluation
Per POSIX, redirections are processed **left-to-right** before command execution:
```bash
# Example: Swap stdout and stderr
cmd 3>&1 1>&2 2>&3 3>&-
# Step 1: 3>&1 - Save stdout to fd 3
# Step 2: 1>&2 - Redirect stdout to stderr
# Step 3: 2>&3 - Redirect stderr to saved stdout (fd 3)
# Step 4: 3>&- - Close fd 3
```

#### Multiple Redirections
Commands can have multiple redirections of the same type:
```bash
cmd >file1 2>file2 3>file3  # Multiple output redirections
cmd <input 3<data 4<config  # Multiple input redirections
```

#### FD Duplication Rules
- `n>&m` - Make fd n a copy of fd m (output)
- `n<&m` - Make fd n a copy of fd m (input)
- `n>&-` - Close fd n (output)
- `n<&-` - Close fd n (input)
- If m is `-`, close fd n
- If m is a number, duplicate fd m to fd n

#### Noclobber Mode
When `set -C` (noclobber) is enabled:
- `>file` fails if file exists
- `>|file` overrides noclobber and creates/truncates file

## Gap Analysis

### Critical Missing Features

#### 1. Explicit FD Numbers (High Priority)
**Current:** Only default fds (0, 1) are supported
**Required:** Support `n<file`, `n>file`, `n>>file` where n is 0-9

**Impact:** Cannot redirect stderr separately from stdout
```bash
# Currently impossible in Rush:
command 2>errors.log        # Redirect stderr to file
command 2>&1                # Redirect stderr to stdout
command 2>/dev/null         # Suppress errors
```

#### 2. FD Duplication (High Priority)
**Current:** No fd duplication support
**Required:** Support `n>&m` and `n<&m`

**Impact:** Cannot combine or separate output streams
```bash
# Currently impossible in Rush:
command 2>&1 | grep error   # Combine stderr with stdout for piping
command 3>&1 1>&2 2>&3      # Swap stdout and stderr
```

#### 3. FD Closing (Medium Priority)
**Current:** No fd closing support
**Required:** Support `n>&-` and `n<&-`

**Impact:** Cannot close unwanted fds
```bash
# Currently impossible in Rush:
command 2>&-                # Close stderr
command 3>&- 4>&-           # Close custom fds
```

#### 4. Read/Write FD Opening (Low Priority)
**Current:** No read/write fd support
**Required:** Support `n<>file`

**Impact:** Cannot open files for both reading and writing
```bash
# Currently impossible in Rush:
exec 3<> datafile           # Open for read/write
echo "data" >&3             # Write to fd 3
read line <&3               # Read from fd 3
```

#### 5. Here-Document Tab Stripping (Low Priority)
**Current:** `<<-` not supported
**Required:** Support `<<-DELIMITER` to strip leading tabs

**Impact:** Minor - affects code formatting in scripts
```bash
# Currently impossible in Rush:
if true; then
    cat <<-EOF
        This text has leading tabs
        that should be stripped
    EOF
fi
```

#### 6. Noclobber Override (Low Priority)
**Current:** No noclobber mode or `>|` operator
**Required:** Support `set -C` and `>|` operator

**Impact:** Cannot prevent accidental file overwrites

### Architecture Gaps

#### 1. No FD Table
**Problem:** No data structure to track open file descriptors
**Required:** Need fd table to manage fd lifecycle

#### 2. No FD State in AST
**Problem:** Parser doesn't capture fd numbers or operations
**Required:** Extend AST to represent all fd operations

#### 3. No FD Restoration
**Problem:** No mechanism to save/restore fd state
**Required:** Save original fds before redirection, restore after

## Proposed Architecture

### 1. File Descriptor Table

Add to [`ShellState`](src/state.rs:71):

```rust
/// File descriptor table for managing open fds
pub struct FileDescriptorTable {
    /// Map of fd number to file handle
    fds: HashMap<i32, FileDescriptor>,
    /// Original fds saved before redirection (for restoration)
    saved_fds: HashMap<i32, FileDescriptor>,
}

pub enum FileDescriptor {
    /// File opened for reading
    File(File),
    /// Pipe reader
    PipeReader(PipeReader),
    /// Pipe writer
    PipeWriter(PipeWriter),
    /// Duplicate of another fd
    Dup(i32),
    /// Closed fd
    Closed,
}

impl FileDescriptorTable {
    pub fn new() -> Self;
    pub fn open_file(&mut self, fd: i32, path: &str, mode: OpenMode) -> Result<(), String>;
    pub fn duplicate(&mut self, from_fd: i32, to_fd: i32) -> Result<(), String>;
    pub fn close(&mut self, fd: i32) -> Result<(), String>;
    pub fn save_fd(&mut self, fd: i32) -> Result<(), String>;
    pub fn restore_fd(&mut self, fd: i32) -> Result<(), String>;
    pub fn get_stdio(&self, fd: i32) -> Option<Stdio>;
}
```

**Rationale:**
- Centralized fd management
- Supports all POSIX fd operations
- Enables fd state save/restore
- Type-safe fd operations

### 2. Enhanced Lexer Tokens

Add to [`Token`](src/lexer.rs:8) enum:

```rust
pub enum Token {
    // ... existing tokens ...
    
    /// Explicit fd redirection: n< or n> or n>>
    /// (fd_number, direction, append)
    RedirFd(i32, RedirDirection, bool),
    
    /// FD duplication: n>&m or n<&m
    /// (from_fd, to_fd, direction)
    RedirDup(i32, i32, RedirDirection),
    
    /// FD closing: n>&- or n<&-
    /// (fd_number, direction)
    RedirClose(i32, RedirDirection),
    
    /// Read/write fd: n<>
    /// (fd_number)
    RedirReadWrite(i32),
    
    /// Noclobber override: >|
    RedirClobber,
    
    /// Here-document with tab stripping: <<-
    RedirHereDocStrip(String, bool),
}

pub enum RedirDirection {
    Input,
    Output,
}
```

**Rationale:**
- Explicit representation of all fd operations
- Captures fd numbers at lexing stage
- Distinguishes between operation types
- Enables proper parsing and execution

### 3. Enhanced Parser AST

Replace [`ShellCommand`](src/parser.rs:54) structure:

```rust
pub struct ShellCommand {
    pub args: Vec<String>,
    /// All redirections in order of appearance
    pub redirections: Vec<Redirection>,
}

pub enum Redirection {
    /// Input from file: [n]< file
    Input { fd: i32, target: String },
    
    /// Output to file: [n]> file
    Output { fd: i32, target: String, append: bool, clobber: bool },
    
    /// Duplicate fd: n>&m or n<&m
    Duplicate { from_fd: i32, to_fd: i32 },
    
    /// Close fd: n>&- or n<&-
    Close { fd: i32 },
    
    /// Read/write: n<> file
    ReadWrite { fd: i32, target: String },
    
    /// Here-document: << or <<-
    HereDoc { fd: i32, delimiter: String, content: String, strip_tabs: bool, quoted: bool },
    
    /// Here-string: <<<
    HereString { fd: i32, content: String },
}
```

**Rationale:**
- Preserves redirection order (critical for POSIX compliance)
- Supports multiple redirections per command
- Explicit fd numbers for all operations
- Clear separation of operation types

### 4. Enhanced Executor Logic

Add to [`src/executor.rs`](src/executor.rs):

```rust
/// Apply all redirections for a command
fn apply_redirections(
    redirections: &[Redirection],
    shell_state: &mut ShellState,
    command: &mut Command,
) -> Result<(), String> {
    // Save current fd state
    for redir in redirections {
        match redir {
            Redirection::Input { fd, target } => {
                apply_input_redirection(*fd, target, shell_state, command)?;
            }
            Redirection::Output { fd, target, append, clobber } => {
                apply_output_redirection(*fd, target, *append, *clobber, shell_state, command)?;
            }
            Redirection::Duplicate { from_fd, to_fd } => {
                apply_fd_duplication(*from_fd, *to_fd, shell_state, command)?;
            }
            Redirection::Close { fd } => {
                apply_fd_close(*fd, shell_state, command)?;
            }
            Redirection::ReadWrite { fd, target } => {
                apply_read_write_redirection(*fd, target, shell_state, command)?;
            }
            Redirection::HereDoc { fd, content, .. } => {
                apply_heredoc_redirection(*fd, content, shell_state, command)?;
            }
            Redirection::HereString { fd, content } => {
                apply_herestring_redirection(*fd, content, shell_state, command)?;
            }
        }
    }
    Ok(())
}

/// Restore fd state after command execution
fn restore_redirections(
    redirections: &[Redirection],
    shell_state: &mut ShellState,
) -> Result<(), String> {
    // Restore saved fds
    for redir in redirections {
        // Restore original fd state
    }
    Ok(())
}
```

**Rationale:**
- Centralized redirection application
- Proper error handling
- FD state management
- POSIX-compliant left-to-right processing

## Implementation Plan

### Phase 1: Foundation (Week 1)

#### 1.1 Add FD Table to State
**File:** [`src/state.rs`](src/state.rs)
**Tasks:**
- Define `FileDescriptorTable` struct
- Define `FileDescriptor` enum
- Implement basic fd operations (open, close, duplicate)
- Add fd table to `ShellState`
- Write unit tests for fd table operations

**Estimated Complexity:** Medium
**Lines of Code:** ~200

#### 1.2 Enhance Lexer Tokens
**File:** [`src/lexer.rs`](src/lexer.rs)
**Tasks:**
- Add new token variants for fd operations
- Implement fd number parsing (lines 592-667)
- Handle `n<`, `n>`, `n>>` patterns
- Handle `n>&m`, `n<&m` patterns
- Handle `n>&-`, `n<&-` patterns
- Handle `n<>` pattern
- Handle `>|` pattern
- Handle `<<-` pattern
- Write comprehensive lexer tests

**Estimated Complexity:** High
**Lines of Code:** ~300

### Phase 2: Parser Enhancement (Week 2)

#### 2.1 Define Redirection AST
**File:** [`src/parser.rs`](src/parser.rs)
**Tasks:**
- Define `Redirection` enum
- Define `RedirDirection` enum
- Update `ShellCommand` structure
- Remove old redirection fields

**Estimated Complexity:** Low
**Lines of Code:** ~100

#### 2.2 Implement Redirection Parsing
**File:** [`src/parser.rs`](src/parser.rs)
**Tasks:**
- Parse all new token types into `Redirection` variants
- Maintain redirection order
- Handle multiple redirections per command
- Update `parse_pipeline` function (lines 612-706)
- Write parser tests for all redirection types

**Estimated Complexity:** High
**Lines of Code:** ~400

### Phase 3: Executor Implementation (Week 3)

#### 3.1 Implement Redirection Application
**File:** [`src/executor.rs`](src/executor.rs)
**Tasks:**
- Implement `apply_redirections` function
- Implement `apply_input_redirection`
- Implement `apply_output_redirection`
- Implement `apply_fd_duplication`
- Implement `apply_fd_close`
- Implement `apply_read_write_redirection`
- Update `execute_single_command` (lines 966-1331)
- Update `execute_pipeline` (lines 1333-1641)

**Estimated Complexity:** Very High
**Lines of Code:** ~600

#### 3.2 Implement FD State Management
**File:** [`src/executor.rs`](src/executor.rs)
**Tasks:**
- Implement fd save/restore logic
- Handle fd inheritance in pipelines
- Handle fd cleanup on errors
- Ensure proper fd lifecycle

**Estimated Complexity:** High
**Lines of Code:** ~200

### Phase 4: Testing & Integration (Week 4)

#### 4.1 Unit Tests
**Files:** All modified files
**Tasks:**
- Test each fd operation individually
- Test fd duplication chains
- Test fd closing
- Test error conditions
- Test edge cases (invalid fds, closed fds, etc.)

**Estimated Complexity:** High
**Test Cases:** ~100

#### 4.2 Integration Tests
**File:** New test file
**Tasks:**
- Test complex redirection combinations
- Test POSIX compliance scenarios
- Test interaction with pipes
- Test interaction with here-documents
- Test fd state restoration

**Estimated Complexity:** High
**Test Cases:** ~50

#### 4.3 Documentation
**Files:** README.md, docs/
**Tasks:**
- Document all fd operations
- Provide usage examples
- Update feature list
- Update compliance matrix

**Estimated Complexity:** Medium
**Lines of Documentation:** ~500

## Error Handling Strategy

### 1. Invalid FD Numbers
```rust
if fd < 0 || fd > 9 {
    return Err(format!("Invalid file descriptor: {}", fd));
}
```

### 2. Closed FD Operations
```rust
if shell_state.fd_table.is_closed(fd) {
    return Err(format!("File descriptor {} is closed", fd));
}
```

### 3. Duplicate to Self
```rust
if from_fd == to_fd {
    // POSIX: This is a no-op, not an error
    return Ok(());
}
```

### 4. File Open Failures
```rust
match File::open(path) {
    Ok(file) => { /* ... */ }
    Err(e) => {
        return Err(format!("Cannot open {}: {}", path, e));
    }
}
```

### 5. Permission Errors
```rust
match File::create(path) {
    Err(e) if e.kind() == ErrorKind::PermissionDenied => {
        return Err(format!("Permission denied: {}", path));
    }
    Err(e) => {
        return Err(format!("Cannot create {}: {}", path, e));
    }
    Ok(file) => { /* ... */ }
}
```

## Test Strategy

### 1. Lexer Tests
```rust
#[test]
fn test_lex_fd_output_redirection() {
    let shell_state = ShellState::new();
    let result = lex("command 2>errors.log", &shell_state).unwrap();
    assert_eq!(result[1], Token::RedirFd(2, RedirDirection::Output, false));
}

#[test]
fn test_lex_fd_duplication() {
    let shell_state = ShellState::new();
    let result = lex("command 2>&1", &shell_state).unwrap();
    assert_eq!(result[1], Token::RedirDup(2, 1, RedirDirection::Output));
}

#[test]
fn test_lex_fd_close() {
    let shell_state = ShellState::new();
    let result = lex("command 2>&-", &shell_state).unwrap();
    assert_eq!(result[1], Token::RedirClose(2, RedirDirection::Output));
}
```

### 2. Parser Tests
```rust
#[test]
fn test_parse_multiple_redirections() {
    let tokens = vec![
        Token::Word("command".to_string()),
        Token::RedirFd(2, RedirDirection::Output, false),
        Token::Word("errors.log".to_string()),
        Token::RedirOut,
        Token::Word("output.log".to_string()),
    ];
    let result = parse(tokens).unwrap();
    // Verify redirections are in correct order
}
```

### 3. Executor Tests
```rust
#[test]
fn test_stderr_redirection() {
    let mut shell_state = ShellState::new();
    let cmd = ShellCommand {
        args: vec!["sh".to_string(), "-c".to_string(), "echo error >&2".to_string()],
        redirections: vec![
            Redirection::Output {
                fd: 2,
                target: "errors.log".to_string(),
                append: false,
                clobber: false,
            }
        ],
    };
    execute_single_command(&cmd, &mut shell_state);
    // Verify errors.log contains "error"
}

#[test]
fn test_fd_duplication_chain() {
    // Test: command 3>&1 1>&2 2>&3 3>&-
    // This swaps stdout and stderr
}
```

### 4. Integration Tests
```rust
#[test]
fn test_complex_redirection_scenario() {
    // Test real-world scenarios like:
    // command 2>&1 | grep error
    // command >output.log 2>&1
    // command 3>&1 1>&2 2>&3 3>&-
}
```

### 5. POSIX Compliance Tests
```rust
#[test]
fn test_posix_redirection_order() {
    // Verify left-to-right processing
}

#[test]
fn test_posix_fd_inheritance() {
    // Verify fds are inherited correctly in pipelines
}
```

## Integration Points

### 1. Lexer → Parser
**Interface:** Token stream
**Changes:** New token types must be handled in parser
**Validation:** Parser tests must cover all new tokens

### 2. Parser → Executor
**Interface:** AST with Redirection list
**Changes:** Executor must process Redirection enum
**Validation:** Executor tests must verify all redirection types

### 3. Executor → State
**Interface:** FileDescriptorTable operations
**Changes:** Executor calls fd table methods
**Validation:** State tests must verify fd lifecycle

### 4. Builtins Integration
**Consideration:** Builtins like `exec` need fd management
**Changes:** May need to expose fd table to builtins
**Future Work:** Phase 2 enhancement

## Performance Considerations

### 1. FD Table Overhead
**Impact:** HashMap lookups for every fd operation
**Mitigation:** Use small HashMap (max 10 entries), very fast

### 2. FD Save/Restore
**Impact:** Additional syscalls to dup/dup2 fds
**Mitigation:** Only save fds that are actually redirected

### 3. Memory Usage
**Impact:** Additional memory for fd table
**Mitigation:** Minimal - only 10 possible fds, small structures

### 4. Redirection Processing
**Impact:** Left-to-right processing adds complexity
**Mitigation:** Straightforward iteration, no significant overhead

## Backward Compatibility

### Existing Code
✅ All existing redirection syntax remains valid
✅ Default fd behavior unchanged (stdin=0, stdout=1)
✅ Existing tests should pass without modification

### New Features
✅ New syntax is additive, doesn't break existing code
✅ Error messages improved for invalid fd operations
✅ Better POSIX compliance benefits all scripts

## Future Enhancements

### Phase 2 (Post-Initial Implementation)
1. **`exec` builtin with fd management**
   - `exec 3<file` - Open fd 3 for reading
   - `exec 3>&-` - Close fd 3
   
2. **Process substitution**
   - `<(command)` - Create fd from command output
   - `>(command)` - Create fd to command input

3. **Coprocess support**
   - `coproc command` - Bidirectional pipe

4. **Advanced fd operations**
   - `{varname}<file` - Allocate fd dynamically
   - `{varname}>&-` - Close dynamically allocated fd

## Risk Assessment

### High Risk
- **FD lifecycle management** - Complex state tracking
  - Mitigation: Comprehensive testing, careful design
  
- **Pipeline fd inheritance** - Fds must propagate correctly
  - Mitigation: Clear fd table semantics, integration tests

### Medium Risk
- **Error handling** - Many new error conditions
  - Mitigation: Consistent error messages, good test coverage
  
- **POSIX compliance** - Subtle semantic requirements
  - Mitigation: Reference POSIX spec, test against bash behavior

### Low Risk
- **Performance** - Minimal overhead expected
  - Mitigation: Benchmark critical paths
  
- **Backward compatibility** - Additive changes only
  - Mitigation: Run existing test suite

## Success Criteria

### Functional Requirements
✅ All POSIX fd operators implemented
✅ Correct left-to-right redirection processing
✅ Proper fd duplication and closing
✅ Multiple redirections per command
✅ FD state save/restore

### Quality Requirements
✅ 100% test coverage for new code
✅ All existing tests pass
✅ No performance regression
✅ Clear error messages
✅ Comprehensive documentation

### Compliance Requirements
✅ POSIX sh compliance for fd operations
✅ Bash-compatible behavior
✅ Proper error handling per POSIX

## Timeline Summary

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| Phase 1: Foundation | Week 1 | FD table, enhanced lexer |
| Phase 2: Parser | Week 2 | Redirection AST, parsing logic |
| Phase 3: Executor | Week 3 | Redirection application, fd management |
| Phase 4: Testing | Week 4 | Tests, documentation, integration |
| **Total** | **4 weeks** | **Full POSIX fd support** |

## Conclusion

This design provides a comprehensive roadmap for implementing full POSIX-compliant file descriptor support in Rush shell. The modular approach allows for incremental development and testing, while the clear architecture ensures maintainability and extensibility.

The implementation will significantly enhance Rush's POSIX compliance and enable advanced shell scripting patterns that are currently impossible. The estimated 4-week timeline is realistic given the complexity of the changes, and the phased approach allows for early validation and course correction if needed.

## References

1. **POSIX.1-2008 Shell Command Language**
   - Section 2.7: Redirection
   - https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_07

2. **Bash Reference Manual**
   - Section 3.6: Redirections
   - https://www.gnu.org/software/bash/manual/html_node/Redirections.html

3. **Advanced Bash-Scripting Guide**
   - Chapter 20: I/O Redirection
   - https://tldp.org/LDP/abs/html/io-redirection.html

4. **Rush Shell Current Implementation**
   - [`src/lexer.rs`](src/lexer.rs) - Tokenization
   - [`src/parser.rs`](src/parser.rs) - AST construction
   - [`src/executor.rs`](src/executor.rs) - Command execution
   - [`src/state.rs`](src/state.rs) - Shell state management
