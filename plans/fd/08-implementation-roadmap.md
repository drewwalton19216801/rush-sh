# Implementation Roadmap for File Descriptor Operations

## Overview

This document outlines the phased implementation plan for adding comprehensive file descriptor (FD) operations support to the Rush shell. The roadmap is organized into logical phases with clear dependencies, deliverables, and acceptance criteria.

## Implementation Phases

### Phase 1: Foundation - FdManager Core (Week 1)

**Objective**: Implement the core FdManager module with basic FD tracking and management capabilities.

#### Tasks

1. **Create FdManager Module**
   - Create `src/fd_manager.rs`
   - Implement `FdManager` struct
   - Implement `FdEntry` struct for FD metadata
   - Add error types for FD operations

2. **Implement Core FD Operations**
   - `allocate_fd()` - Allocate new FD
   - `close_fd()` - Close specific FD
   - `is_open()` - Check if FD is open
   - `get_fd_info()` - Get FD metadata

3. **Add Unit Tests**
   - Test FD allocation
   - Test FD closure
   - Test FD queries
   - Test error handling

#### Deliverables

- [ ] `src/fd_manager.rs` with core functionality
- [ ] Unit tests in `src/fd_manager.rs` (#[cfg(test)] module)
- [ ] Documentation for all public APIs

#### Acceptance Criteria

- All unit tests pass
- Code follows Rush coding standards
- Documentation is complete
- No clippy warnings

#### Dependencies

- None (foundation phase)

---

### Phase 2: FdManager Advanced Features (Week 1-2)

**Objective**: Extend FdManager with advanced FD operations including duplication and redirection.

#### Tasks

1. **Implement FD Duplication**
   - `dup_fd()` - Duplicate FD to another FD
   - `dup2_fd()` - Duplicate FD to specific FD (with close)
   - Handle FD conflicts

2. **Implement FD Redirection**
   - `redirect_fd()` - Redirect FD to file
   - `redirect_fd_append()` - Redirect FD with append mode
   - `redirect_fd_to_fd()` - Redirect FD to another FD

3. **Add FD Query Methods**
   - `list_open_fds()` - List all open FDs
   - `get_fd_count()` - Get count of open FDs
   - `get_fds_in_range()` - Get FDs in specific range

4. **Add Unit Tests**
   - Test FD duplication
   - Test FD redirection
   - Test FD queries
   - Test edge cases

#### Deliverables

- [ ] Extended `src/fd_manager.rs` with advanced features
- [ ] Comprehensive unit tests
- [ ] Documentation for new APIs

#### Acceptance Criteria

- All unit tests pass
- FD duplication works correctly
- FD redirection works correctly
- Error handling is robust

#### Dependencies

- Phase 1 complete

---

### Phase 3: Lexer Extensions (Week 2)

**Objective**: Extend the lexer to recognize and tokenize FD operation syntax.

#### Tasks

1. **Add Token Types**
   - `Token::FdNumber(i32)` - FD number token
   - `Token::FdDupOut` - `>&` operator
   - `Token::FdDupIn` - `<&` operator
   - `Token::FdClose` - `>&-` or `<&-` operator

2. **Implement Tokenization Logic**
   - Recognize FD numbers before operators
   - Parse FD duplication operators
   - Parse FD close operators
   - Handle combined FD operations

3. **Add Unit Tests**
   - Test FD number tokenization
   - Test FD operator tokenization
   - Test complex FD operation tokenization
   - Test edge cases

#### Deliverables

- [ ] Updated `src/lexer.rs` with FD token types
  - [ ] Token enum extended
  - [ ] Tokenization logic implemented
- [ ] Unit tests for lexer FD operations
- [ ] Documentation updates

#### Acceptance Criteria

- All lexer tests pass
- FD operations are correctly tokenized
- No regressions in existing lexer functionality

#### Dependencies

- Phase 2 complete

---

### Phase 4: Parser Extensions (Week 2-3)

**Objective**: Extend the parser to construct AST nodes for FD operations.

#### Tasks

1. **Add AST Node Types**
   - `AstNode::FdRedirection` - FD redirection node
   - `AstNode::FdDuplication` - FD duplication node
   - `AstNode::FdClosure` - FD closure node

2. **Implement Parsing Logic**
   - Parse FD redirections
   - Parse FD duplications
   - Parse FD closures
   - Handle multiple FD operations per command

3. **Integrate with Command Parsing**
   - Attach FD operations to commands
   - Handle FD operations in pipelines
   - Handle FD operations in subshells

4. **Add Unit Tests**
   - Test FD redirection parsing
   - Test FD duplication parsing
   - Test FD closure parsing
   - Test complex FD operation parsing

#### Deliverables

- [ ] Updated `src/parser.rs` with FD AST nodes
  - [ ] AST enum extended
  - [ ] Parsing logic implemented
- [ ] Unit tests for parser FD operations
- [ ] Documentation updates

#### Acceptance Criteria

- All parser tests pass
- FD operations are correctly parsed
- AST structure is correct
- No regressions in existing parser functionality

#### Dependencies

- Phase 3 complete

---

### Phase 5: Executor Integration (Week 3-4)

**Objective**: Integrate FD operations into the executor for actual command execution.

#### Tasks

1. **Integrate FdManager into Executor**
   - Add FdManager to executor context
   - Initialize FdManager for each command
   - Clean up FDs after command execution

2. **Implement FD Redirection Execution**
   - Execute FD redirections to files
   - Execute FD redirections with append
   - Handle redirection errors

3. **Implement FD Duplication Execution**
   - Execute FD duplications
   - Handle FD conflicts
   - Preserve original FDs

4. **Implement FD Closure Execution**
   - Execute FD closures
   - Handle closure errors

5. **Handle FD Operations in Pipelines**
   - Apply FD operations before pipeline setup
   - Restore FDs after pipeline execution
   - Handle FD operations in subshells

6. **Add Unit Tests**
   - Test FD redirection execution
   - Test FD duplication execution
   - Test FD closure execution
   - Test FD operations in pipelines

#### Deliverables

- [ ] Updated `src/executor.rs` with FD operation execution
  - [ ] FdManager integration
  - [ ] FD operation execution logic
- [ ] Unit tests for executor FD operations
- [ ] Documentation updates

#### Acceptance Criteria

- All executor tests pass
- FD operations work correctly in commands
- FD operations work correctly in pipelines
- FD operations work correctly in subshells
- No regressions in existing executor functionality

#### Dependencies

- Phase 4 complete

---

### Phase 6: State Management Integration (Week 4)

**Objective**: Integrate FD operations with shell state management.

#### Tasks

1. **Add FD State to ShellState**
   - Add FdManager to ShellState
   - Initialize FdManager in ShellState::new()
   - Add methods for FD state access

2. **Implement FD State Persistence**
   - Save FD state across commands
   - Restore FD state after errors
   - Handle FD state in functions

3. **Add FD State to Built-in Commands**
   - Update built-in commands to respect FD state
   - Handle FD operations in built-in commands
   - Ensure built-in commands don't leak FDs

4. **Add Unit Tests**
   - Test FD state persistence
   - Test FD state in functions
   - Test FD state in built-in commands

#### Deliverables

- [ ] Updated `src/state.rs` with FD state management
  - [ ] FdManager integration
  - [ ] FD state methods
- [ ] Updated built-in commands
- [ ] Unit tests for state FD operations
- [ ] Documentation updates

#### Acceptance Criteria

- All state tests pass
- FD state is correctly managed
- Built-in commands work correctly with FD state
- No regressions in existing state functionality

#### Dependencies

- Phase 5 complete

---

### Phase 7: Integration Testing (Week 4-5)

**Objective**: Create comprehensive integration tests for FD operations.

#### Tasks

1. **Create Integration Test Module**
   - Create `src/tests/fd_integration_tests.rs`
   - Set up test infrastructure

2. **Implement Integration Tests**
   - Test standard FD operations
   - Test FD duplications
   - Test FD closures
   - Test complex FD operation scenarios
   - Test FD operations with pipelines
   - Test FD operations with subshells

3. **Add POSIX Compliance Tests**
   - Test POSIX redirection behavior
   - Test POSIX FD duplication behavior
   - Test POSIX FD closure behavior

4. **Add Edge Case Tests**
   - Test invalid FD numbers
   - Test permission errors
   - Test resource limits
   - Test concurrent operations

#### Deliverables

- [ ] `src/tests/fd_integration_tests.rs` with comprehensive tests
- [ ] POSIX compliance tests
- [ ] Edge case tests
- [ ] Test documentation

#### Acceptance Criteria

- All integration tests pass
- POSIX compliance verified
- Edge cases handled correctly
- Test coverage meets targets

#### Dependencies

- Phase 6 complete

---

### Phase 8: Here-Document Support (Week 5-6)

**Objective**: Implement here-document support with FD operations.

#### Tasks

1. **Extend Lexer for Here-Documents**
   - Recognize here-document syntax (`<< EOF`)
   - Recognize here-document with FD (`3<< EOF`)
   - Recognize quoted here-documents (`<< 'EOF'`)

2. **Extend Parser for Here-Documents**
   - Parse here-document AST nodes
   - Handle here-document content
   - Handle here-document with FD

3. **Implement Here-Document Execution**
   - Create temporary files for here-documents
   - Redirect FD to here-document file
   - Clean up temporary files

4. **Add Unit and Integration Tests**
   - Test here-document parsing
   - Test here-document execution
   - Test here-document with FD
   - Test quoted here-documents

#### Deliverables

- [ ] Updated `src/lexer.rs` with here-document support
- [ ] Updated `src/parser.rs` with here-document parsing
- [ ] Updated `src/executor.rs` with here-document execution
- [ ] Unit and integration tests
- [ ] Documentation updates

#### Acceptance Criteria

- All tests pass
- Here-documents work correctly
- Here-documents with FD work correctly
- Quoted here-documents work correctly

#### Dependencies

- Phase 7 complete

---

### Phase 9: Here-String Support (Week 6)

**Objective**: Implement here-string support with FD operations.

#### Tasks

1. **Extend Lexer for Here-Strings**
   - Recognize here-string syntax (`<<< "text"`)
   - Recognize here-string with FD (`3<<< "text"`)

2. **Extend Parser for Here-Strings**
   - Parse here-string AST nodes
   - Handle here-string content
   - Handle here-string with FD

3. **Implement Here-String Execution**
   - Create temporary files for here-strings
   - Redirect FD to here-string file
   - Clean up temporary files

4. **Add Unit and Integration Tests**
   - Test here-string parsing
   - Test here-string execution
   - Test here-string with FD

#### Deliverables

- [ ] Updated `src/lexer.rs` with here-string support
- [ ] Updated `src/parser.rs` with here-string parsing
- [ ] Updated `src/executor.rs` with here-string execution
- [ ] Unit and integration tests
- [ ] Documentation updates

#### Acceptance Criteria

- All tests pass
- Here-strings work correctly
- Here-strings with FD work correctly

#### Dependencies

- Phase 8 complete

---

### Phase 10: Error Handling and Diagnostics (Week 6-7)

**Objective**: Implement comprehensive error handling and diagnostics for FD operations.

#### Tasks

1. **Implement Error Messages**
   - Clear error messages for invalid FD operations
   - Helpful error messages for permission errors
   - Informative error messages for resource limits

2. **Add Diagnostics**
   - Warning for potentially dangerous FD operations
   - Info messages for FD state changes
   - Debug logging for FD operations

3. **Implement Error Recovery**
   - Graceful handling of FD operation failures
   - FD state restoration on errors
   - Cleanup of partial FD operations

4. **Add Tests**
   - Test error messages
   - Test error recovery
   - Test diagnostics

#### Deliverables

- [ ] Comprehensive error handling
- [ ] Clear error messages
- [ ] Diagnostics implementation
- [ ] Error recovery logic
- [ ] Tests for error handling

#### Acceptance Criteria

- Error messages are clear and helpful
- Error recovery works correctly
- Diagnostics are informative
- All error handling tests pass

#### Dependencies

- Phase 9 complete

---

### Phase 11: Performance Optimization (Week 7)

**Objective**: Optimize FD operations for performance.

#### Tasks

1. **Profile FD Operations**
   - Identify performance bottlenecks
   - Measure FD operation overhead
   - Analyze memory usage

2. **Optimize Hot Paths**
   - Optimize FD allocation
   - Optimize FD duplication
   - Optimize FD redirection

3. **Implement Caching**
   - Cache FD lookups
   - Cache FD metadata
   - Optimize FD state queries

4. **Add Performance Tests**
   - Benchmark FD operations
   - Measure performance improvements
   - Verify no regressions

#### Deliverables

- [ ] Performance optimizations
- [ ] Performance benchmarks
- [ ] Performance tests
- [ ] Performance documentation

#### Acceptance Criteria

- Performance improvements verified
- No regressions in functionality
- Performance tests pass

#### Dependencies

- Phase 10 complete

---

### Phase 12: Documentation and Examples (Week 7-8)

**Objective**: Create comprehensive documentation and examples for FD operations.

#### Tasks

1. **Update User Documentation**
   - Add FD operations to usage guide
   - Add examples for FD operations
   - Document POSIX compliance

2. **Update Developer Documentation**
   - Document FdManager API
   - Document FD operation implementation
   - Add architecture diagrams

3. **Create Example Scripts**
   - Create example scripts demonstrating FD operations
   - Add comments explaining FD operations
   - Include edge case examples

4. **Update README**
   - Add FD operations feature list
   - Add FD operations examples
   - Update feature matrix

#### Deliverables

- [ ] Updated user documentation
- [ ] Updated developer documentation
- [ ] Example scripts
- [ ] Updated README

#### Acceptance Criteria

- Documentation is comprehensive
- Examples are clear and helpful
- Documentation is accurate

#### Dependencies

- Phase 11 complete

---

### Phase 13: Final Testing and Validation (Week 8)

**Objective**: Perform final testing and validation of FD operations.

#### Tasks

1. **Run Full Test Suite**
   - Run all unit tests
   - Run all integration tests
   - Run all POSIX compliance tests

2. **Perform Manual Testing**
   - Test FD operations interactively
   - Test FD operations in scripts
   - Test edge cases manually

3. **Validate POSIX Compliance**
   - Verify all POSIX requirements met
   - Test against POSIX test suite
   - Document any deviations

4. **Performance Validation**
   - Run performance benchmarks
   - Verify performance targets met
   - Document performance characteristics

5. **Code Review**
   - Review all code changes
   - Ensure code quality standards met
   - Address any issues

#### Deliverables

- [ ] Full test suite passing
- [ ] Manual testing results
- [ ] POSIX compliance validation
- [ ] Performance validation
- [ ] Code review complete

#### Acceptance Criteria

- All tests pass
- POSIX compliance verified
- Performance targets met
- Code quality standards met

#### Dependencies

- Phase 12 complete

---

## Dependencies and Critical Path

### Critical Path

The critical path for implementation is:

```
Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 → Phase 6 → Phase 7 → Phase 8 → Phase 9 → Phase 10 → Phase 11 → Phase 12 → Phase 13
```

### Parallel Opportunities

Some phases can be partially parallelized:

- **Phase 3 (Lexer)** and **Phase 4 (Parser)** can overlap once Phase 2 is complete
- **Phase 8 (Here-Documents)** and **Phase 9 (Here-Strings)** can be developed in parallel
- **Phase 12 (Documentation)** can start during Phase 11

### Risk Mitigation

1. **Complex FD Operations**: Start with simple operations, add complexity incrementally
2. **POSIX Compliance**: Reference POSIX specification throughout implementation
3. **Performance**: Profile early and optimize iteratively
4. **Testing**: Write tests alongside implementation, not after

## Milestones

### Milestone 1: Foundation Complete (End of Week 2)

- FdManager core and advanced features complete
- Lexer and parser extensions complete
- Basic FD operations working

### Milestone 2: Integration Complete (End of Week 4)

- Executor integration complete
- State management integration complete
- FD operations working in commands and pipelines

### Milestone 3: Feature Complete (End of Week 6)

- Here-document support complete
- Here-string support complete
- Error handling and diagnostics complete

### Milestone 4: Production Ready (End of Week 8)

- Performance optimized
- Documentation complete
- All tests passing
- POSIX compliance validated

## Success Criteria

### Functional Requirements

- [ ] All standard FD operations work correctly
- [ ] FD duplication works correctly
- [ ] FD closure works correctly
- [ ] Here-documents work correctly
- [ ] Here-strings work correctly
- [ ] FD operations work in pipelines
- [ ] FD operations work in subshells
- [ ] FD operations work in functions

### Quality Requirements

- [ ] Unit test coverage ≥ 90%
- [ ] Integration test coverage ≥ 85%
- [ ] No clippy warnings
- [ ] All tests pass
- [ ] Code follows Rush coding standards

### Performance Requirements

- [ ] FD operations have minimal overhead
- [ ] No performance regressions
- [ ] Memory usage is reasonable

### Documentation Requirements

- [ ] User documentation complete
- [ ] Developer documentation complete
- [ ] Examples provided
- [ ] API documentation complete

### POSIX Compliance Requirements

- [ ] All POSIX FD operations supported
- [ ] Behavior matches POSIX specification
- [ ] Error handling matches POSIX specification

## Rollback Plan

If issues arise during implementation:

1. **Phase Rollback**: Each phase can be rolled back independently
2. **Feature Flags**: Use feature flags to enable/disable FD operations
3. **Gradual Rollout**: Enable FD operations incrementally
4. **Monitoring**: Monitor for issues after each phase

## Next Steps

1. Review and approve this roadmap
2. Assign resources and timeline
3. Begin Phase 1 implementation
4. Track progress against milestones
5. Adjust timeline as needed

## Summary

This implementation roadmap provides a structured, phased approach to adding comprehensive file descriptor operations support to the Rush shell. The roadmap is designed to:

- Build incrementally from foundation to advanced features
- Maintain code quality through comprehensive testing
- Ensure POSIX compliance throughout
- Optimize performance iteratively
- Provide clear milestones and success criteria

Following this roadmap will result in a robust, performant, and POSIX-compliant FD operations implementation for the Rush shell.
