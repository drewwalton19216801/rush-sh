#!/usr/bin/env rush

# Redirections example for Rush shell
# This script tests input and output redirections

echo "Testing redirections in Rush shell"

# Output redirection
echo "Hello from Rush shell" > test_output.txt
echo "Output redirection test completed"

# Append redirection
echo "Appending more text" >> test_output.txt
echo "Append redirection test completed"

# Input redirection
echo "Reading from file:"
cat < test_output.txt

# Combined with pipes and redirections
echo "Combining pipes and redirections:"
echo "line1\nline2\nline3" | grep "line" > filtered.txt
cat < filtered.txt

# Clean up
rm test_output.txt filtered.txt
echo "Redirections test completed and files cleaned up"