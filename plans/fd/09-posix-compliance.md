# POSIX Compliance Requirements for File Descriptor Operations

## Overview

This document outlines the POSIX compliance requirements for file descriptor (FD) operations in the Rush shell. It references the IEEE Std 1003.1-2008 (POSIX.1-2008) specification and defines the expected behavior for all FD operations.

## POSIX Specification References

### Primary References

- **IEEE Std 1003.1-2008**: POSIX Shell and Utilities
- **Section 2.7**: Redirection
- **Section 2.7.1**: Redirecting Input
- **Section 2.7.2**: Redirecting Output
- **Section 2.7.3**: Appending Redirected Output
- **Section 2.7.4**: Here-Document
- **Section 2.7.5**: Duplicating an Input File Descriptor
- **Section 2.7.6**: Duplicating an Output File Descriptor
- **Section 2.7.7**: Closing File Descriptors

### Key Concepts

1. **File Descriptors**: Non-negative integers used to identify open files
2. **Standard File Descriptors**: 0 (stdin), 1 (stdout), 2 (stderr)
3. **Redirection**: Changing the association between a file descriptor and a file
4. **Duplication**: Creating a copy of a file descriptor
5. **Closure**: Closing a file descriptor

## POSIX Requirements by Operation

### 1. Input Redirection (`<`)

#### Syntax

```
[n]<word
```

#### POSIX Requirements

1. **Behavior**:
   - Open `word` for reading on file descriptor `n` (default 0)
   - If `word` cannot be opened for reading, the shell reports an error
   - The shell does not perform any expansion on `word` before opening

2. **File Creation**:
   - If the file does not exist, the redirection fails
   - The file is not created

3. **File Permissions**:
   - The file must have read permission for the user
   - If permissions are insufficient, the redirection fails

4. **Examples**:

   ```sh
   # Redirect stdin from file
   command < input.txt

   # Redirect FD 3 from file
   command 3< input.txt
   ```

#### Rush Implementation Requirements

- [ ] Implement input redirection with FD specification
- [ ] Handle file not found errors
- [ ] Handle permission errors
- [ ] Preserve original FD on error
- [ ] Support default FD 0 when not specified

---

### 2. Output Redirection (`>`)

#### Syntax

```
[n]>word
```

#### POSIX Requirements

1. **Behavior**:
   - Open `word` for writing on file descriptor `n` (default 1)
   - If the file exists, truncate it to zero length
   - If the file does not exist, create it

2. **File Creation**:
   - Create the file if it does not exist
   - Use default file permissions (mode 0666 modified by umask)

3. **File Permissions**:
   - The file must have write permission for the user
   - If permissions are insufficient, the redirection fails

4. **Examples**:

   ```sh
   # Redirect stdout to file
   command > output.txt

   # Redirect FD 3 to file
   command 3> output.txt
   ```

#### Rush Implementation Requirements

- [ ] Implement output redirection with FD specification
- [ ] Truncate existing files
- [ ] Create new files if they don't exist
- [ ] Handle permission errors
- [ ] Support default FD 1 when not specified
- [ ] Apply umask to file permissions

---

### 3. Appending Output Redirection (`>>`)

#### Syntax

```
[n]>>word
```

#### POSIX Requirements

1. **Behavior**:
   - Open `word` for writing on file descriptor `n` (default 1)
   - If the file exists, position at end of file
   - If the file does not exist, create it

2. **File Creation**:
   - Create the file if it does not exist
   - Use default file permissions (mode 0666 modified by umask)

3. **File Permissions**:
   - The file must have write permission for the user
   - If permissions are insufficient, the redirection fails

4. **Examples**:

   ```sh
   # Append stdout to file
   command >> output.txt

   # Append FD 3 to file
   command 3>> output.txt
   ```

#### Rush Implementation Requirements

- [ ] Implement append redirection with FD specification
- [ ] Position at end of file for existing files
- [ ] Create new files if they don't exist
- [ ] Handle permission errors
- [ ] Support default FD 1 when not specified
- [ ] Apply umask to file permissions

