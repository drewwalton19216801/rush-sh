#!/usr/bin/env rush-sh

# Return Builtin Demo for Rush Shell
# This script demonstrates the POSIX-compliant return builtin command

echo "=========================================="
echo "    RUSH SHELL RETURN BUILTIN DEMO"
echo "=========================================="
echo ""

# =============================================================================
# BASIC RETURN USAGE
# =============================================================================

echo "=== BASIC RETURN USAGE ==="
echo ""

echo "1. Basic return with no arguments (returns 0):"
echo ""

basic_return() {
    echo "   Executing function..."
    return
    echo "   This line should not print"
}

basic_return
echo "   Exit code: $?"
echo ""

echo "2. Return with explicit exit code:"
echo ""

return_with_code() {
    echo "   Returning with code 42"
    return 42
}

return_with_code
echo "   Exit code: $?"
echo ""

# =============================================================================
# EARLY EXIT FROM FUNCTIONS
# =============================================================================

echo "=== EARLY EXIT FROM FUNCTIONS ==="
echo ""

echo "3. Early return to skip remaining code:"
echo ""

early_exit() {
    echo "   Before return"
    return 5
    echo "   After return (should not print)"
    echo "   More code (should not print)"
}

early_exit
echo "   Exit code: $?"
echo ""

echo "4. Conditional early return:"
echo ""

check_positive() {
    if [ "$1" -le 0 ]; then
        echo "   Error: Number must be positive"
        return 1
    fi
    echo "   Number $1 is valid"
    return 0
}

echo "   Testing with 5:"
check_positive 5
echo "   Exit code: $?"
echo ""
echo "   Testing with -3:"
check_positive -3
echo "   Exit code: $?"
echo ""

# =============================================================================
# RETURN IN NESTED FUNCTIONS
# =============================================================================

echo "=== RETURN IN NESTED FUNCTIONS ==="
echo ""

echo "5. Return from nested function:"
echo ""

outer_function() {
    echo "   Outer function start"
    
    inner_function() {
        echo "     Inner function executing"
        return 10
        echo "     This should not print"
    }
    
    inner_function
    local inner_result=$?
    echo "   Inner function returned: $inner_result"
    
    return 20
}

outer_function
echo "   Outer function returned: $?"
echo ""

echo "6. Multiple nested returns:"
echo ""

level1() {
    echo "   Level 1"
    level2
    echo "   Level 1 got: $?"
    return 1
}

level2() {
    echo "   Level 2"
    level3
    echo "   Level 2 got: $?"
    return 2
}

level3() {
    echo "   Level 3"
    return 3
}

level1
echo "   Final exit code: $?"
echo ""

# =============================================================================
# CAPTURING RETURN VALUES
# =============================================================================

echo "=== CAPTURING RETURN VALUES ==="
echo ""

echo "7. Capture return value with \$?:"
echo ""

compute_value() {
    local result=$((5 * 8))
    return "$result"
}

compute_value
captured=$?
echo "   Captured return value: $captured"
echo ""

echo "8. Using return values in conditionals:"
echo ""

is_valid() {
    if [ "$1" -gt 0 ]; then
        if [ "$1" -lt 100 ]; then
            return 0
        fi
    fi
    return 1
}

if is_valid 50; then
    echo "   50 is valid"
else
    echo "   50 is invalid"
fi

if is_valid 150; then
    echo "   150 is valid"
else
    echo "   150 is invalid"
fi
echo ""

# =============================================================================
# RETURN WITH DIFFERENT EXIT CODES
# =============================================================================

echo "=== RETURN WITH DIFFERENT EXIT CODES ==="
echo ""

echo "9. Using different exit codes for different conditions:"
echo ""

process_file() {
    local filename=$1
    
    if [ -z "$filename" ]; then
        echo "   Error: No filename provided"
        return 1
    fi
    
    if [ -f "$filename" ]; then
        echo "   File is valid and readable"
        return 0
    else
        echo "   Error: File does not exist"
        return 2
    fi
}

