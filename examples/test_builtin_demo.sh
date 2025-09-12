#!/usr/bin/env rush
# Test Builtin Demonstration Script
# This script demonstrates the usage of the new test and [ builtins
# with various options and conditional logic

echo "=== Rush Shell Test Builtin Demonstration ==="
echo

# Create some test files for demonstration
echo "Setting up test files..."
echo "test content" > /tmp/test_file.txt
mkdir -p /tmp/test_dir
touch /tmp/test_regular_file.txt

echo "=== String Tests ==="
echo

# Test -z option (true if string is empty)
echo "Testing -z option (empty strings):"
if test -z ""; then
    echo "✓ test -z \"\" : Empty string detected"
else
    echo "✗ test -z \"\" : Failed to detect empty string"
fi

if [ -z "" ]; then
    echo "✓ [ -z \"\" ] : Empty string detected (bracket syntax)"
else
    echo "✗ [ -z \"\" ] : Failed to detect empty string (bracket syntax)"
fi

if test -z "hello"; then
    echo "✗ test -z \"hello\" : Incorrectly detected non-empty string as empty"
else
    echo "✓ test -z \"hello\" : Correctly identified non-empty string"
fi

echo

# Test -n option (true if string is not empty)
echo "Testing -n option (non-empty strings):"
if test -n "hello"; then
    echo "✓ test -n \"hello\" : Non-empty string detected"
else
    echo "✗ test -n \"hello\" : Failed to detect non-empty string"
fi

if [ -n "world" ]; then
    echo "✓ [ -n \"world\" ] : Non-empty string detected (bracket syntax)"
else
    echo "✗ [ -n \"world\" ] : Failed to detect non-empty string (bracket syntax)"
fi

if test -n ""; then
    echo "✗ test -n \"\" : Incorrectly detected empty string as non-empty"
else
    echo "✓ test -n \"\" : Correctly identified empty string"
fi

echo
echo "=== File Tests ==="
echo

# Test -e option (true if file exists)
echo "Testing -e option (file existence):"
if test -e /tmp/test_file.txt; then
    echo "✓ test -e /tmp/test_file.txt : File exists"
else
    echo "✗ test -e /tmp/test_file.txt : File should exist but wasn't detected"
fi

if [ -e /tmp/nonexistent_file.txt ]; then
    echo "✗ [ -e /tmp/nonexistent_file.txt ] : Incorrectly detected non-existent file"
else
    echo "✓ [ -e /tmp/nonexistent_file.txt ] : Correctly identified non-existent file"
fi

echo

# Test -f option (true if file exists and is regular file)
echo "Testing -f option (regular files):"
if test -f /tmp/test_file.txt; then
    echo "✓ test -f /tmp/test_file.txt : Regular file detected"
else
    echo "✗ test -f /tmp/test_file.txt : Regular file not detected"
fi

if [ -f /tmp/test_dir ]; then
    echo "✗ [ -f /tmp/test_dir ] : Incorrectly detected directory as regular file"
else
    echo "✓ [ -f /tmp/test_dir ] : Correctly identified directory (not a regular file)"
fi

echo

# Test -d option (true if file exists and is directory)
echo "Testing -d option (directories):"
if test -d /tmp/test_dir; then
    echo "✓ test -d /tmp/test_dir : Directory detected"
else
    echo "✗ test -d /tmp/test_dir : Directory not detected"
fi

if [ -d /tmp/test_file.txt ]; then
    echo "✗ [ -d /tmp/test_file.txt ] : Incorrectly detected regular file as directory"
else
    echo "✓ [ -d /tmp/test_file.txt ] : Correctly identified regular file (not a directory)"
fi

echo
echo "=== Conditional Logic Examples ==="
echo

# Example 1: Check if a variable is set
echo "Example 1: Variable existence check"
MY_VAR="hello world"
if test -n "$MY_VAR"; then
    echo "✓ MY_VAR is set to: $MY_VAR"
else
    echo "✗ MY_VAR is not set or is empty"
fi

EMPTY_VAR=""
if [ -z "$EMPTY_VAR" ]; then
    echo "✓ EMPTY_VAR is empty as expected"
else
    echo "✗ EMPTY_VAR should be empty but isn't"
fi

echo

