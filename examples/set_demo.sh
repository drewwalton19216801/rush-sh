#!/usr/bin/env rush-sh
# Comprehensive demonstration of the set builtin command
# This script showcases all major features of the set builtin

echo "=== Rush Shell: set Builtin Demonstration ==="
echo

# ============================================================================
# Part 1: Display Modes
# ============================================================================
echo "--- Part 1: Display Modes ---"
echo

echo "1.1: Display all shell variables (set with no arguments)"
TEST_VAR="example_value"
ANOTHER_VAR="another_example"
set | grep -E "(TEST_VAR|ANOTHER_VAR)"
echo

echo "1.2: Display all shell options (set +o)"
set +o
echo

# ============================================================================
# Part 2: Basic Option Management
# ============================================================================
echo "--- Part 2: Basic Option Management ---"
echo

echo "2.1: Enable errexit (-e) - exit on command failure"
echo "Before: errexit is off"
set +o | grep errexit
set -e
echo "After: errexit is on"
set +o | grep errexit
set +e  # Disable for rest of demo
echo

echo "2.2: Enable nounset (-u) - error on unset variables"
set -u
echo "With nounset enabled, accessing undefined variables causes errors"
# Uncomment to see error: echo $UNDEFINED_VAR
set +u  # Disable for rest of demo
echo "nounset disabled"
echo

echo "2.3: Enable xtrace (-x) - print commands before execution"
set -x
echo "This command will be printed before execution"
VAR="test"
echo "Variable value: $VAR"
set +x
echo "xtrace disabled (this line not traced)"
echo

# ============================================================================
# Part 3: Combined Options
# ============================================================================
echo "--- Part 3: Combined Options ---"
echo

echo "3.1: Enable multiple options at once (-eux)"
set -eux
echo "errexit, nounset, and xtrace are now enabled"
set +eux
echo "All three options disabled"
echo

echo "3.2: Mix enable and disable operations"
set -e +u -x
echo "errexit and xtrace on, nounset off"
set +ex
echo "All disabled"
echo

# ============================================================================
# Part 4: Named Options
# ============================================================================
echo "--- Part 4: Named Options (-o and +o) ---"
echo

echo "4.1: Enable option by name"
set -o errexit
echo "errexit enabled via -o errexit"
set +o | grep errexit
set +o errexit
echo

echo "4.2: Disable option by name"
set -o xtrace
echo "xtrace enabled"
set +o xtrace
echo "xtrace disabled via +o xtrace"
echo

# ============================================================================
# Part 5: Positional Parameters
# ============================================================================
echo "--- Part 5: Positional Parameters ---"
echo

echo "5.1: Set positional parameters"
set -- arg1 arg2 arg3 arg4
echo "Positional parameters set to: arg1 arg2 arg3 arg4"
echo "  \$1 = $1"
echo "  \$2 = $2"
echo "  \$3 = $3"
echo "  \$4 = $4"
echo "  \$# = $#"
echo "  \$* = $*"
echo

echo "5.2: Clear positional parameters"
set --
echo "Positional parameters cleared"
echo "  \$# = $#"
echo "  \$1 = ${1:-<not set>}"
echo

echo "5.3: Combine options with positional parameters"
set -e -- new_arg1 new_arg2
echo "errexit enabled and positional parameters set"
echo "  \$1 = $1"
echo "  \$2 = $2"
set +e --
echo

# ============================================================================
# Part 6: Practical Use Cases
# ============================================================================
echo "--- Part 6: Practical Use Cases ---"
echo

echo "6.1: Strict mode for safer scripts"
echo "set -euo pipefail  # (pipefail not yet implemented)"
set -eu
echo "Strict mode enabled (errexit + nounset)"
set +eu
echo

echo "6.2: Debug mode with xtrace"
set -x
echo "Debug mode: commands are printed before execution"
for i in 1 2 3; do
    echo "Iteration $i"
done
set +x
echo

echo "6.3: Syntax check mode (noexec)"
echo "set -n enables syntax checking without execution"
# Note: In noexec mode, commands after set -n won't execute
# Uncomment to test: set -n
# echo "This won't be printed"
# set +n
echo "noexec demonstration skipped (would prevent rest of script)"
echo

