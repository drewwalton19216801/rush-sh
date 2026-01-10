# Implementation Plan: Unused Functions in Rush Shell

## Executive Summary

This plan addresses the implementation of shell features that utilize currently unused functions in [`src/state.rs`](src/state.rs:1), specifically the [`FileDescriptorTable`](src/state.rs:1) infrastructure and various [`ShellState`](src/state.rs:1) helper methods. The FileDescriptorTable is fully implemented with 17 passing tests but not integrated into the executor. These features are critical for POSIX compliance and are listed as "High Priority" in [`AGENTS.md`](AGENTS.md:1).

## Current State Analysis

### ✅ Already Implemented

1. **FileDescriptorTable Infrastructure** (Complete, 17 tests passing)
   - [`open_fd()`](src/state.rs:1), [`duplicate_fd()`](src/state.rs:1), [`close_fd()`](src/state.rs:1)
   - [`save_fd()`](src/state.rs:1), [`restore_fd()`](src/state.rs:1), [`save_all_fds()`](src/state.rs:1), [`restore_all_fds()`](src/state.rs:1)
   - [`get_stdio()`](src/state.rs:1), [`is_open()`](src/state.rs:1), [`is_closed()`](src/state.rs:1), [`clear()`](src/state.rs:1)

2. **Parser Support** (Complete)
   - All FD redirection tokens recognized in [`src/lexer.rs`](src/lexer.rs:1)
   - All FD [`Redirection`](src/parser.rs:59) enum variants defined
   - Parser correctly builds AST with FD redirections

3. **Partial Executor Integration**
   - [`apply_redirections()`](src/executor.rs:613) function exists
   - Basic FD operations implemented but **not fully utilized**
   - 17 integration tests in [`src/executor.rs`](src/executor.rs:2236) (lines 2236-2725)

### ❌ Missing/Incomplete

1. **Builtin FD Redirection Support**
   - Builtins don't use FileDescriptorTable
   - No fd state management for builtin commands

2. **Missing POSIX Builtins**
   - `set` - manipulate shell options and positional parameters
   - `eval` - evaluate arguments as shell commands
   - `exec` - replace shell with command
   - `readonly` - mark variables as read-only

3. **Unused ShellState Functions**
   - [`get_positional_params()`](src/state.rs:1) - needed for `set` builtin
   - [`push_positional_param()`](src/state.rs:1) - dynamic parameter manipulation
   - [`remove_function()`](src/state.rs:1) - for `unset -f`
   - [`get_function_names()`](src/state.rs:1) - for `declare -F`
   - [`clear_traps()`](src/state.rs:1) - trap management in subshells

## Implementation Phases

### Phase 1: Complete FD Redirection Integration (High Priority)

**Goal**: Fully integrate FileDescriptorTable with executor and builtins

#### 1.1 Builtin FD Support

**Files to Modify**:
- [`src/builtins.rs`](src/builtins.rs:1) - Update [`execute_builtin()`](src/builtins.rs:101)
- All builtin files that produce output

**Changes**:
```rust
// In execute_builtin()
// 1. Save current fd state before redirections
shell_state.fd_table.borrow_mut().save_all_fds()?;

// 2. Apply redirections using FileDescriptorTable
for redir in &cmd.redirections {
    match redir {
        Redirection::FdOutput(fd, file) => {
            shell_state.fd_table.borrow_mut().open_fd(*fd, file, false, true, false, true)?;
        }
        // ... handle all FD redirection types
    }
}

// 3. Execute builtin with proper fd context

// 4. Restore fd state after execution
shell_state.fd_table.borrow_mut().restore_all_fds()?;
```

**Testing Strategy**:
- Test each builtin with FD redirections (`echo test 2>err.log`)
- Test FD duplication with builtins (`echo test 2>&1`)
- Test FD closing with builtins (`echo test 2>&-`)
- Test multiple FD operations in sequence

#### 1.2 External Command FD Integration

**Files to Modify**:
- [`src/executor.rs`](src/executor.rs:1) - [`execute_single_command()`](src/executor.rs:1217)

**Changes**:
- Enhance [`apply_redirections()`](src/executor.rs:613) to fully utilize FileDescriptorTable
- Implement proper fd inheritance for external commands
- Add fd state save/restore around command execution

