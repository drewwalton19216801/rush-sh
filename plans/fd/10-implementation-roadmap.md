# Implementation Roadmap for File Descriptor Operations

## Overview

This document provides a phased implementation roadmap for adding file descriptor (FD) operations to the Rush shell. The roadmap is organized into phases, each with specific deliverables, dependencies, and acceptance criteria.

## Phase 0: Foundation and Preparation

### Goals

- Establish the foundation for FD operations
- Set up testing infrastructure
- Create necessary documentation

### Deliverables

1. **FdManager Module** (`src/fd_manager.rs`)
   - Core FD management functionality
   - FD tracking and validation
   - Basic FD operations (open, close, duplicate)

2. **Test Infrastructure**
   - Test utilities for FD operations
   - Test fixtures and helpers
   - POSIX compliance test suite

3. **Documentation**
   - Architecture documentation
   - API documentation
   - POSIX compliance requirements

### Tasks

- [ ] Create `src/fd_manager.rs` module
- [ ] Implement `FdManager` struct with basic functionality
- [ ] Implement `FdEntry` struct for tracking FD state
- [ ] Implement FD validation functions
- [ ] Create test utilities in `tests/fd_test_utils.rs`
- [ ] Create POSIX compliance test suite in `tests/fd_posix_tests.rs`
- [ ] Write architecture documentation
- [ ] Write API documentation
- [ ] Write POSIX compliance requirements document

### Dependencies

- None (foundation phase)

### Acceptance Criteria

- [ ] `FdManager` can be instantiated
- [ ] FD validation works correctly
- [ ] Test infrastructure is in place
- [ ] Documentation is complete

### Estimated Complexity

- **Low**: Foundation work is straightforward
- **Risk**: Low - no dependencies on existing code

---

## Phase 1: Basic Redirections

### Goals

- Implement basic input/output redirections
- Support FD specification
- Handle basic error conditions

### Deliverables

1. **Lexer Extensions**
   - Tokenize redirection operators (`<`, `>`, `>>`)
   - Tokenize FD numbers
   - Tokenize file paths

2. **Parser Extensions**
   - Parse redirection syntax
   - Create `Redirection` AST nodes
   - Integrate with command parsing

3. **Executor Extensions**
   - Execute basic redirections
   - Handle FD specification
   - Manage FD lifecycle

4. **Tests**
   - Unit tests for lexer extensions
   - Unit tests for parser extensions
   - Unit tests for executor extensions
   - Integration tests for basic redirections

### Tasks

#### Lexer Extensions

- [ ] Add `Token::FdNumber` variant
- [ ] Add `Token::RedirectInput` variant
- [ ] Add `Token::RedirectOutput` variant
- [ ] Add `Token::RedirectAppend` variant
- [ ] Implement FD number tokenization
- [ ] Implement redirection operator tokenization
- [ ] Write unit tests for lexer extensions

#### Parser Extensions

- [ ] Add `Redirection` enum to AST
- [ ] Add `RedirectionType` enum
- [ ] Implement redirection parsing
- [ ] Integrate redirections with `Command` nodes
- [ ] Write unit tests for parser extensions

#### Executor Extensions

- [ ] Implement input redirection execution
- [ ] Implement output redirection execution
- [ ] Implement append redirection execution
- [ ] Handle FD specification
- [ ] Implement error handling
- [ ] Write unit tests for executor extensions

#### Integration Tests

- [ ] Test input redirection
- [ ] Test output redirection
- [ ] Test append redirection
- [ ] Test FD specification
- [ ] Test error conditions

### Dependencies

- Phase 0 must be complete

### Acceptance Criteria

- [ ] Basic redirections work correctly
- [ ] FD specification is supported
- [ ] Error conditions are handled properly
- [ ] All tests pass

### Estimated Complexity

- **Medium**: Requires changes to lexer, parser, and executor
- **Risk**: Medium - affects core parsing and execution

---

## Phase 2: Here-Documents and Here-Strings

### Goals

- Implement here-document functionality
- Implement here-string functionality
- Support expansion in here-documents/here-strings

### Deliverables

1. **Lexer Extensions**
   - Tokenize here-document delimiters
   - Tokenize here-string syntax
   - Handle quoted delimiters

2. **Parser Extensions**
   - Parse here-document syntax
   - Parse here-string syntax
   - Create `HereDocument` and `HereString` AST nodes

3. **Executor Extensions**
   - Execute here-documents
   - Execute here-strings
   - Handle expansion
   - Manage temporary files

