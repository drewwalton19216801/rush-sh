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

**Status**: Completed

### Goals

- Implement local variable stack in ShellState
- Handle variable isolation between functions
- Support nested function calls

### Tasks ✅ All Completed

- [x] Add 'local' keyword support to lexer (Token::Local)
- [x] Update parser to handle 'local var=value' assignments
- [x] Add local variable stack to ShellState (Vec<HashMap<String, String>>)
- [x] Modify variable get operation to search local scopes first, then outer scopes
- [x] Modify variable set operation to handle local vs global variables
- [x] Update function execution to push/pop variable scopes
- [x] Handle special variables (readonly, exported) in local scope context
- [x] Add comprehensive tests for variable isolation and scoping
- [x] Test nested function calls with variable scoping
- [x] Update function demo script to showcase Phase 2 features

### Implementation Summary

Successfully implemented local variable scoping with:

- **Local keyword support**: `local var=value` syntax for explicit local variables
- **Stack-based scoping**: `Vec<HashMap<String, String>>` for nested function scopes
- **Scope inheritance**: Inner functions can access outer scope variables
- **Automatic cleanup**: Local scopes are properly managed during function execution
- **Bash compatibility**: Follows standard bash variable scoping behavior
- **Comprehensive testing**: Full test coverage for all scoping scenarios

### Working Examples

```bash
# Local variable declaration
myfunc() {
    local local_var="local_value"
    global_var="modified_in_function"
    echo "Local: $local_var, Global: $global_var"
}

# Variable isolation
func1() {
    local my_var="func1_value"
    echo "func1: $my_var"
}

func2() {
    local my_var="func2_value"  # Different variable, no conflict
    echo "func2: $my_var"
}

# Nested functions with scoping
outer() {
    local outer_var="outer"
    inner() {
        echo "inner can see outer_var: $outer_var"
        local inner_var="inner"
    }
    inner
}
```

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

### Local Variables (Phase 2)

```bash
# Local variable declaration
local myvar="value"

# Local variable with separate tokens
local myvar value

# Local variables in functions
myfunc() {
    local local_var="local_value"
    global_var="global_value"  # This affects global scope
}
```

## Implementation Notes

### AST Structure

**Phase 1:**
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

**Phase 2:**
```rust
LocalAssignment {
    var: String,
    value: String,
},
```

### Shell State Changes

**Phase 1:**
- Add `functions: HashMap<String, Ast>` for function storage

**Phase 2:**
- Add `local_vars: Vec<HashMap<String, String>>` for local variable stack
- Add `function_depth: usize` for tracking function call depth
- Enhanced variable resolution with scope-aware get/set operations

### Parser Integration

**Phase 1:**
- Function definitions parsed at the top level
- Function calls parsed as regular commands
- Special handling for `{` `}` tokens in function bodies

**Phase 2:**
- Local assignments parsed with `local` keyword support
- Enhanced AST with `LocalAssignment` variant
- Variable scoping integrated into existing parser framework
