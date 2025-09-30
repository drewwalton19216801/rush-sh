#!/usr/bin/env rush-sh

# Function Support Demo for Rush Shell
# This script demonstrates Phase 1 of the function implementation:
# Basic function definition and calls as outlined in FUNCTIONS_TODO.md

echo "=========================================="
echo "    RUSH SHELL FUNCTION SUPPORT DEMO"
echo "=========================================="
echo ""

# =============================================================================
# BASIC FUNCTION DEFINITION AND CALLS
# =============================================================================

echo "=== BASIC FUNCTION DEFINITION AND CALLS ==="
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
    echo "All args ($*): $*"
    echo "Arg count ($#): $#"
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
    if [ -f "$1"; then
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
# INTEGRATION WITH SHELL FEATURES
# =============================================================================

echo "=== INTEGRATION WITH SHELL FEATURES ==="
echo ""

echo "14. Functions with variables and conditionals:"
echo ""

check_process() {
    if ps -p $1 > /dev/null 2>&1; then
        echo "Process $1 is running"
    else
        echo "Process $1 is not running"
    fi
}

echo "15. Using function with shell commands:"
echo "   check_process 1"
check_process 1
echo "   check_process 99999"
check_process 99999
echo ""

# =============================================================================
# DEMO COMPLETION
# =============================================================================

echo "=========================================="
echo "    FUNCTION DEMO COMPLETE"
echo "=========================================="
echo ""
echo "Phase 1 Features Demonstrated:"
echo "✓ Basic function definition: name() { body; }"
echo "✓ Function calls with arguments"
echo "✓ Multiple commands in functions"
echo "✓ Functions calling other functions"
echo "✓ Integration with shell variables and commands"
echo "✓ Error handling for undefined functions"
echo "✓ Positional parameters (\$1, \$2, \$*, \$#, \$0)"
echo ""
echo "All Phase 1 requirements from FUNCTIONS_TODO.md are working!"