4. **Tests**
   - Unit tests for lexer extensions
   - Unit tests for parser extensions
   - Unit tests for executor extensions
   - Integration tests for here-documents/here-strings

### Tasks

#### Lexer Extensions

- [ ] Add `Token::HereDocument` variant
- [ ] Add `Token::HereString` variant
- [ ] Implement here-document delimiter tokenization
- [ ] Implement here-string tokenization
- [ ] Handle quoted delimiters
- [ ] Write unit tests for lexer extensions

#### Parser Extensions

- [ ] Add `HereDocument` struct to AST
- [ ] Add `HereString` struct to AST
- [ ] Implement here-document parsing
- [ ] Implement here-string parsing
- [ ] Handle quoted vs unquoted delimiters
- [ ] Write unit tests for parser extensions

#### Executor Extensions

- [ ] Implement here-document execution
- [ ] Implement here-string execution
- [ ] Handle expansion in unquoted here-documents
- [ ] Skip expansion in quoted here-documents
- [ ] Strip leading tabs from here-document lines
- [ ] Create and manage temporary files
- [ ] Clean up temporary files after use
- [ ] Write unit tests for executor extensions

#### Integration Tests

- [ ] Test unquoted here-documents
- [ ] Test quoted here-documents
- [ ] Test here-strings
- [ ] Test expansion in here-documents
- [ ] Test expansion in here-strings
- [ ] Test tab stripping
- [ ] Test temporary file cleanup

### Dependencies

- Phase 1 must be complete

### Acceptance Criteria

- [ ] Here-documents work correctly
- [ ] Here-strings work correctly
- [ ] Expansion is handled properly
- [ ] Temporary files are cleaned up
- [ ] All tests pass

### Estimated Complexity

- **High**: Complex parsing and execution logic
- **Risk**: High - requires careful handling of temporary files and expansion

---

## Phase 3: FD Duplication and Closure

### Goals

- Implement FD duplication operations
- Implement FD closure operations
- Support FD specification

### Deliverables

1. **Lexer Extensions**
   - Tokenize FD duplication operators (`<&`, `>&`)
   - Tokenize FD closure operators (`<&-`, `>&-`)

2. **Parser Extensions**
   - Parse FD duplication syntax
   - Parse FD closure syntax
   - Create `FdDuplication` and `FdClosure` AST nodes

3. **Executor Extensions**
   - Execute FD duplication
   - Execute FD closure
   - Handle FD validation

4. **Tests**
   - Unit tests for lexer extensions
   - Unit tests for parser extensions
   - Unit tests for executor extensions
   - Integration tests for FD duplication/closure

### Tasks

#### Lexer Extensions

- [ ] Add `Token::DupInput` variant
- [ ] Add `Token::DupOutput` variant
- [ ] Add `Token::CloseInput` variant
- [ ] Add `Token::CloseOutput` variant
- [ ] Implement FD duplication operator tokenization
- [ ] Implement FD closure operator tokenization
- [ ] Write unit tests for lexer extensions

#### Parser Extensions

- [ ] Add `FdDuplication` struct to AST
- [ ] Add `FdClosure` struct to AST
- [ ] Implement FD duplication parsing
- [ ] Implement FD closure parsing
- [ ] Handle `-` syntax for closure
- [ ] Write unit tests for parser extensions

#### Executor Extensions

- [ ] Implement FD duplication for input
- [ ] Implement FD duplication for output
- [ ] Implement FD closure
- [ ] Validate FD numbers
- [ ] Handle invalid FD errors
- [ ] Ensure FDs share file offset and status flags
- [ ] Write unit tests for executor extensions

#### Integration Tests

- [ ] Test FD duplication for input
- [ ] Test FD duplication for output
- [ ] Test FD closure
- [ ] Test FD validation
- [ ] Test error conditions

### Dependencies

- Phase 1 must be complete

### Acceptance Criteria

- [ ] FD duplication works correctly
- [ ] FD closure works correctly
- [ ] FD validation works correctly
- [ ] Error conditions are handled properly
- [ ] All tests pass

### Estimated Complexity

- **Medium**: Requires careful FD management
- **Risk**: Medium - FD operations can be tricky

---

## Phase 4: Complex Scenarios

### Goals

- Implement multiple redirections per command
- Implement redirections in pipelines
- Implement redirections in subshells
- Implement redirections in command substitutions
- Implement redirections in functions

### Deliverables