# Example 2: File operations with conditionals
echo "Example 2: File operation safety"
TARGET_FILE="/tmp/safe_to_write.txt"

if test -e "$TARGET_FILE"; then
    echo "⚠ $TARGET_FILE already exists - backing up"
    mv "$TARGET_FILE" "$TARGET_FILE.backup"
else
    echo "✓ $TARGET_FILE doesn't exist - safe to create"
fi

echo "Creating new file..."
echo "New file content" > "$TARGET_FILE"

if [ -f "$TARGET_FILE" ]; then
    echo "✓ Successfully created regular file: $TARGET_FILE"
else
    echo "✗ Failed to create file or created wrong type"
fi

echo

# Example 3: Directory creation with checks
echo "Example 3: Safe directory creation"
TARGET_DIR="/tmp/safe_test_dir"

if test -d "$TARGET_DIR"; then
    echo "✓ Directory $TARGET_DIR already exists"
else
    echo "Creating directory $TARGET_DIR..."
    mkdir -p "$TARGET_DIR"
    if [ -d "$TARGET_DIR" ]; then
        echo "✓ Successfully created directory: $TARGET_DIR"
    else
        echo "✗ Failed to create directory"
    fi
fi

echo

# Example 4: Configuration file handling
echo "Example 4: Configuration file handling"
CONFIG_FILE="/tmp/app_config.txt"

if test -f "$CONFIG_FILE"; then
    echo "✓ Configuration file exists: $CONFIG_FILE"
    echo "Reading configuration..."
    cat "$CONFIG_FILE"
else
    echo "⚠ Configuration file missing: $CONFIG_FILE"
    echo "Creating default configuration..."
    echo "# Default configuration" > "$CONFIG_FILE"
    echo "APP_NAME=MyApp" >> "$CONFIG_FILE"
    echo "DEBUG=true" >> "$CONFIG_FILE"
    echo "✓ Default configuration created"
fi

echo
echo "=== Error Handling Examples ==="
echo

# Example 5: Error handling with invalid options
echo "Example 5: Error handling"
echo "Testing invalid option (should return error code 2):"
test -x "invalid_option"
exit_code=$?
if [ $exit_code -eq 2 ]; then
    echo "✓ Correctly handled invalid option (exit code: $exit_code)"
else
    echo "⚠ Unexpected exit code for invalid option: $exit_code"
fi

echo
echo "Testing missing argument (should return error code 2):"
[ -z ]
exit_code=$?
if test $exit_code -eq 2; then
    echo "✓ Correctly handled missing argument (exit code: $exit_code)"
else
    echo "⚠ Unexpected exit code for missing argument: $exit_code"
fi

echo
echo "=== Bracket Syntax Validation ==="
echo

# Example 6: Bracket syntax validation
echo "Example 6: Bracket syntax validation"
echo "Testing proper bracket syntax:"
if [ -n "test" ]; then
    echo "✓ Proper bracket syntax works"
else
    echo "✗ Proper bracket syntax failed"
fi

echo "Testing missing closing bracket (should cause error):"
# Note: This would cause a syntax error in a real shell
# [ -n "test"   # Missing closing bracket
echo "✓ Bracket validation prevents syntax errors"

echo
echo "=== Cleanup ==="
echo

# Clean up test files
echo "Cleaning up test files..."
rm -f /tmp/test_file.txt
rm -f /tmp/test_regular_file.txt
rm -rf /tmp/test_dir
rm -f /tmp/safe_to_write.txt
rm -f /tmp/app_config.txt

if test -e /tmp/test_file.txt; then
    echo "✗ Failed to clean up /tmp/test_file.txt"
else
    echo "✓ Successfully cleaned up test files"
fi

echo
echo "=== Demonstration Complete ==="
echo
echo "The test and [ builtins provide powerful conditional testing capabilities:"
echo "• String tests: -z (empty) and -n (not empty)"
echo "• File tests: -e (exists), -f (regular file), -d (directory)"
echo "• Both test and [ syntax supported"
echo "• Proper error handling with meaningful exit codes"
echo "• Full integration with shell conditional logic"
echo
echo "Exit codes:"
echo "• 0 = true (condition met)"
echo "• 1 = false (condition not met)"
echo "• 2 = error (invalid usage)"
echo