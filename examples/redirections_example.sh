#!/usr/bin/env rush-sh

# Redirections example for Rush shell
# This script tests input and output redirections

echo "Testing redirections in Rush shell"
echo ""

# ============================================================================
# Basic Redirections
# ============================================================================
echo "=== Basic Redirections ==="

# Output redirection
echo "Hello from Rush shell" > test_output.txt
echo "Output redirection test completed"

# Append redirection
echo "Appending more text" >> test_output.txt
echo "Append redirection test completed"

# Input redirection
echo "Reading from file:"
cat < test_output.txt
echo ""

# ============================================================================
# File Descriptor Redirections
# ============================================================================
echo "=== File Descriptor Redirections ==="

# Redirect stderr to a file
echo "Testing stderr redirection..."
ls /nonexistent_directory 2>test_errors.txt
echo "Stderr captured in test_errors.txt:"
cat test_errors.txt
echo ""

# Redirect stdout and stderr to different files
echo "Separating stdout and stderr..."
echo "This is stdout" >test_stdout.txt
ls /nonexistent 2>test_stderr.txt
echo "Stdout:"
cat test_stdout.txt
echo "Stderr:"
cat test_stderr.txt
echo ""

# Combine stderr into stdout
echo "Combining stderr into stdout..."
ls /nonexistent 2>test_combined.txt
echo "Combined output:"
cat test_combined.txt
echo ""

# Append stderr to a file
echo "Appending stderr..."
ls /another_nonexistent 2>>test_errors.txt
echo "Updated error log:"
cat test_errors.txt
echo ""

# ============================================================================
# Combined with pipes and redirections
# ============================================================================
echo "=== Combining pipes and redirections ==="
echo "line1\nline2\nline3" | grep "line" > filtered.txt
cat < filtered.txt
echo ""

# ============================================================================
# Clean up
# ============================================================================
echo "=== Cleanup ==="
rm -f test_output.txt filtered.txt test_errors.txt test_stdout.txt test_stderr.txt test_combined.txt
echo "Redirections test completed and files cleaned up"
echo ""
echo "For more advanced file descriptor examples, see: examples/fd_redirection_demo.sh"