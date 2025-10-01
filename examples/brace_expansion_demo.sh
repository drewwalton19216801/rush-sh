#!/usr/bin/env rush-sh
# Brace Expansion Demonstration Script
# This script demonstrates all brace expansion features in Rush shell

echo "=== Brace Expansion Demo ==="
echo ""

echo "1. Simple comma-separated lists:"
echo "   Pattern: {a,b,c}"
echo "   Result: " {a,b,c}
echo ""

echo "2. Numeric ranges:"
echo "   Pattern: {1..5}"
echo "   Result: " {1..5}
echo ""

echo "3. Alphabetic ranges:"
echo "   Pattern: {a..e}"
echo "   Result: " {a..e}
echo ""

echo "4. Prefix and suffix combinations:"
echo "   Pattern: file{1,2,3}.txt"
echo "   Result: " file{1,2,3}.txt
echo ""

echo "5. Nested brace expansion:"
echo "   Pattern: {{a,b},{c,d}}"
echo "   Result: " {{a,b},{c,d}}
echo ""

echo "6. Multiple brace patterns:"
echo "   Pattern: {a,b}{1,2}"
echo "   Result: " {a,b}{1,2}
echo ""

echo "7. Complex nested patterns:"
echo "   Pattern: prefix_{x,y,z}_suffix"
echo "   Result: " prefix_{x,y,z}_suffix
echo ""

echo "8. Numeric range with prefix:"
echo "   Pattern: test{1..3}.log"
echo "   Result: " test{1..3}.log
echo ""

echo "9. Alphabetic range with suffix:"
echo "   Pattern: file{a..c}.txt"
echo "   Result: " file{a..c}.txt
echo ""

echo "10. Nested with ranges:"
echo "    Pattern: {{1..3},{a..c}}"
echo "    Result: " {{1..3},{a..c}}
echo ""

echo "=== Practical Examples ==="
echo ""

echo "Creating multiple files (simulation):"
echo "touch document{1..5}.txt"
echo "Would create: " document{1..5}.txt
echo ""

echo "Creating directory structure (simulation):"
echo "mkdir -p project/{src,test,docs}"
echo "Would create: " project/{src,test,docs}
echo ""

echo "Batch file operations (simulation):"
echo "cp photo{1..3}.jpg backup/"
echo "Would copy: " photo{1..3}.jpg
echo ""

echo "=== Demo Complete ==="