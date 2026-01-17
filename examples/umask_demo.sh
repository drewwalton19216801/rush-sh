#!/usr/bin/env rush-sh
# umask Demonstration Script
# This script demonstrates the umask builtin functionality in Rush shell

echo "=== Rush Shell umask Demonstration ==="
echo ""

# 1. Display current umask
echo "1. Displaying current umask:"
echo "   Command: umask"
umask
echo ""

# 2. Display umask in symbolic notation
echo "2. Displaying umask in symbolic notation:"
echo "   Command: umask -S"
umask -S
echo ""

# 3. Set umask with octal notation
echo "3. Setting umask with octal notation:"
echo "   Command: umask 022"
umask 022
echo "   New umask: $(umask)"
echo "   Symbolic: $(umask -S)"
echo ""

# 4. Create a file to demonstrate permissions
echo "4. Creating a file to show permission effects:"
echo "   With umask 022, new files get permissions: 644 (rw-r--r--)"
touch /tmp/umask_test_file_022.txt 2>/dev/null
if [ -f /tmp/umask_test_file_022.txt ]; then
    echo "   File created: /tmp/umask_test_file_022.txt"
    ls -l /tmp/umask_test_file_022.txt 2>/dev/null || echo "   (permissions: 644)"
    rm -f /tmp/umask_test_file_022.txt 2>/dev/null
fi
echo ""

# 5. Set a more restrictive umask
echo "5. Setting a more restrictive umask:"
echo "   Command: umask 077"
umask 077
echo "   New umask: $(umask)"
echo "   Symbolic: $(umask -S)"
echo ""

# 6. Create a file with restrictive umask
echo "6. Creating a file with restrictive umask:"
echo "   With umask 077, new files get permissions: 600 (rw-------)"
touch /tmp/umask_test_file_077.txt 2>/dev/null
if [ -f /tmp/umask_test_file_077.txt ]; then
    echo "   File created: /tmp/umask_test_file_077.txt"
    ls -l /tmp/umask_test_file_077.txt 2>/dev/null || echo "   (permissions: 600)"
    rm -f /tmp/umask_test_file_077.txt 2>/dev/null
fi
echo ""

# 7. Set umask with symbolic notation
echo "7. Setting umask with symbolic notation:"
echo "   Command: umask u=rwx,g=rx,o=rx"
umask u=rwx,g=rx,o=rx
echo "   New umask: $(umask)"
echo "   Symbolic: $(umask -S)"
echo ""

# 8. Another symbolic notation example
echo "8. Another symbolic notation example:"
echo "   Command: umask u=rwx,g=,o="
umask u=rwx,g=,o=
echo "   New umask: $(umask)"
echo "   Symbolic: $(umask -S)"
echo ""

# 9. Demonstrate umask in subshells
echo "9. Demonstrating umask in subshells:"
echo "   Setting umask to 022 in parent shell"
umask 022
echo "   Parent umask: $(umask)"
echo "   Creating subshell and changing umask to 077"
( 
    umask 077 ; 
    echo "   Subshell umask: $(umask)"
)
echo "   Parent umask after subshell: $(umask)"
echo "   (umask changes in subshells don't affect parent)"
echo ""

# 10. Common umask values
echo "10. Common umask values and their meanings:"
echo "    022 - Default for most systems (files: 644, dirs: 755)"
echo "    002 - Group-writable (files: 664, dirs: 775)"
echo "    027 - Group-readable only (files: 640, dirs: 750)"
echo "    077 - Private to user (files: 600, dirs: 700)"
echo ""

# 11. Error handling examples
echo "11. Error handling examples:"
echo "    Invalid octal value:"
echo "    Command: umask 999"
umask 999 2>&1 || echo "    (Error: invalid octal value)"
echo ""
echo "    Invalid symbolic notation:"
echo "    Command: umask invalid"
umask invalid 2>&1 || echo "    (Error: invalid symbolic notation)"
echo ""

# 12. Restore default umask
echo "12. Restoring default umask:"
echo "    Command: umask 022"
umask 022
echo "    Final umask: $(umask)"
echo "    Symbolic: $(umask -S)"
echo ""

echo "=== umask Demonstration Complete ==="
echo ""
echo "Key Points:"
echo "  - umask controls default permissions for new files and directories"
echo "  - Lower umask = more permissive (e.g., 022)"
echo "  - Higher umask = more restrictive (e.g., 077)"
echo "  - Octal notation: 3 digits representing user/group/other permissions to REMOVE"
echo "  - Symbolic notation: u=rwx,g=rx,o=rx format for permissions to KEEP"
echo "  - Changes in subshells don't affect the parent shell"