---

### 4. Here-Document (`<<`)

#### Syntax

```
[n]<<word
    here-document-content
word
```

#### POSIX Requirements

1. **Behavior**:
   - Read input until a line containing only `word` is encountered
   - Provide the input on file descriptor `n` (default 0)
   - The `word` is not subject to parameter expansion, command substitution, or arithmetic expansion

2. **Quoting**:
   - If any character in `word` is quoted, the here-document is treated literally
   - No parameter expansion, command substitution, or arithmetic expansion occurs
   - If `word` is unquoted, parameter expansion, command substitution, and arithmetic expansion occur

3. **Tab Handling**:
   - Leading tab characters are stripped from each line of the here-document
   - This is for indentation purposes in shell scripts

4. **Examples**:

   ```sh
   # Unquoted here-document (expansion occurs)
   cat << EOF
   Hello $USER
   EOF

   # Quoted here-document (literal)
   cat << 'EOF'
   Hello $USER
   EOF

   # Here-document with FD
   command 3<< EOF
   content
   EOF
   ```

#### Rush Implementation Requirements

- [ ] Implement here-document parsing
- [ ] Handle quoted and unquoted delimiters
- [ ] Perform expansion for unquoted delimiters
- [ ] Skip expansion for quoted delimiters
- [ ] Strip leading tabs from lines
- [ ] Support FD specification
- [ ] Create temporary files for here-document content
- [ ] Clean up temporary files after use

---

### 5. Here-String (`<<<`)

#### Syntax

```
[n]<<<word
```

#### POSIX Requirements

1. **Behavior**:
   - Expand `word` and supply it as input on file descriptor `n` (default 0)
   - The expansion is subject to parameter expansion, command substitution, and arithmetic expansion
   - The result is treated as a single line with a trailing newline

2. **Expansion**:
   - Perform all standard expansions on `word`
   - The result is provided as input to the command

3. **Examples**:

   ```sh
   # Here-string with expansion
   cat <<< "Hello $USER"

   # Here-string with FD
   command 3<<< "content"
   ```

#### Rush Implementation Requirements

- [ ] Implement here-string parsing
- [ ] Perform all standard expansions
- [ ] Add trailing newline to content
- [ ] Support FD specification
- [ ] Create temporary files for here-string content
- [ ] Clean up temporary files after use

---

### 6. Duplicating Input File Descriptor (`<&`)

#### Syntax

```
[n]<&word
```

#### POSIX Requirements

1. **Behavior**:
   - Duplicate input file descriptor `word` to file descriptor `n` (default 0)
   - If `word` evaluates to '-', file descriptor `n` is closed
   - If `word` is not a valid file descriptor, the shell reports an error

2. **Duplication**:
   - The two file descriptors share the same file offset and file status flags
   - Closing one file descriptor does not affect the other

3. **Examples**:

   ```sh
   # Duplicate stdin to FD 3
   command 3<&0

   # Close FD 3
   command 3<&-

   # Duplicate FD 4 to stdin
   command <&4
   ```

#### Rush Implementation Requirements

- [ ] Implement FD duplication for input
- [ ] Implement FD closure with `-`
- [ ] Validate FD numbers
- [ ] Handle invalid FD errors
- [ ] Support default FD 0 when not specified
- [ ] Ensure FDs share file offset and status flags

---

### 7. Duplicating Output File Descriptor (`>&`)

#### Syntax

```
[n]>&word
```

#### POSIX Requirements

1. **Behavior**:
   - Duplicate output file descriptor `word` to file descriptor `n` (default 1)
   - If `word` evaluates to '-', file descriptor `n` is closed
   - If `word` is not a valid file descriptor, the shell reports an error

2. **Duplication**:
   - The two file descriptors share the same file offset and file status flags
   - Closing one file descriptor does not affect the other

