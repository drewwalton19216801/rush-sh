#!/usr/bin/env rush

#!/usr/bin/env rush

# Complex example for Rush shell
# This script demonstrates multiple features: variables, pipes, redirections,
# built-ins, and command substitution

echo "Testing complex features in Rush shell"

# Set variables using command substitution
PROJECT_DIR="$(pwd)/src"
OUTPUT_FILE="complex_output.txt"
CURRENT_USER="$(whoami)"
FILE_COUNT="$(ls *.rs 2>/dev/null | wc -l)"

# Use variables in commands
echo "Project directory: $PROJECT_DIR"
echo "Current user: $CURRENT_USER"
echo "Rust files found: $FILE_COUNT"

cd "$PROJECT_DIR"
pwd

# Command substitution in assignments and commands
echo "Current directory (via substitution): $(pwd)"
echo "Date and time: $(date)"

# Pipe with redirection and variables
echo "Listing Rust files and counting them:"
ls *.rs | wc -l > "$OUTPUT_FILE"

# Read back the result
echo "Number of Rust files:"
cat < "$OUTPUT_FILE"

# More complex pipe chain with command substitution
echo "Finding .rs files with 'fn' and counting:"
grep -l "fn" *.rs | wc -l >> "$OUTPUT_FILE"

# Command substitution with backticks (alternative syntax)
echo "System information: `uname -a`"

# Command substitution in variable assignments
RUST_VERSION="$(rustc --version | cut -d' ' -f2)"
echo "Rust version: $RUST_VERSION"

# Command substitution with error handling
NONEXISTENT="$(nonexistent_command 2>/dev/null || echo 'command failed')"
echo "Nonexistent command result: $NONEXISTENT"

# Command substitution in complex expressions
echo "Files modified today: $(find . -name '*.rs' -mtime -1 | wc -l)"

# Display environment with command substitution
echo "Shell environment (first PATH entry):"
echo "$PATH" | tr ':' '\n' | head -1

# Command substitution with multiple commands
echo "Combined output: $(echo 'Hello'; echo 'World')"

# Clean up
rm "$OUTPUT_FILE"
echo "Complex test with command substitution completed and cleaned up"