#!/usr/bin/env rush
# File Descriptor Redirection Examples
# Demonstrates Rush shell's comprehensive FD management capabilities

echo "=== File Descriptor Redirection Demo ==="
echo

# Example 1: Basic stderr redirection
echo "1. Redirecting stderr to a file (2>file)"
echo "This is stdout"
echo "This is stderr" >&2 2>/tmp/rush_stderr.txt
echo "Stderr was redirected to /tmp/rush_stderr.txt:"
cat /tmp/rush_stderr.txt
rm /tmp/rush_stderr.txt
echo

# Example 2: Combining stdout and stderr (2>&1)
echo "2. Combining stderr with stdout (>file 2>&1)"
sh -c "echo stdout; echo stderr >&2" >/tmp/rush_combined.txt 2>&1
echo "Both outputs captured in /tmp/rush_combined.txt:"
cat /tmp/rush_combined.txt
rm /tmp/rush_combined.txt
echo

# Example 3: Arbitrary FD numbers
echo "3. Using arbitrary file descriptor numbers (3>file)"
sh -c "echo 'Writing to FD 3' >&3" 3>/tmp/rush_fd3.txt
echo "Output written to FD 3:"
cat /tmp/rush_fd3.txt
rm /tmp/rush_fd3.txt
echo

# Example 4: Multiple FD redirections
echo "4. Multiple FD redirections in one command"
sh -c "echo stdout; echo stderr >&2; echo 'FD 3 output' >&3" \
    >/tmp/rush_stdout.txt \
    2>/tmp/rush_stderr.txt \
    3>/tmp/rush_fd3.txt
echo "Stdout:"
cat /tmp/rush_stdout.txt
echo "Stderr:"
cat /tmp/rush_stderr.txt
echo "FD 3:"
cat /tmp/rush_fd3.txt
rm /tmp/rush_stdout.txt /tmp/rush_stderr.txt /tmp/rush_fd3.txt
echo

# Example 5: FD duplication chains
echo "5. FD duplication chains (3>&1 4>&3)"
sh -c "echo 'To FD 4' >&4" 3>&1 4>&3 >/tmp/rush_chain.txt
echo "Output through FD chain:"
cat /tmp/rush_chain.txt
rm /tmp/rush_chain.txt
echo

# Example 6: Closing file descriptors
echo "6. Closing file descriptors (2>&-)"
echo "Running command with stderr closed (errors will be suppressed)"
sh -c "echo 'This works'; echo 'This fails' >&2" 2>&-
echo "Command completed (stderr was closed)"
echo

# Example 7: Appending to FDs
echo "7. Appending to file descriptors (3>>file)"
echo "First write" >/tmp/rush_append.txt
sh -c "echo 'Appended via FD 3' >&3" 3>>/tmp/rush_append.txt
echo "File contents after append:"
cat /tmp/rush_append.txt
rm /tmp/rush_append.txt
echo

# Example 8: Reading from arbitrary FDs
echo "8. Reading from arbitrary FDs (0<&3)"
echo "Input data for FD 3" >/tmp/rush_input.txt
sh -c "cat <&3" 3</tmp/rush_input.txt
rm /tmp/rush_input.txt
echo

# Example 9: Complex redirection scenario
echo "9. Complex scenario: logging with timestamps"
LOGFILE="/tmp/rush_complex.log"
sh -c "
    echo '[INFO] Starting process'
    echo '[ERROR] Simulated error' >&2
    echo '[DEBUG] Debug info' >&3
" >/dev/null 2>&1 3>>"$LOGFILE"
echo "Log file contents:"
cat "$LOGFILE"
rm "$LOGFILE"
echo

echo "=== Demo Complete ==="
echo "All FD redirection features demonstrated successfully!"