1. **Executor Extensions**
   - Handle multiple redirections
   - Handle redirections in pipelines
   - Handle redirections in subshells
   - Handle redirections in command substitutions
   - Handle redirections in functions

2. **State Management Extensions**
   - Manage FD state in subshells
   - Manage FD state in command substitutions
   - Manage FD state in functions

3. **Tests**
   - Integration tests for multiple redirections
   - Integration tests for redirections in pipelines
   - Integration tests for redirections in subshells
   - Integration tests for redirections in command substitutions
   - Integration tests for redirections in functions

### Tasks

#### Multiple Redirections

- [ ] Implement left-to-right evaluation of redirections
- [ ] Allow later redirections to override earlier ones
- [ ] Write integration tests for multiple redirections

#### Redirections in Pipelines

- [ ] Set up pipe connections before applying redirections
- [ ] Allow redirections to override pipe connections
- [ ] Write integration tests for redirections in pipelines

#### Redirections in Subshells

- [ ] Isolate FD state in subshells
- [ ] Restore FD state after subshell execution
- [ ] Write integration tests for redirections in subshells

#### Redirections in Command Substitutions

- [ ] Isolate FD state in command substitutions
- [ ] Restore FD state after command substitution execution
- [ ] Write integration tests for redirections in command substitutions

#### Redirections in Functions

- [ ] Handle FD operations in functions
- [ ] Manage FD state in function scope
- [ ] Support FD state persistence/restoration
- [ ] Write integration tests for redirections in functions

### Dependencies

- Phases 1, 2, and 3 must be complete

### Acceptance Criteria

- [ ] Multiple redirections work correctly
- [ ] Redirections in pipelines work correctly
- [ ] Redirections in subshells work correctly
- [ ] Redirections in command substitutions work correctly
- [ ] Redirections in functions work correctly
- [ ] All tests pass

### Estimated Complexity

- **High**: Complex interactions between different shell features
- **Risk**: High - requires careful state management

---

## Phase 5: Error Handling and Robustness

### Goals

- Implement comprehensive error handling
- Add diagnostic output
- Improve error messages
- Handle edge cases

### Deliverables

1. **Error Handling**
   - File not found errors
   - Permission denied errors
   - Invalid FD errors
   - Resource limit errors

2. **Diagnostic Output**
   - Detailed error messages
   - Contextual information
   - Suggestions for fixing errors

3. **Tests**
   - Error handling tests
   - Edge case tests
   - Robustness tests

### Tasks

#### Error Handling

- [ ] Implement file not found error handling
- [ ] Implement permission denied error handling
- [ ] Implement invalid FD error handling
- [ ] Implement resource limit error handling
- [ ] Add clear error messages
- [ ] Write error handling tests

#### Diagnostic Output

- [ ] Add detailed error messages
- [ ] Add contextual information
- [ ] Add suggestions for fixing errors
- [ ] Write diagnostic output tests

#### Edge Cases

- [ ] Handle closing already-closed FDs
- [ ] Handle duplicating to closed FDs
- [ ] Handle redirections with invalid paths
- [ ] Handle redirections with special characters
- [ ] Write edge case tests

### Dependencies

- Phases 1, 2, 3, and 4 must be complete

### Acceptance Criteria

- [ ] All error conditions are handled
- [ ] Error messages are clear and helpful
- [ ] Edge cases are handled gracefully
- [ ] All tests pass

### Estimated Complexity

- **Medium**: Requires careful error handling
- **Risk**: Medium - edge cases can be tricky

---

## Phase 6: POSIX Compliance and Testing

### Goals

- Verify POSIX compliance
- Run comprehensive test suite
- Document any deviations
- Fix any compliance issues

### Deliverables

1. **POSIX Compliance Tests**
   - Run all POSIX test cases
   - Verify behavior matches POSIX specification
   - Document any deviations

2. **Comprehensive Test Suite**
   - Unit tests for all FD operations
   - Integration tests for all scenarios
   - POSIX compliance tests
   - Performance tests

3. **Documentation**
   - POSIX compliance report
   - Deviations documentation
   - Extensions documentation

### Tasks

#### POSIX Compliance Tests

- [ ] Run all POSIX test cases
- [ ] Verify behavior matches POSIX specification
- [ ] Document any deviations
- [ ] Fix any compliance issues

#### Comprehensive Test Suite

- [ ] Write unit tests for all FD operations
- [ ] Write integration tests for all scenarios
- [ ] Write POSIX compliance tests
- [ ] Write performance tests
- [ ] Ensure all tests pass