**Testing Strategy**:
- Test external commands with custom FDs (`ls 3>file.txt`)
- Test fd swapping patterns (`cmd 3>&1 1>&2 2>&3 3>&-`)
- Test fd persistence across pipeline stages

#### 1.3 Pipeline FD Management

**Files to Modify**:
- [`src/executor.rs`](src/executor.rs:1) - [`execute_pipeline()`](src/executor.rs:1443)

**Changes**:
- Implement fd state isolation between pipeline stages
- Ensure proper fd cleanup after pipeline completion
- Handle fd redirections in middle of pipelines

**Testing Strategy**:
- Test `cmd1 2>err.log | cmd2 | cmd3`
- Test `cmd1 | cmd2 3>file.txt | cmd3`
- Test complex multi-fd pipelines

### Phase 2: Implement Missing POSIX Builtins (High Priority)

#### 2.1 `set` Builtin

**Purpose**: Manipulate shell options and positional parameters

**Files to Create**:
- `src/builtins/builtin_set.rs`

**Implementation**:
```rust
// Key features:
// - set -- arg1 arg2 arg3  (set positional parameters)
// - set -x  (enable xtrace)
// - set -e  (exit on error)
// - set -o option  (set named option)
// - set +x  (disable option)
// - set (display all variables)

// Uses: shell_state.set_positional_params()
//       shell_state.get_positional_params()
```

**Testing Strategy**:
- Test setting positional parameters
- Test shell option flags (-x, -e, -u, -v)
- Test option persistence across function calls
- Test `set` without arguments (display variables)

#### 2.2 `eval` Builtin

**Purpose**: Evaluate arguments as shell commands

**Files to Create**:
- `src/builtins/builtin_eval.rs`

**Implementation**:
```rust
// Key features:
// - eval "command string"
// - Variable expansion before evaluation
// - Proper error handling
// - Exit code propagation

// Uses: crate::lexer::lex()
//       crate::parser::parse()
//       crate::executor::execute()
```

**Testing Strategy**:
- Test simple command evaluation
- Test variable expansion in eval
- Test nested eval calls
- Test eval with redirections
- Test eval error handling

#### 2.3 `exec` Builtin

**Purpose**: Replace shell process with command

**Files to Create**:
- `src/builtins/builtin_exec.rs`

**Implementation**:
```rust
// Key features:
// - exec command args  (replace shell)
// - exec <file  (redirect stdin without command)
// - exec 3>file  (open fd without command)
// - Proper cleanup before exec

// Uses: std::process::Command
//       shell_state.fd_table for fd-only exec
```

**Testing Strategy**:
- Test exec with external command
- Test exec with redirections only
- Test exec with fd operations
- Test exec error handling (command not found)

#### 2.4 `readonly` Builtin

**Purpose**: Mark variables as read-only

**Files to Create**:
- `src/builtins/builtin_readonly.rs`

**Files to Modify**:
- [`src/state.rs`](src/state.rs:1) - Add `readonly_vars: HashSet<String>`
- [`src/state.rs`](src/state.rs:1) - Update [`set_var()`](src/state.rs:1) to check readonly status

**Implementation**:
```rust
// Key features:
// - readonly VAR=value
// - readonly VAR
// - readonly -p  (list readonly variables)
// - Prevent modification of readonly variables

// New ShellState fields:
// - readonly_vars: HashSet<String>
// - is_readonly(name: &str) -> bool
// - mark_readonly(name: &str)
```

**Testing Strategy**:
- Test marking variables readonly
- Test preventing modification of readonly vars
- Test readonly with functions
- Test readonly listing

### Phase 3: Implement Unused ShellState Functions (Medium Priority)

#### 3.1 Enhanced `unset` Builtin

**Files to Modify**:
- [`src/builtins/builtin_unset.rs`](src/builtins/builtin_unset.rs:1)

**Changes**:
```rust
// Add support for:
// - unset -f function_name  (remove function)
// - unset -v variable_name  (remove variable, default)

// Uses: shell_state.remove_function()
```

