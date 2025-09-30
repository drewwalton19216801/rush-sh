# Function Support Implementation Plan

## Overview

This document tracks the implementation of function support in rush-sh across multiple phases.

## Phase 1: Basic Function Definition and Calls ✅

**Status**: Completed

### Goals

- Add AST variants for functions
- Add lexer tokens for function syntax
- Add parser support for function definitions and calls
- Add basic execution support
- Add function storage to shell state

### Tasks ✅ All Completed

- [x] Create FUNCTIONS_TODO.md document
- [x] Add new AST variants for FunctionDefinition and FunctionCall
- [x] Add new tokens to lexer (LeftBrace, RightBrace)
- [x] Add parser support for function definitions and calls
- [x] Add function storage to ShellState
- [x] Add basic function execution support in executor
- [x] Test basic function implementation

### Implementation Summary

Successfully implemented basic function support with:

- Function definition syntax: `name() { body; }`
- Function call syntax: `name arg1 arg2`
- Function storage in ShellState
- Integration with existing command execution pipeline
- Comprehensive test coverage

### Working Examples

```bash
# Function definition
myfunc() {
    echo "Hello $1"
}

# Function call
myfunc world  # Outputs: Hello world
```

## Phase 2: Local Variable Scoping

**Status**: Not Started

### Goals

- Implement local variable stack in ShellState
- Handle variable isolation between functions
- Support nested function calls

### Tasks

- [ ] Add local variable stack to ShellState
- [ ] Implement variable scoping for functions
- [ ] Handle nested function calls
- [ ] Test variable isolation

## Phase 3: Advanced Features

**Status**: Not Started

### Goals

- Add return statements
- Add function recursion limits
- Add function introspection (`declare -f`)
- Add function export/import capabilities

### Tasks

- [ ] Add Return AST variant and execution
- [ ] Add recursion detection and limits
- [ ] Add function introspection builtin
- [ ] Add function serialization/deserialization

## Function Syntax Support

### Function Definition

```bash
# Traditional syntax
myfunc() {
    echo "Hello $1"
    echo "Args: $@"
}

# Optional function keyword
function myfunc() {
    echo "Hello $1"
}
```

### Function Calls

```bash
myfunc arg1 arg2
result=$(myfunc)
```

### Special Variables

- `$1`, `$2`, etc. - positional arguments
- `$#` - number of arguments
- `$@` - all arguments as separate words
- `$*` - all arguments as single string

## Implementation Notes

### AST Structure

```rust
FunctionDefinition {
    name: String,
    body: Box<Ast>,
},
FunctionCall {
    name: String,
    args: Vec<String>,
},
```

### Shell State Changes

- Add `functions: HashMap<String, Ast>` for function storage
- Add `local_vars: Vec<HashMap<String, String>>` for local variable stack

### Parser Integration

- Function definitions parsed at the top level
- Function calls parsed as regular commands
- Special handling for `{` `}` tokens in function bodies
