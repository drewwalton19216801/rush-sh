#!/usr/bin/env rush-sh

# File Descriptor Redirection Demo for Rush Shell
# This script demonstrates comprehensive file descriptor operations

echo "=== File Descriptor Redirection Demo ==="
echo ""

# Clean up any existing test files
rm -f test_output.txt test_errors.txt test_combined.txt test_fd3.txt test_fd4.txt test_rw.txt

# ============================================================================
# 1. Basic FD Output Redirection (2>)
# ============================================================================
echo "1. Basic FD Output Redirection (2>errors.log)"
echo "   Redirecting stderr to a file..."

# This command will produce an error on stderr
ls /nonexistent_directory 2>test_errors.txt
echo "   Error message captured in test_errors.txt:"
cat test_errors.txt
echo ""

# ============================================================================
# 2. FD Input Redirection (3<)
# ============================================================================
echo "2. FD Input Redirection (3<input.txt)"
echo "   Creating a test file and reading from FD 3..."

# Create a test file
echo "Line 1 from FD 3" > test_fd3.txt
echo "Line 2 from FD 3" >> test_fd3.txt
echo "Line 3 from FD 3" >> test_fd3.txt

# Read from FD 3 (note: this demonstrates the syntax, actual reading requires shell support)
cat 3<test_fd3.txt
echo ""

# ============================================================================
# 3. FD Append (2>>)
# ============================================================================
echo "3. FD Append (2>>errors.log)"
echo "   Appending stderr to existing file..."

# Append more errors to the same file
ls /another_nonexistent 2>>test_errors.txt
echo "   Updated error log:"
cat test_errors.txt
echo ""

# ============================================================================
# 4. FD Duplication (2>&1 and 1>&2)
# ============================================================================
echo "4. FD Duplication (2>&1 - stderr to stdout)"
echo "   Combining stderr and stdout..."

# Redirect stderr to stdout, then capture both
ls /nonexistent 2>&1 > test_combined.txt
echo "   Combined output:"
cat test_combined.txt
echo ""

echo "5. FD Duplication (1>&2 - stdout to stderr)"
echo "   Redirecting stdout to stderr..."

# This will send stdout to stderr (visible in terminal as error)
echo "This message goes to stderr" 1>&2
echo ""

# ============================================================================
# 6. FD Closing (2>&-)
# ============================================================================
echo "6. FD Closing (2>&-)"
echo "   Closing stderr to suppress error messages..."

# Close stderr - error messages will be suppressed
ls /nonexistent 2>&-
echo "   (Error message was suppressed by closing FD 2)"
echo ""

# ============================================================================
# 7. FD Read/Write (3<>)
# ============================================================================
echo "7. FD Read/Write (3<>file.txt)"
echo "   Opening file for both reading and writing..."

# Create a file for read/write operations
echo "Initial content" > test_rw.txt
echo "   Initial content:"
cat test_rw.txt

# Open for read/write (demonstrates syntax)
cat 3<>test_rw.txt
echo "   File opened for read/write on FD 3"
echo ""

# ============================================================================
# 8. Multiple Redirections on One Command
# ============================================================================
echo "8. Multiple Redirections on One Command"
echo "   Redirecting both stdout and stderr separately..."

# Redirect stdout to one file and stderr to another
echo "Success message" >test_output.txt
ls /nonexistent 2>test_errors.txt
echo "   Stdout captured in test_output.txt:"
cat test_output.txt
echo "   Stderr captured in test_errors.txt:"
cat test_errors.txt
echo ""

# ============================================================================
# 9. FD Swap Pattern
# ============================================================================
echo "9. FD Swap Pattern (swapping stdout and stderr)"
echo "   Swapping stdout and stderr using FD 3..."

# This is a classic pattern to swap stdout and stderr
# Note: Full FD swapping requires subshell support
echo "   (FD swapping pattern: 3>&1 1>&2 2>&3 3>&-)"
echo "   This advanced pattern will be demonstrated when subshells are supported"
echo ""

# ============================================================================
# 10. Practical Use Cases
# ============================================================================
echo "10. Practical Use Cases"
echo ""

echo "    a) Separating stdout and stderr for logging:"
echo "       command >output.log 2>error.log"
echo "Normal output" >test_output.txt
ls /nonexistent 2>test_errors.txt
echo "       Output logged separately"
echo ""

echo "    b) Discarding errors while keeping output:"
echo "       command 2>/dev/null"
ls /nonexistent 2>/dev/null
echo "       (Errors discarded)"
echo ""

echo "    c) Logging both streams to the same file:"
echo "       command >combined.log 2>&1"
ls /nonexistent >test_combined.txt 2>&1
echo "       Both streams in test_combined.txt:"
cat test_combined.txt
echo ""

echo "    d) Redirecting to different files:"
echo "       command 1>out.txt 2>err.txt 3>custom.txt"
echo "       (Multiple FDs can be redirected independently)"
echo ""

echo "    e) Pipeline with error handling:"
echo "       command 2>&1 | grep error"
ls /nonexistent 2>&1 | grep -i "cannot"
echo ""

# ============================================================================
# 11. Advanced Patterns
# ============================================================================
echo "11. Advanced Patterns"
echo ""

echo "    a) Saving and restoring file descriptors:"
echo "       exec 3>&1        # Save stdout to FD 3"
echo "       exec 1>file.txt  # Redirect stdout to file"
echo "       echo 'to file'   # Goes to file"
echo "       exec 1>&3        # Restore stdout from FD 3"
echo "       exec 3>&-        # Close FD 3"
echo ""

echo "    b) Error handling in scripts:"
echo "       if ! command 2>error.log; then"
echo "           cat error.log"
echo "           exit 1"
echo "       fi"
echo ""

echo "    c) Logging with timestamps:"
echo "       command 2>&1 | while read line; do"
echo "           echo \"[\$(date)] \$line\""
echo "       done"
echo ""

# ============================================================================
# Cleanup
# ============================================================================
echo "=== Cleanup ==="
echo "Removing test files..."
rm -f test_output.txt test_errors.txt test_combined.txt test_fd3.txt test_fd4.txt test_rw.txt
echo "Demo completed!"
echo ""

# ============================================================================
# Summary
# ============================================================================
echo "=== Summary of FD Operations ==="
echo ""
echo "Basic Redirections:"
echo "  2>file       - Redirect stderr to file"
echo "  3<file       - Open file for reading on FD 3"
echo "  2>>file      - Append stderr to file"
echo ""
echo "FD Duplication:"
echo "  2>&1         - Redirect stderr to stdout"
echo "  1>&2         - Redirect stdout to stderr"
echo "  3>&1         - Redirect FD 3 to stdout"
echo ""
echo "FD Closing:"
echo "  2>&-         - Close stderr"
echo "  3>&-         - Close FD 3"
echo ""
echo "FD Read/Write:"
echo "  3<>file      - Open file for reading and writing on FD 3"
echo ""
echo "Common Patterns:"
echo "  cmd >out.txt 2>err.txt    - Separate stdout and stderr"
echo "  cmd >all.txt 2>&1         - Combine stderr into stdout"
echo "  cmd 2>&1 | grep pattern   - Pipe both streams"
echo "  cmd 2>/dev/null           - Discard errors"
echo "  cmd >/dev/null 2>&1       - Discard all output"
echo ""