echo "   Testing with no argument:"
process_file
echo "   Exit code: $?"
echo ""

echo "   Testing with non-existent file:"
process_file "/nonexistent/file.txt"
echo "   Exit code: $?"
echo ""

echo "   Testing with existing file:"
process_file "$0"
echo "   Exit code: $?"
echo ""

# =============================================================================
# PRACTICAL EXAMPLES
# =============================================================================

echo "=== PRACTICAL EXAMPLES ==="
echo ""

echo "10. Calculator function with return codes:"
echo ""

divide() {
    local a=$1
    local b=$2
    
    if [ "$b" -eq 0 ]; then
        echo "   Error: Division by zero"
        return 255
    fi
    
    local result=$((a / b))
    echo "   Result: $result"
    return 0
}

echo "   Testing: 10 / 2"
divide 10 2
echo "   Exit code: $?"
echo ""

echo "   Testing: 10 / 0"
divide 10 0
echo "   Exit code: $?"
echo ""

echo "11. Multiple return codes in a workflow:"
echo ""

check_range() {
    local value=$1
    
    if [ "$value" -lt 1 ]; then
        return 1
    fi
    
    if [ "$value" -gt 100 ]; then
        return 2
    fi
    
    return 0
}

process_number() {
    local num=$1
    
    check_range "$num"
    local range_check=$?
    
    if [ "$range_check" -eq 0 ]; then
        echo "   Number $num is in valid range (1-100)"
        return 0
    fi
    
    if [ "$range_check" -eq 1 ]; then
        echo "   Error: $num is too small (must be >= 1)"
        return 1
    fi
    
    if [ "$range_check" -eq 2 ]; then
        echo "   Error: $num is too large (must be <= 100)"
        return 2
    fi
}

echo "   Testing with 50:"
process_number 50
echo "   Exit code: $?"
echo ""

echo "   Testing with 0:"
process_number 0
echo "   Exit code: $?"
echo ""

echo "   Testing with 150:"
process_number 150
echo "   Exit code: $?"
echo ""

# =============================================================================
# ERROR CASE: RETURN OUTSIDE FUNCTION
# =============================================================================

echo "=== ERROR CASE: RETURN OUTSIDE FUNCTION ==="
echo ""

echo "12. Attempting return outside function (should error):"
echo ""
echo "   return 5"
return 5 2>&1 || echo "   Error: return can only be used within a function (expected)"
echo ""

# =============================================================================
# RETURN WITH COMMAND SUBSTITUTION
# =============================================================================

echo "=== RETURN WITH COMMAND SUBSTITUTION ==="
echo ""

echo "13. Return value based on command substitution:"
echo ""

get_file_count() {
    local dir=$1
    local count=$(ls -1 "$dir" 2>/dev/null | wc -l)
    
    if [ "$count" -eq 0 ]; then
        return 1
    fi
    
    return 0
}

echo "   Testing with /tmp:"
if get_file_count /tmp; then
    echo "   /tmp has files"
else
    echo "   /tmp is empty"
fi
echo ""

# =============================================================================
# DEMO COMPLETION
# =============================================================================

echo "=========================================="
echo "    RETURN BUILTIN DEMO COMPLETE"
echo "=========================================="
echo ""
echo "Features Demonstrated:"
echo "✓ Basic return with no arguments (returns 0)"
echo "✓ Return with explicit exit code"
echo "✓ Early exit from functions"
echo "✓ Conditional returns"
echo "✓ Return in nested functions"
echo "✓ Capturing return values with \$?"
echo "✓ Using return values in conditionals"
echo "✓ Different exit codes for different conditions"
echo "✓ Practical validation and error handling"
echo "✓ Error case: return outside function"
echo "✓ Return with command substitution"
echo ""
echo "The return builtin is fully POSIX-compliant and integrates"
echo "seamlessly with Rush shell's function system!"