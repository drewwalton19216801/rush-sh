#!/usr/bin/env rush-sh

# Arithmetic Expansion Demo for Rush Shell
# This script demonstrates the $((...)) arithmetic expansion feature

echo "=== Rush Shell Arithmetic Expansion Demo ==="
echo

# Basic arithmetic operations
echo "1. Basic Arithmetic Operations:"
echo "   2 + 3 = $((2 + 3))"
echo "   10 * 5 = $((10 * 5))"
echo "   20 / 4 = $((20 / 4))"
echo "   17 % 3 = $((17 % 3))"
echo

# Variable arithmetic
echo "2. Variables in Arithmetic:"
x=15
y=7
echo "   x = $x, y = $y"
echo "   x + y = $((x + y))"
echo "   x * y = $((x * y))"
echo "   x / y = $((x / y))"
echo "   x % y = $((x % y))"
echo

# Complex expressions with precedence
echo "3. Operator Precedence:"
echo "   2 + 3 * 4 = $((2 + 3 * 4))"
echo "   (2 + 3) * 4 = $(((2 + 3) * 4))"
echo "   2 * 3 + 4 * 5 = $((2 * 3 + 4 * 5))"
echo "   2 * (3 + 4) * 5 = $((2 * (3 + 4) * 5))"
echo

# Comparison operations (basic)
echo "4. Comparison Operations (1=true, 0=false):"
echo "   10 == 10 = $((10 == 10))"
echo "   10 != 5 = $((10 != 5))"
echo "   5 > 3 = $((5 > 3))"
echo "   3 > 5 = $((3 > 5))"
echo

# Bitwise operations (basic)
echo "5. Bitwise Operations:"
echo "   5 & 3 = $((5 & 3))"
echo "   5 | 3 = $((5 | 3))"
echo "   5 ^ 3 = $((5 ^ 3))"
echo "   Note: << and >> operators work but may need special handling in scripts"
echo

# Logical operations
echo "6. Logical Operations (1=true, 0=false):"
echo "   5 && 3 = $((5 && 3))"
echo "   5 && 0 = $((5 && 0))"
echo "   0 || 5 = $((0 || 5))"
echo "   0 || 0 = $((0 || 0))"
echo

# Real-world examples
echo "7. Real-world Examples:"
# Calculate area of rectangle
length=10
width=5
echo "   Rectangle: length=$length, width=$width"
echo "   Area = $((length * width))"

# Temperature conversion
celsius=25
fahrenheit=$((celsius * 9 / 5 + 32))
echo "   Temperature: $celsius°C = ${fahrenheit}°F"

# Array length calculation (simulated)
items=8
per_page=3
pages=$(((items + per_page - 1) / per_page))
echo "   Items: $items, Per page: $per_page"
echo "   Pages needed = $pages"
echo

# Error handling demo
echo "8. Error Handling:"
echo "   Division by zero: $((5 / 0)) (will show error)"
echo "   Unmatched parentheses: $((2 + 3 (will show literal)"
echo

echo "=== Demo Complete ==="
echo "Arithmetic expansion allows you to perform mathematical operations"
echo "directly in your shell scripts using the $((expression)) syntax."