3. **Examples**:

   ```sh
   # Duplicate stdout to FD 3
   command 3>&1

   # Close FD 3
   command 3>&-

   # Duplicate FD 4 to stdout
   command >&4
   ```

#### Rush Implementation Requirements

- [ ] Implement FD duplication for output
- [ ] Implement FD closure with `-`
- [ ] Validate FD numbers
- [ ] Handle invalid FD errors
- [ ] Support default FD 1 when not specified
- [ ] Ensure FDs share file offset and status flags

---

### 8. Closing File Descriptors

#### Syntax

```
[n]<&-
[n]> &-
```

#### POSIX Requirements

1. **Behavior**:
   - Close file descriptor `n`
   - If `n` is not open, the operation is a no-op
   - Closing standard file descriptors (0, 1, 2) has special behavior

2. **Standard FDs**:
   - Closing stdin (0) causes reads to return EOF
   - Closing stdout (1) or stderr (2) causes writes to fail

3. **Examples**:

   ```sh
   # Close stdin
   command 0<&-

   # Close stdout
   command 1>&-

   # Close stderr
   command 2>&-

   # Close FD 3
   command 3>&-
   ```

#### Rush Implementation Requirements

- [ ] Implement FD closure
- [ ] Handle already-closed FDs gracefully
- [ ] Handle closing standard FDs
- [ ] Support both `<&-` and `>&-` syntax

---

## POSIX Requirements for Complex Scenarios

### Multiple Redirections

#### POSIX Requirements

1. **Order of Evaluation**:
   - Redirections are evaluated from left to right
   - Later redirections can override earlier ones

2. **Examples**:

   ```sh
   # Redirect stdout to file, then duplicate stderr to stdout
   command > output.txt 2>&1

   # Duplicate stderr to stdout, then redirect stdout to file
   command 2>&1 > output.txt
   ```

#### Rush Implementation Requirements

- [ ] Evaluate redirections left to right
- [ ] Allow later redirections to override earlier ones
- [ ] Handle multiple redirections correctly

---

### Redirections in Pipelines

#### POSIX Requirements

1. **Behavior**:
   - Each command in a pipeline has its standard input connected to the previous command's standard output
   - Redirections specified in the pipeline override the pipe connections

2. **Examples**:

   ```sh
   # Standard pipeline
   command1 | command2

   # Pipeline with redirection
   command1 < input.txt | command2 > output.txt
   ```

#### Rush Implementation Requirements

- [ ] Set up pipe connections before applying redirections
- [ ] Allow redirections to override pipe connections
- [ ] Handle FD operations in pipelines correctly

---

### Redirections in Subshells

#### POSIX Requirements

1. **Behavior**:
   - Redirections in subshells do not affect the parent shell
   - FD state is isolated between subshell and parent

2. **Examples**:

   ```sh
   # Subshell with redirection
   (command > output.txt)

   # Parent shell unaffected
   ```

#### Rush Implementation Requirements

- [ ] Isolate FD state in subshells
- [ ] Restore FD state after subshell execution
- [ ] Handle FD operations in subshells correctly

---

### Redirections in Command Substitutions

#### POSIX Requirements

1. **Behavior**:
   - Redirections in command substitutions do not affect the parent shell
   - FD state is isolated between command substitution and parent

2. **Examples**:

   ```sh
   # Command substitution with redirection
   var=$(command > output.txt)

   # Parent shell unaffected
   ```

#### Rush Implementation Requirements

- [ ] Isolate FD state in command substitutions
- [ ] Restore FD state after command substitution execution
- [ ] Handle FD operations in command substitutions correctly

---

### Redirections in Functions

#### POSIX Requirements

1. **Behavior**:
   - Redirections in functions affect the function's execution
   - FD state changes persist after function execution unless explicitly restored

2. **Examples**:

   ```sh
   # Function with redirection
   myfunc() {
       command > output.txt
   }

   # Call function
   myfunc
   ```

#### Rush Implementation Requirements

