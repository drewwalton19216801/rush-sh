#!/usr/bin/env rush-sh

# Function Support Demo for Rush Shell
# This script demonstrates Phases 1, 2, and 3 of the function implementation:
# Phase 1: Basic function definition and calls
# Phase 2: Local variable scoping
# Phase 3: Advanced features (return statements, recursion, introspection)

echo "=========================================="
echo "    RUSH SHELL FUNCTION SUPPORT DEMO"
echo "=========================================="
echo ""

# =============================================================================
# PHASE 1: BASIC FUNCTION DEFINITION AND CALLS
# =============================================================================

echo "=== PHASE 1: BASIC FUNCTION DEFINITION AND CALLS ==="
echo ""

# Define a simple function that greets someone
echo "1. Defining a simple greeting function:"
echo ""

greet() {
    echo "Hello $1!"
}

echo "2. Calling the function:"
echo "   greet world"
greet world
echo ""

# Define a function that shows function arguments
echo "3. Defining a function to show arguments:"
echo ""

show_args() {
    echo "Function name: $0"
    echo "First arg: $1"
    echo "Second arg: $2"
    echo "All args (\$*): $*"
    echo "Arg count (\$#): $#"
}

echo "4. Calling with multiple arguments:"
echo "   show_args Alice Bob"
show_args Alice Bob
echo ""

# =============================================================================
# FUNCTIONS WITH MULTIPLE COMMANDS
# =============================================================================

echo "=== FUNCTIONS WITH MULTIPLE COMMANDS ==="
echo ""

echo "5. Defining a function with multiple commands:"
echo ""

countdown() {
    echo "Starting countdown..."
    for i in 3 2 1
    do
        echo "$i"
    done
    echo "Go!"
}

echo "6. Calling the countdown function:"
countdown
echo ""

# =============================================================================
# FUNCTIONS CALLING OTHER FUNCTIONS
# =============================================================================

echo "=== FUNCTIONS CALLING OTHER FUNCTIONS ==="
echo ""

echo "7. Defining helper functions:"
echo ""

get_name() {
    echo "Rush"
}

get_version() {
    echo "1.0.0"
}

show_info() {
    echo "Shell: $(get_name)"
    echo "Version: $(get_version)"
}

echo "8. Calling function that uses other functions:"
show_info
echo ""

# =============================================================================
# PRACTICAL EXAMPLES
# =============================================================================

echo "=== PRACTICAL EXAMPLES ==="
echo ""

echo "9. Defining utility functions:"
echo ""

backup_file() {
    if [ -f "$1" ]; then
        cp "$1" "$1.backup"
        echo "Backed up $1"
    else
        echo "File $1 does not exist"
    fi
}

create_temp() {
    temp_file="/tmp/rush_temp_$$"
    echo "Temporary data" > "$temp_file"
    echo "$temp_file"
}

echo "10. Using utility functions:"
echo "Creating a test file..."
echo "test content" > test_demo.txt
echo "Backing up file:"
backup_file test_demo.txt
echo "Creating temporary file:"
temp_file=$(create_temp)
echo "Temp file created: $temp_file"
echo "Cleaning up..."
rm -f test_demo.txt test_demo.txt.backup "$temp_file"
echo ""

# =============================================================================
# ERROR HANDLING AND EDGE CASES
# =============================================================================

echo "=== ERROR HANDLING AND EDGE CASES ==="
echo ""

echo "11. Calling undefined function:"
echo "   undefined_func 2>&1 || echo \"Function not found (expected)\""
undefined_func 2>&1 || echo "Function not found (expected)"
echo ""

echo "12. Calling function with no arguments:"
echo "   greet"
greet
echo ""

echo "13. Calling function with too many arguments:"
echo "   show_args arg1 arg2 arg3 arg4 arg5"
show_args arg1 arg2 arg3 arg4 arg5
echo ""

# =============================================================================
# PHASE 2: LOCAL VARIABLE SCOPING
# =============================================================================

echo "=== PHASE 2: LOCAL VARIABLE SCOPING ==="
echo ""

echo "14. Demonstrating local variable scoping:"
echo ""

# Set a global variable
global_var="global_value"
echo "   Global variable set: global_var = $global_var"

# Define function with local variable
demo_local() {
    echo "   Inside function demo_local:"
    echo "   Global var before: $global_var"

    # Declare local variable (Phase 2 feature)
    local local_var="local_value"
    echo "   Local variable: local_var = $local_var"

    # Modify global variable
    global_var="modified_in_function"
    echo "   Global var after modification: $global_var"
}

echo "15. Calling function with local variables:"
demo_local

echo ""
echo "16. After function call:"
echo "   Global variable: global_var = $global_var"
echo "   Local variable (should be undefined): local_var = ${local_var:-'<undefined>'}"
echo ""

# =============================================================================
# VARIABLE ISOLATION BETWEEN FUNCTIONS
# =============================================================================