echo "6.4: Disable globbing (noglob)"
set -f
echo "With noglob enabled, wildcards are literal:"
echo *.txt
set +f
echo "With noglob disabled, wildcards expand:"
echo *.txt 2>/dev/null || echo "(no .txt files found)"
echo

echo "6.5: Prevent file overwrites (noclobber)"
set -C
echo "test data" > /tmp/rush_set_demo_test.txt
echo "File created: /tmp/rush_set_demo_test.txt"
echo
echo "Attempting to overwrite with > (should fail)..."
echo "new data" > /tmp/rush_set_demo_test.txt 2>&1 || echo "✗ Error: noclobber prevented overwrite (expected)"
echo
echo "Using >| operator to force overwrite (noclobber override):"
echo "overwritten data" >| /tmp/rush_set_demo_test.txt
cat /tmp/rush_set_demo_test.txt
echo "✓ File successfully overwritten with >| operator"
echo
echo "Appending with >> still works (not affected by noclobber):"
echo "appended line" >> /tmp/rush_set_demo_test.txt
cat /tmp/rush_set_demo_test.txt
echo "✓ Append operation successful"
set +C
rm -f /tmp/rush_set_demo_test.txt
echo

echo "6.6: Auto-export variables (allexport)"
set -a
AUTO_EXPORTED_VAR="This variable is automatically exported"
set +a
echo "Variable set with allexport enabled"
# In a real shell, this would be in the environment
# env | grep AUTO_EXPORTED_VAR
echo

# ============================================================================
# Part 7: Option Display and Inspection
# ============================================================================
echo "--- Part 7: Option Display and Inspection ---"
echo

echo "7.1: Display specific option status"
set -e
set +o | grep errexit
set +e
echo

echo "7.2: Display all options with current state"
set -eu
echo "With errexit and nounset enabled:"
set +o | head -5
set +eu
echo

# ============================================================================
# Part 8: Error Handling
# ============================================================================
echo "--- Part 8: Error Handling ---"
echo

echo "8.1: Invalid option handling"
set -Z 2>&1 || echo "Error: Invalid option -Z (expected)"
echo

echo "8.2: Display options with -o (no argument)"
set -o | head -3
echo "(POSIX: set -o without argument displays all options)"
echo

echo "8.3: Invalid named option"
set -o invalid_option 2>&1 || echo "Error: Invalid option name (expected)"
echo

# ============================================================================
# Part 9: Advanced Patterns
# ============================================================================
echo "--- Part 9: Advanced Patterns ---"
echo

echo "9.1: Temporarily enable option"
echo "Original state: xtrace off"
set -x
echo "xtrace temporarily enabled"
set +x
echo "xtrace disabled again"
echo

echo "9.2: Custom PS4 for xtrace"
PS4='+ [${LINENO}] '
set -x
echo "Traced with custom PS4 showing line numbers"
set +x
PS4='+ '
echo

echo "9.3: Option state preservation pattern"
# Save current errexit state
SAVED_ERREXIT=$(set +o | grep errexit | grep -q "on" && echo "on" || echo "off")
set +e
echo "errexit temporarily disabled"
# Restore errexit state
if [ "$SAVED_ERREXIT" = "on" ]; then
    set -e
fi
echo "errexit state restored"
echo

# ============================================================================
# Summary
# ============================================================================
echo "=== Demonstration Complete ==="
echo
echo "Key Takeaways:"
echo "  - Use 'set' alone to display all variables"
echo "  - Use 'set +o' to display all options"
echo "  - Use '-' to enable options, '+' to disable"
echo "  - Combine multiple short options: set -eux"
echo "  - Use named options: set -o errexit"
echo "  - Manage positional parameters: set -- arg1 arg2"
echo "  - Combine options and parameters: set -e -- args"
echo
echo "Common patterns:"
echo "  - Strict mode: set -eu"
echo "  - Debug mode: set -x"
echo "  - Syntax check: set -n"
echo "  - Protect files: set -C"
echo
echo "For more information, see the Rush shell documentation."