- [ ] Handle FD operations in functions
- [ ] Manage FD state in function scope
- [ ] Support FD state persistence/restoration

---

## POSIX Error Handling Requirements

### File Not Found

#### POSIX Requirements

1. **Behavior**:
   - If a file cannot be found for input redirection, the shell reports an error
   - The command is not executed

2. **Error Message**:
   - The shell should provide a clear error message indicating the file was not found

#### Rush Implementation Requirements

- [ ] Detect file not found errors
- [ ] Report clear error messages
- [ ] Prevent command execution on error

---

### Permission Denied

#### POSIX Requirements

1. **Behavior**:
   - If a file cannot be accessed due to insufficient permissions, the shell reports an error
   - The command is not executed

2. **Error Message**:
   - The shell should provide a clear error message indicating permission was denied

#### Rush Implementation Requirements

- [ ] Detect permission errors
- [ ] Report clear error messages
- [ ] Prevent command execution on error

---

### Invalid File Descriptor

#### POSIX Requirements

1. **Behavior**:
   - If an invalid file descriptor is specified for duplication, the shell reports an error
   - The command is not executed

2. **Error Message**:
   - The shell should provide a clear error message indicating the FD was invalid

#### Rush Implementation Requirements

- [ ] Validate FD numbers
- [ ] Report clear error messages for invalid FDs
- [ ] Prevent command execution on error

---

### Resource Limits

#### POSIX Requirements

1. **Behavior**:
   - If the system resource limit for open file descriptors is exceeded, the shell reports an error
   - The command is not executed

2. **Error Message**:
   - The shell should provide a clear error message indicating the resource limit was exceeded

#### Rush Implementation Requirements

- [ ] Detect resource limit errors
- [ ] Report clear error messages
- [ ] Prevent command execution on error

---

## POSIX Test Cases

### Basic Redirection Tests

```sh
# Test 1: Input redirection
echo "test" > /tmp/input.txt
cat < /tmp/input.txt
# Expected: test

# Test 2: Output redirection
echo "test" > /tmp/output.txt
cat /tmp/output.txt
# Expected: test

# Test 3: Append redirection
echo "line1" > /tmp/append.txt
echo "line2" >> /tmp/append.txt
cat /tmp/append.txt
# Expected: line1\nline2

# Test 4: FD redirection
echo "test" 3> /tmp/fd3.txt
# Expected: No output, file created

# Test 5: FD duplication
echo "test" 3>&1
# Expected: test (on stdout)
```

### Here-Document Tests

```sh
# Test 6: Unquoted here-document
cat << EOF
Hello $USER
EOF
# Expected: Hello <username>

# Test 7: Quoted here-document
cat << 'EOF'
Hello $USER
EOF
# Expected: Hello $USER

# Test 8: Here-document with tabs
cat << EOF
 indented line
EOF
# Expected: indented line (tabs stripped)
```

### Here-String Tests

```sh
# Test 9: Here-string with expansion
cat <<< "Hello $USER"
# Expected: Hello <username>

# Test 10: Here-string with command substitution
cat <<< "Date: $(date)"
# Expected: Date: <current date>
```

### FD Duplication Tests

```sh
# Test 11: Duplicate stdin
exec 3<&0
echo "test" >&3
# Expected: test (on stdin)

# Test 12: Duplicate stdout
exec 3>&1
echo "test" >&3
# Expected: test (on stdout)

# Test 13: Close FD
exec 3>&-
echo "test" >&3
# Expected: Error: Bad file descriptor
```

### Complex Scenario Tests

```sh
# Test 14: Multiple redirections
echo "test" > /tmp/out.txt 2>&1
# Expected: test (in /tmp/out.txt)

# Test 15: Pipeline with redirections
echo "test" | cat > /tmp/pipe.txt
cat /tmp/pipe.txt
# Expected: test

# Test 16: Subshell with redirections
(echo "test") > /tmp/subshell.txt
cat /tmp/subshell.txt
# Expected: test
```