#### Documentation

- [ ] Write POSIX compliance report
- [ ] Document any deviations
- [ ] Document any extensions
- [ ] Update user documentation
- [ ] Update developer documentation

### Dependencies

- Phases 1, 2, 3, 4, and 5 must be complete

### Acceptance Criteria

- [ ] All POSIX test cases pass
- [ ] Behavior matches POSIX specification
- [ ] All deviations are documented
- [ ] All tests pass
- [ ] Documentation is complete

### Estimated Complexity

- **Medium**: Requires thorough testing and documentation
- **Risk**: Low - mostly verification work

---

## Phase 7: Performance Optimization

### Goals

- Optimize FD operations for performance
- Reduce memory usage
- Improve execution speed
- Benchmark and verify improvements

### Deliverables

1. **Performance Optimizations**
   - Optimize hot paths
   - Reduce allocations
   - Improve caching

2. **Benchmarks**
   - Benchmark FD operations
   - Compare with baseline
   - Verify improvements

3. **Documentation**
   - Performance report
   - Optimization notes

### Tasks

#### Performance Optimizations

- [ ] Profile FD operations
- [ ] Identify hot paths
- [ ] Optimize hot paths
- [ ] Reduce allocations
- [ ] Improve caching
- [ ] Write performance tests

#### Benchmarks

- [ ] Benchmark FD operations
- [ ] Compare with baseline
- [ ] Verify improvements
- [ ] Write benchmark report

#### Documentation

- [ ] Write performance report
- [ ] Document optimizations
- [ ] Update developer documentation

### Dependencies

- Phases 1, 2, 3, 4, 5, and 6 must be complete

### Acceptance Criteria

- [ ] Performance is improved
- [ ] Memory usage is reduced
- [ ] All tests still pass
- [ ] Documentation is complete

### Estimated Complexity

- **Medium**: Requires profiling and optimization
- **Risk**: Low - optimizations can be reverted if they cause issues

---

## Phase 8: Documentation and Examples

### Goals

- Complete user documentation
- Complete developer documentation
- Create examples
- Create tutorials

### Deliverables

1. **User Documentation**
   - FD operations guide
   - Examples
   - Tutorials

2. **Developer Documentation**
   - Architecture documentation
   - API documentation
   - Implementation notes

3. **Examples**
   - Example scripts
   - Example use cases

### Tasks

#### User Documentation

- [ ] Write FD operations guide
- [ ] Write examples
- [ ] Write tutorials
- [ ] Update README

#### Developer Documentation

- [ ] Update architecture documentation
- [ ] Update API documentation
- [ ] Write implementation notes
- [ ] Update AGENTS.md

#### Examples

- [ ] Create example scripts
- [ ] Create example use cases
- [ ] Add examples to examples/ directory

### Dependencies

- Phases 1, 2, 3, 4, 5, 6, and 7 must be complete

### Acceptance Criteria

- [ ] User documentation is complete
- [ ] Developer documentation is complete
- [ ] Examples are provided
- [ ] Tutorials are provided

### Estimated Complexity

- **Low**: Documentation work
- **Risk**: Low - no code changes

---

## Testing Strategy

### Unit Tests

- Test each component in isolation
- Test all public APIs
- Test error conditions
- Test edge cases

### Integration Tests

- Test interactions between components
- Test complex scenarios
- Test end-to-end workflows

### POSIX Compliance Tests

- Test all POSIX requirements
- Verify behavior matches specification
- Document any deviations

### Performance Tests

- Benchmark FD operations
- Verify no regressions
- Measure improvements

### Test Organization

```
tests/
├── fd/
│   ├── fd_manager_tests.rs      # FdManager unit tests
│   ├── lexer_tests.rs           # Lexer extension tests
│   ├── parser_tests.rs          # Parser extension tests
│   ├── executor_tests.rs        # Executor extension tests
│   ├── integration_tests.rs     # Integration tests
│   ├── posix_tests.rs           # POSIX compliance tests
│   └── performance_tests.rs     # Performance tests
└── fd_test_utils.rs             # Test utilities
```

### Test Coverage Goals

- **Unit Tests**: 90%+ coverage
- **Integration Tests**: All major scenarios
- **POSIX Tests**: All POSIX requirements
- **Performance Tests**: All FD operations

---

## Risk Management

### High-Risk Items

1. **Here-Documents and Here-Strings**
   - **Risk**: Complex parsing and execution logic
   - **Mitigation**: Thorough testing, careful implementation