echo "=== VARIABLE ISOLATION BETWEEN FUNCTIONS ==="
echo ""

func1() {
    local my_var="func1_value"
    echo "   func1: my_var = $my_var"
}

func2() {
    local my_var="func2_value"
    echo "   func2: my_var = $my_var"
}

echo "17. Testing variable isolation:"
func1
func2
echo "   After both functions: my_var = ${my_var:-'<undefined>'}"
echo ""

# =============================================================================
# NESTED FUNCTION CALLS WITH SCOPING
# =============================================================================

echo "=== NESTED FUNCTION CALLS WITH SCOPING ==="
echo ""

outer_func() {
    local outer_var="outer_value"
    echo "   outer_func: outer_var = $outer_var"

    inner_func() {
        local inner_var="inner_value"
        echo "     inner_func: inner_var = $inner_var"
        echo "     inner_func: outer_var (inherited) = $outer_var"

        # Modify outer scope variable
        outer_var="modified_by_inner"
        echo "     inner_func: outer_var after modification = $outer_var"
    }

    echo "   outer_func before inner call: outer_var = $outer_var"
    inner_func
    echo "   outer_func after inner call: outer_var = $outer_var"
    echo "   outer_func: inner_var (should be undefined) = ${inner_var:-'<undefined>'}"
}

echo "18. Calling nested functions:"
outer_func
echo ""

# =============================================================================
# PHASE 3: ADVANCED FEATURES - RETURN STATEMENTS
# =============================================================================

echo "=== PHASE 3: ADVANCED FEATURES ==="
echo ""
echo "=== SECTION A: RETURN STATEMENTS ==="
echo ""

echo "19. Basic return (no value):"
echo ""

basic_return() {
    echo "   Function executing"
    return
    echo "   This should not print"
}

basic_return
echo "   Exit code: $?"
echo ""

echo "20. Return with explicit value:"
echo ""

add_three() {
    return 3
}

add_three
echo "   Return value: $?"
echo ""

echo "21. Early return from function:"
echo ""

early_return() {
    echo "   Before return"
    return 42
    echo "   After return (should not print)"
}

early_return
echo "   Exit code: $?"
echo ""

echo "22. Conditional returns:"
echo ""

check_number() {
    if [ $1 -gt 0 ]; then
        echo "   Positive"
        return 0
    else
        echo "   Non-positive"
        return 1
    fi
}

echo "   Checking 5:"
check_number 5
echo "   Exit code: $?"
echo "   Checking -3:"
check_number -3
echo "   Exit code: $?"
echo ""

echo "23. Capture return value with \$?:"
echo ""

compute() {
    local result=$((5 * 8))
    return $result
}

compute
captured=$?
echo "   Captured return value: $captured"
echo ""

echo "24. Return outside function (error case):"
echo ""
echo "   Attempting return outside function:"
return 5 2>&1 || echo "   Error caught (expected)"
echo ""

# =============================================================================
# PHASE 3: SECTION B - RECURSION AND LIMITS
# =============================================================================

echo "=== SECTION B: RECURSION AND LIMITS ==="
echo ""

echo "25. Simple recursion - factorial (using echo for output):"
echo ""

factorial() {
    if [ $1 -le 1 ]; then
        echo 1
        return 0
    fi
    local prev=$(factorial $(($1 - 1)))
    echo $(($1 * $prev))
}

echo "   Computing factorial(5):"
result=$(factorial 5)
echo "   5! = $result"
echo ""

echo "26. Fibonacci with echo output:"
echo ""

fib() {
    if [ $1 -le 1 ]; then
        echo $1
        return 0
    fi
    local a=$(fib $(($1 - 1)))
    local b=$(fib $(($1 - 2)))
    echo $((a + b))
}

echo "   Computing fib(6):"
result=$(fib 6)
echo "   fib(6) = $result"
echo ""

echo "27. Mutual recursion (even/odd):"
echo ""

is_even() {
    if [ $1 -eq 0 ]; then
        return 0
    fi
    is_odd $(($1 - 1))
}

is_odd() {
    if [ $1 -eq 0 ]; then
        return 1
    fi
    is_even $(($1 - 1))
}

echo "   Testing is_even(4):"
is_even 4
echo "   Result: $? (0 = true)"
echo "   Testing is_odd(4):"
is_odd 4
echo "   Result: $? (1 = false)"
echo ""

echo "28. Recursion limit detection:"
echo ""

deep_recursion() {
    deep_recursion $(($1 + 1))
}

echo "   Testing recursion limit (should error at 500):"
deep_recursion 1 2>&1 || echo "   Recursion limit reached (expected)"
echo ""

# =============================================================================
# PHASE 3: SECTION C - FUNCTION INTROSPECTION
# =============================================================================

echo "=== SECTION C: FUNCTION INTROSPECTION ==="
echo ""

echo "29. List all defined functions:"
echo ""
echo "   declare -f | head -20"
declare -f | head -20
echo "   ... (output truncated)"
echo ""