**Testing Strategy**:
- Test `unset -f` with existing functions
- Test `unset -f` with non-existent functions
- Test `unset -v` for variables
- Test unset with readonly variables (should fail)

#### 3.2 Enhanced `declare` Builtin

**Files to Modify**:
- [`src/builtins/builtin_declare.rs`](src/builtins/builtin_declare.rs:1)

**Changes**:
```rust
// Add support for:
// - declare -F  (list function names only)
// - declare -f  (list function definitions)

// Uses: shell_state.get_function_names()
```

**Testing Strategy**:
- Test `declare -F` lists all functions
- Test `declare -f` shows function bodies
- Test `declare -f func_name` shows specific function

#### 3.3 Subshell Trap Management

**Files to Modify**:
- [`src/executor.rs`](src/executor.rs:1) - [`execute_subshell()`](src/executor.rs:1649)

**Changes**:
```rust
// In execute_subshell():
// 1. Clone trap handlers (already done)
// 2. Clear traps if needed for certain scenarios
// 3. Ensure trap isolation

// Uses: shell_state.clear_traps()
```

**Testing Strategy**:
- Test trap inheritance in subshells
- Test trap isolation (subshell traps don't affect parent)
- Test trap clearing in specific scenarios

#### 3.4 Dynamic Positional Parameters

**Files to Modify**:
- [`src/state.rs`](src/state.rs:1) - Document usage of [`push_positional_param()`](src/state.rs:1)

**Usage Scenarios**:
```rust
// Potential use in:
// - shift builtin (already uses set_positional_params)
// - set builtin (for adding parameters)
// - Function argument handling
```

**Testing Strategy**:
- Test adding parameters dynamically
- Test parameter ordering
- Test with `$@` and `$*` expansion

### Phase 4: Advanced FD Features (Lower Priority)

#### 4.1 Here-Document FD Support

**Files to Modify**:
- [`src/executor.rs`](src/executor.rs:1) - [`apply_heredoc_redirection()`](src/executor.rs:805)

**Changes**:
- Support custom FD for here-documents (`3<<EOF`)
- Integrate with FileDescriptorTable

#### 4.2 FD State Persistence

**Files to Modify**:
- [`src/state.rs`](src/state.rs:1) - Add fd state to shell state serialization

**Changes**:
- Implement fd state save/restore across shell invocations
- Handle fd state in subshells properly

## Testing Strategy

### Unit Tests

Each phase should include comprehensive unit tests:

1. **FD Operations** (17 existing tests + new ones)
   - Test all FD redirection types
   - Test error conditions (invalid fd, closed fd, etc.)
   - Test fd state save/restore
   - Test fd cleanup

2. **Builtin Commands** (new tests for each builtin)
   - Test basic functionality
   - Test with various options
   - Test error handling
   - Test integration with other features

3. **Integration Tests**
   - Test FD operations with builtins
   - Test FD operations with external commands
   - Test FD operations in pipelines
   - Test FD operations in subshells

### POSIX Compliance Tests

Create test suite based on POSIX specifications:

1. **FD Redirection Tests**
   - Test all POSIX-required fd operations
   - Test fd number range (0-9)
   - Test fd duplication semantics
   - Test fd closing semantics

2. **Builtin Tests**
   - Test `set` with all POSIX options
   - Test `eval` with complex expressions
   - Test `exec` with various scenarios
   - Test `readonly` with all features

### Regression Tests

Ensure existing functionality remains intact:

1. Run full existing test suite after each phase
2. Verify no performance degradation
3. Check for memory leaks with fd operations
4. Validate error message consistency

## Implementation Order (Prioritized)

### Sprint 1: Core FD Integration (1-2 weeks)
1. Phase 1.1: Builtin FD Support
2. Phase 1.2: External Command FD Integration
3. Phase 1.3: Pipeline FD Management

**Deliverable**: All FD operations working with builtins and external commands

### Sprint 2: Essential Builtins (1 week)
1. Phase 2.1: `set` builtin
2. Phase 2.2: `eval` builtin

**Deliverable**: Core POSIX builtins implemented

### Sprint 3: Advanced Builtins (1 week)
1. Phase 2.3: `exec` builtin
2. Phase 2.4: `readonly` builtin

**Deliverable**: All high-priority POSIX builtins complete

### Sprint 4: Unused Functions (3-5 days)
1. Phase 3.1: Enhanced `unset`
2. Phase 3.2: Enhanced `declare`
3. Phase 3.3: Subshell trap management
4. Phase 3.4: Dynamic positional parameters

**Deliverable**: All unused ShellState functions utilized

### Sprint 5: Polish & Advanced Features (3-5 days)
1. Phase 4.1: Here-document FD support
2. Phase 4.2: FD state persistence
3. Documentation updates
4. Performance optimization

**Deliverable**: Complete, polished implementation

## Success Criteria

### Functional Requirements

✅ All FileDescriptorTable functions are used in production code
✅ All unused ShellState functions are utilized
✅ All high-priority POSIX builtins implemented
✅ FD operations work correctly with builtins
✅ FD operations work correctly with external commands
✅ FD operations work correctly in pipelines

### Quality Requirements

✅ Test coverage remains above 90%
✅ All new features have comprehensive tests
✅ No regressions in existing functionality
✅ Error messages are clear and helpful
✅ Code follows existing patterns and style

### POSIX Compliance

✅ FD operations match POSIX semantics
✅ Builtin commands match POSIX specifications
✅ Error handling matches POSIX requirements
✅ Edge cases handled correctly

## Risk Mitigation

### Technical Risks

1. **FD State Management Complexity**
   - **Risk**: FD state corruption across commands
   - **Mitigation**: Comprehensive save/restore testing, use RAII patterns

2. **Builtin Integration Complexity**
   - **Risk**: Breaking existing builtin behavior
   - **Mitigation**: Extensive regression testing, incremental rollout

3. **Performance Impact**
   - **Risk**: FD operations slow down execution
   - **Mitigation**: Benchmark before/after, optimize hot paths

### Process Risks

1. **Scope Creep**
   - **Risk**: Adding features beyond unused functions
   - **Mitigation**: Strict adherence to plan, defer non-essential features

2. **Testing Overhead**
   - **Risk**: Test suite becomes too slow
   - **Mitigation**: Parallel test execution, optimize slow tests

## Dependencies

### Internal Dependencies

- Existing FileDescriptorTable implementation (complete)
- Existing parser support for FD operations (complete)
- Existing executor infrastructure (complete)

### External Dependencies

- None (all features use standard Rust libraries)

## Documentation Updates

### Code Documentation

1. Add comprehensive doc comments to all new functions
2. Update existing doc comments where behavior changes
3. Add examples to complex functions

### User Documentation

1. Update [`README.md`](README.md:1) with new features
2. Update [`docs/features.html`](docs/features.html:1) with FD operations
3. Add examples to [`examples/`](examples:1) directory:
   - `fd_advanced_demo.sh` - Advanced FD operations
   - `set_builtin_demo.sh` - `set` command examples
   - `eval_builtin_demo.sh` - `eval` command examples
   - `exec_builtin_demo.sh` - `exec` command examples
   - `readonly_demo.sh` - `readonly` variable examples

### Developer Documentation

1. Update [`AGENTS.md`](AGENTS.md:1) with implementation details
2. Document FD state management patterns
3. Add architecture diagrams for FD flow

## Monitoring & Metrics

### Implementation Metrics

- Lines of code added/modified
- Test coverage percentage
- Number of tests added
- Build time impact

### Quality Metrics

- Test pass rate
- Code review feedback
- Bug count in new features
- Performance benchmarks

## Conclusion

This implementation plan provides a clear, phased approach to utilizing all unused functions in [`src/state.rs`](src/state.rs:1), with a primary focus on completing the FileDescriptorTable integration. The plan prioritizes POSIX compliance and maintains the project's high quality standards through comprehensive testing and incremental delivery.

The phased approach allows for:
- Early delivery of high-value features (FD operations)
- Manageable scope for each sprint
- Continuous integration and testing
- Risk mitigation through incremental changes

Upon completion, Rush will have:
- Full POSIX-compliant FD redirection support
- All essential POSIX builtins implemented
- 100% utilization of implemented infrastructure
- Enhanced shell functionality for users
- Improved POSIX compliance rating (from ~90% to ~95%+)