2. **Complex Scenarios**
   - **Risk**: Complex interactions between shell features
   - **Mitigation**: Incremental implementation, comprehensive testing

3. **FD State Management**
   - **Risk**: FD state can be tricky to manage
   - **Mitigation**: Clear API, comprehensive testing

### Medium-Risk Items

1. **Basic Redirections**
   - **Risk**: Affects core parsing and execution
   - **Mitigation**: Careful implementation, thorough testing

2. **FD Duplication and Closure**
   - **Risk**: FD operations can be tricky
   - **Mitigation**: Clear API, comprehensive testing

3. **Error Handling**
   - **Risk**: Edge cases can be tricky
   - **Mitigation**: Comprehensive testing, clear error messages

### Low-Risk Items

1. **Foundation and Preparation**
   - **Risk**: Low - no dependencies on existing code
   - **Mitigation**: Straightforward implementation

2. **POSIX Compliance and Testing**
   - **Risk**: Low - mostly verification work
   - **Mitigation**: Thorough testing, documentation

3. **Performance Optimization**
   - **Risk**: Low - optimizations can be reverted
   - **Mitigation**: Profiling, benchmarking

4. **Documentation and Examples**
   - **Risk**: Low - no code changes
   - **Mitigation**: Clear documentation

---

## Success Criteria

### Functional Requirements

- [ ] All POSIX FD operations are supported
- [ ] All FD operations work correctly
- [ ] All error conditions are handled
- [ ] All edge cases are handled

### Non-Functional Requirements

- [ ] Performance is acceptable
- [ ] Memory usage is reasonable
- [ ] Code is maintainable
- [ ] Documentation is complete

### Quality Requirements

- [ ] All tests pass
- [ ] Test coverage is adequate
- [ ] Code follows project standards
- [ ] POSIX compliance is verified

---

## Timeline

### Phase 0: Foundation and Preparation

- **Duration**: 1-2 weeks
- **Dependencies**: None

### Phase 1: Basic Redirections

- **Duration**: 2-3 weeks
- **Dependencies**: Phase 0

### Phase 2: Here-Documents and Here-Strings

- **Duration**: 3-4 weeks
- **Dependencies**: Phase 1

### Phase 3: FD Duplication and Closure

- **Duration**: 2-3 weeks
- **Dependencies**: Phase 1

### Phase 4: Complex Scenarios

- **Duration**: 3-4 weeks
- **Dependencies**: Phases 1, 2, 3

### Phase 5: Error Handling and Robustness

- **Duration**: 2-3 weeks
- **Dependencies**: Phases 1, 2, 3, 4

### Phase 6: POSIX Compliance and Testing

- **Duration**: 2-3 weeks
- **Dependencies**: Phases 1, 2, 3, 4, 5

### Phase 7: Performance Optimization

- **Duration**: 1-2 weeks
- **Dependencies**: Phases 1, 2, 3, 4, 5, 6

### Phase 8: Documentation and Examples

- **Duration**: 1-2 weeks
- **Dependencies**: Phases 1, 2, 3, 4, 5, 6, 7

### Total Duration

- **Minimum**: 17 weeks
- **Maximum**: 26 weeks

---

## Next Steps

1. **Review and Approve**: Review this roadmap and approve the plan
2. **Start Phase 0**: Begin with foundation and preparation
3. **Iterate**: Work through each phase sequentially
4. **Test**: Thoroughly test each phase before moving to the next
5. **Document**: Keep documentation up to date throughout the process

---

## Summary

This implementation roadmap provides a clear, phased approach to adding file descriptor operations to the Rush shell. By following this roadmap, the project will achieve full POSIX compliance for FD operations while maintaining the project's goals of performance, reliability, and maintainability.

### Key Takeaways

1. **Phased Approach**: Break down the work into manageable phases
2. **Dependencies**: Each phase depends on previous phases
3. **Testing**: Comprehensive testing at each phase
4. **Documentation**: Keep documentation up to date
5. **Risk Management**: Identify and mitigate risks early

### Success Factors

1. **Clear Requirements**: POSIX compliance requirements are well-defined
2. **Incremental Implementation**: Each phase builds on the previous one
3. **Comprehensive Testing**: Thorough testing at each phase
4. **Clear Documentation**: Documentation is kept up to date
5. **Risk Management**: Risks are identified and mitigated early

By following this roadmap, the Rush shell will achieve full POSIX compliance for file descriptor operations while maintaining the project's high standards for code quality, performance, and reliability.
