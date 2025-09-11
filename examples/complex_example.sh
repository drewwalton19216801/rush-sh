#!/usr/bin/env rush

# Complex example for Rush shell
# This script combines multiple features: variables, pipes, redirections, built-ins

echo "Testing complex features in Rush shell"

# Set variables
PROJECT_DIR="/home/drew/projects/rust/rush"
OUTPUT_FILE="complex_output.txt"

# Use variables in commands
echo "Project directory: $PROJECT_DIR"
cd "$PROJECT_DIR"
pwd

# Pipe with redirection and variables
echo "Listing Rust files and counting them:"
ls *.rs | wc -l > "$OUTPUT_FILE"

# Read back the result
echo "Number of Rust files:"
cat < "$OUTPUT_FILE"

# More complex pipe chain
echo "Finding .rs files with 'fn' and counting:"
grep -l "fn" *.rs | wc -l >> "$OUTPUT_FILE"

# Display environment
echo "Shell environment:"
env | grep "PATH" | head -1

# Clean up
rm "$OUTPUT_FILE"
echo "Complex test completed and cleaned up"