### Error Handling Tests

```sh
# Test 17: File not found
cat < /tmp/nonexistent.txt
# Expected: Error: No such file or directory

# Test 18: Permission denied
cat < /root/.bashrc
# Expected: Error: Permission denied

# Test 19: Invalid FD
cat <&999
# Expected: Error: Bad file descriptor
```

## Rush Compliance Checklist

### Core Functionality

- [ ] Input redirection (`<`)
- [ ] Output redirection (`>`)
- [ ] Append redirection (`>>`)
- [ ] Here-document (`<<`)
- [ ] Here-string (`<<<`)
- [ ] FD duplication input (`<&`)
- [ ] FD duplication output (`>&`)
- [ ] FD closure (`<&-`, `>&-`)

### Advanced Features

- [ ] FD specification (e.g., `3<file`)
- [ ] Multiple redirections per command
- [ ] Redirections in pipelines
- [ ] Redirections in subshells
- [ ] Redirections in command substitutions
- [ ] Redirections in functions

### Error Handling

- [ ] File not found errors
- [ ] Permission denied errors
- [ ] Invalid FD errors
- [ ] Resource limit errors
- [ ] Clear error messages

### POSIX Behavior

- [ ] Left-to-right evaluation of redirections
- [ ] Proper FD sharing in duplications
- [ ] FD isolation in subshells
- [ ] FD isolation in command substitutions
- [ ] FD state management in functions

### Here-Document Features

- [ ] Quoted and unquoted delimiters
- [ ] Expansion in unquoted here-documents
- [ ] No expansion in quoted here-documents
- [ ] Tab stripping from lines
- [ ] FD specification

### Here-String Features

- [ ] Expansion in here-strings
- [ ] Trailing newline addition
- [ ] FD specification

## Deviations and Extensions

### Allowed Deviations

The following deviations from POSIX are allowed if documented:

1. **Performance Optimizations**: Internal optimizations that don't affect observable behavior
2. **Error Message Format**: Different wording as long as the meaning is clear
3. **Internal Implementation**: Different internal structure as long as behavior matches

### Extensions

The following extensions are allowed:

1. **Additional FD Operations**: Any FD operations not specified by POSIX
2. **Enhanced Error Messages**: More detailed error information
3. **Diagnostic Output**: Additional diagnostic information for debugging

### Documentation Requirements

All deviations and extensions must be documented in:

- User documentation
- Developer documentation
- POSIX compliance report

## Testing Requirements

### Unit Tests

- [ ] Test each FD operation individually
- [ ] Test error conditions
- [ ] Test edge cases

### Integration Tests

- [ ] Test FD operations in commands
- [ ] Test FD operations in pipelines
- [ ] Test FD operations in subshells
- [ ] Test FD operations in command substitutions
- [ ] Test FD operations in functions

### POSIX Compliance Tests

- [ ] Run all POSIX test cases
- [ ] Verify behavior matches POSIX specification
- [ ] Document any deviations

### Performance Tests

- [ ] Benchmark FD operations
- [ ] Verify no performance regressions
- [ ] Optimize hot paths

## Summary

This document provides a comprehensive overview of POSIX compliance requirements for file descriptor operations in the Rush shell. By following these requirements, Rush will achieve full POSIX compliance for FD operations while maintaining the project's goals of performance, reliability, and maintainability.

### Key Takeaways

1. **Comprehensive Coverage**: All POSIX FD operations must be supported
2. **Correct Behavior**: Behavior must match POSIX specification exactly
3. **Error Handling**: Errors must be handled gracefully with clear messages
4. **Testing**: Comprehensive testing is required to ensure compliance
5. **Documentation**: All behavior must be thoroughly documented

### Next Steps

1. Implement FD operations according to this specification
2. Write comprehensive tests for all operations
3. Verify POSIX compliance through testing
4. Document any deviations or extensions
5. Maintain compliance through ongoing testing