echo "30. Show specific function definition:"
echo ""
echo "   declare -f factorial"
declare -f factorial
echo ""

echo "31. Show non-existent function (error case):"
echo ""
echo "   declare -f nonexistent_function"
declare -f nonexistent_function 2>&1 || echo "   Not found (expected)"
echo ""

echo "32. List function names only:"
echo ""
echo "   declare -F"
declare -F
echo ""

echo "33. Introspect complex function:"
echo ""

complex_func() {
    local x=$1
    if [ $x -gt 10 ]; then
        for i in 1 2 3; do
            echo "Loop: $i"
        done
        return 0
    else
        return 1
    fi
}

echo "   declare -f complex_func"
declare -f complex_func
echo ""

# =============================================================================
# INTEGRATION TESTS - COMBINING ALL FEATURES
# =============================================================================

echo "=== INTEGRATION TESTS ==="
echo ""

echo "34. Advanced calculator (all features combined):"
echo ""

advanced_calculator() {
    local operation=$1
    local a=$2
    local b=$3
    
    if [ "$operation" = "add" ]; then
        echo $((a + b))
        return 0
    fi
    
    if [ "$operation" = "subtract" ]; then
        echo $((a - b))
        return 0
    fi
    
    if [ "$operation" = "multiply" ]; then
        if [ $b -eq 0 ]; then
            echo 0
            return 0
        fi
        if [ $b -eq 1 ]; then
            echo $a
            return 0
        fi
        # Iterative multiplication
        local result=0
        local i=0
        while [ $i -lt $b ]; do
            result=$((result + a))
            i=$((i + 1))
        done
        echo $result
        return 0
    fi
    
    echo "   Unknown operation: $operation"
    return 255
}

echo "   Testing: 5 + 3"
result=$(advanced_calculator add 5 3)
echo "   Result: $result"
echo ""
echo "   Testing: 10 - 4"
result=$(advanced_calculator subtract 10 4)
echo "   Result: $result"
echo ""
echo "   Testing: 4 * 3 (iterative)"
result=$(advanced_calculator multiply 4 3)
echo "   Result: $result"
echo ""

echo "35. Real-world utility function:"
echo ""

process_file() {
    local filename=$1
    local action=$2
    
    if [ ! -f "$filename" ]; then
        echo "   Error: File not found"
        return 1
    fi
    
    if [ "$action" = "count" ]; then
        local lines=$(wc -l < "$filename")
        echo "   Lines in file: $lines"
        return 0
    fi
    
    if [ "$action" = "backup" ]; then
        cp "$filename" "$filename.bak"
        echo "   Backup created: $filename.bak"
        return 0
    fi
    
    echo "   Unknown action: $action"
    return 2
}

echo "   Creating test file..."
echo "line 1" > integration_test.txt
echo "line 2" >> integration_test.txt
echo "line 3" >> integration_test.txt

echo "   Testing count action:"
process_file integration_test.txt count

echo "   Testing backup action:"
process_file integration_test.txt backup

echo "   Cleaning up..."
rm -f integration_test.txt integration_test.txt.bak
echo ""

# =============================================================================
# DEMO COMPLETION
# =============================================================================

echo "=========================================="
echo "    FUNCTION DEMO COMPLETE"
echo "=========================================="
echo ""
echo "Phase 1 Features Demonstrated:"
echo "✓ Basic function definition with body"
echo "✓ Function calls with arguments"
echo "✓ Multiple commands in functions"
echo "✓ Functions calling other functions"
echo "✓ Integration with shell variables and commands"
echo "✓ Error handling for undefined functions"
echo "✓ Positional parameters (\$1, \$2, \$*, \$#, \$0)"
echo ""
echo "Phase 2 Features Demonstrated:"
echo "✓ Local variable declarations with 'local' keyword"
echo "✓ Variable isolation between functions"
echo "✓ Nested function calls with proper scoping"
echo "✓ Local variables vs global variables"
echo "✓ Variable inheritance from outer scopes"
echo "✓ Automatic cleanup of local scopes"
echo ""
echo "Phase 3 Features Demonstrated:"
echo "✓ Return statements (with and without values)"
echo "✓ Early returns from functions"
echo "✓ Conditional returns"
echo "✓ Return value capture with \$?"
echo "✓ Error handling for return outside function"
echo "✓ Recursion with echo output (factorial, fibonacci)"
echo "✓ Mutual recursion (even/odd functions)"
echo "✓ Recursion limit detection (max depth: 500)"
echo "✓ Function introspection (declare -f)"
echo "✓ List all defined functions"
echo "✓ Show specific function definitions"
echo "✓ Function name listing (declare -F)"
echo "✓ Complex function introspection"
echo "✓ Integration of all features (calculator, file processor)"
echo ""
echo "All Phase 1, 2, and 3 requirements from FUNCTIONS_TODO.md are working!"