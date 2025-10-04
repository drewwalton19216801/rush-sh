#!/usr/bin/env rush-sh

# Trap Builtin Demonstration Script for Rush Shell
# This script demonstrates the comprehensive trap functionality
# including signal handling, EXIT traps, and POSIX compliance

echo "=========================================="
echo "    RUSH SHELL TRAP BUILTIN DEMO"
echo "=========================================="
echo ""

# =============================================================================
# BASIC TRAP SETUP AND DISPLAY
# =============================================================================

echo "=== BASIC TRAP SETUP AND DISPLAY ==="
echo ""

echo "1. Setting up a basic trap for SIGINT (Ctrl+C):"
echo ""
trap 'echo "You pressed Ctrl+C! But the trap caught it."' INT
echo "✓ Trap set for SIGINT"
echo ""

echo "2. Displaying current traps:"
echo "   trap"
trap
echo ""

echo "3. Setting multiple traps:"
echo ""
trap 'echo "TERM signal received - cleaning up..."' TERM
trap 'echo "HUP signal received - reloading config..."' HUP
echo "✓ Traps set for TERM and HUP"
echo ""

echo "4. Displaying all traps after setting multiple:"
echo "   trap"
trap
echo ""

# =============================================================================
# TESTING TRAP FUNCTIONALITY
# =============================================================================

echo "=== TESTING TRAP FUNCTIONALITY ==="
echo ""

echo "5. Testing SIGINT trap (press Ctrl+C now):"
echo "   (The trap should catch it and display the message)"
echo "   Press Ctrl+C to test the trap..."
echo ""

# Give user time to test Ctrl+C
sleep 3
echo "   (If you didn't press Ctrl+C, that's fine - continuing demo)"
echo ""

echo "6. Testing EXIT trap setup:"
echo ""
trap 'echo "=== EXIT TRAP EXECUTED ==="; echo "Cleaning up temporary files..."; echo "Goodbye from Rush shell!"' EXIT
echo "✓ EXIT trap set - will execute when script ends"
echo ""

# =============================================================================
# SIGNAL NAME VS NUMBER DEMONSTRATION
# =============================================================================

echo "=== SIGNAL NAME VS NUMBER DEMONSTRATION ==="
echo ""

echo "7. Setting traps using signal numbers:"
echo ""
trap 'echo "Signal 15 (TERM) received!"' 15
trap 'echo "Signal 2 (INT) received!"' 2
echo "✓ Traps set using signal numbers"
echo ""

echo "8. Displaying traps with signal names:"
echo "   trap"
trap
echo ""

# =============================================================================
# TRAP RESETTING
# =============================================================================

echo "=== TRAP RESETTING ==="
echo ""

echo "9. Resetting specific traps:"
echo ""
echo "Before reset:"
trap
echo ""

trap - INT TERM
echo "✓ Reset INT and TERM traps"
echo ""

echo "After reset:"
trap
echo ""

# =============================================================================
# PRACTICAL USE CASES
# =============================================================================

echo "=== PRACTICAL USE CASES ==="
echo ""

echo "10. Cleanup trap for temporary files:"
echo ""
# Create some temporary files for cleanup demo
TEMP_DIR="/tmp/rush_trap_demo_$$"
mkdir -p "$TEMP_DIR"
echo "test data" > "$TEMP_DIR/test1.txt"
echo "more data" > "$TEMP_DIR/test2.txt"

echo "Created temporary directory: $TEMP_DIR"
echo "Files created:"
ls "$TEMP_DIR"
echo ""

# Set up cleanup trap
trap 'echo "Cleaning up temporary files..."; rm -rf "'"$TEMP_DIR"'"' EXIT
echo "✓ Cleanup trap set for EXIT"
echo ""

echo "11. Signal handling in long-running operations:"
echo ""
trap 'echo "Operation cancelled by user"; exit 1' INT TERM
echo "✓ Cancellation trap set"
echo ""

echo "Simulating long operation (5 seconds)..."
echo "   You can press Ctrl+C to cancel..."
for i in 1 2 3 4 5; do
    sleep 1
    echo "   Progress: $i/5"
done
echo "✓ Long operation completed"
echo ""

# =============================================================================
# ADVANCED TRAP SCENARIOS
# =============================================================================

echo "=== ADVANCED TRAP SCENARIOS ==="
echo ""

echo "12. Multiple commands in trap:"
echo ""
trap 'echo "Multiple commands:"; date; echo "Signal processed"' USR1
echo "✓ Multi-command trap set for USR1"
echo ""

echo "13. Trap with variable expansion:"
echo ""
CURRENT_PID=$$
trap 'echo "Process $CURRENT_PID received signal"' USR2
echo "✓ Trap with variable expansion set for USR2"
echo ""

echo "14. Conditional trap execution:"
echo ""
DEBUG_TRAP=false
if [ "$DEBUG_TRAP" = "true" ]; then
    trap 'echo "Debug: Signal received at $(date)"' TRAP
    echo "✓ Debug trap set for TRAP signal"
else
    echo "Debug trap disabled (DEBUG_TRAP != true)"
fi
echo ""

# =============================================================================
# ERROR HANDLING AND EDGE CASES
# =============================================================================

echo "=== ERROR HANDLING AND EDGE CASES ==="
echo ""

echo "15. Invalid signal handling:"
echo ""
echo "Testing invalid signal (should fail):"
trap 'echo "This should not work"' INVALID_SIGNAL 2>&1 || echo "✓ Correctly handled invalid signal"
echo ""

echo "16. Trap with empty command (should remove trap):"
echo ""
echo "Setting trap with empty command to remove it:"
trap '' HUP
echo "✓ HUP trap removed"
echo ""

# =============================================================================
# INTEGRATION WITH SHELL FEATURES
# =============================================================================

echo "=== INTEGRATION WITH SHELL FEATURES ==="
echo ""

echo "17. Trap with command substitution:"
echo ""
trap 'echo "Files in directory: $(ls | wc -l)"' WINCH
echo "✓ Trap with command substitution set for WINCH"
echo ""

echo "18. Trap in combination with variables:"
echo ""
SCRIPT_START=$(date)
trap 'echo "Script started at: $SCRIPT_START"' EXIT
echo "✓ Trap using variable set at script start"
echo ""

# =============================================================================
# DEMONSTRATION COMPLETION
# =============================================================================

echo "=== DEMONSTRATION COMPLETION ==="
echo ""

echo "19. Final trap status:"
echo "   trap"
trap
echo ""

echo "20. Script completion with EXIT trap:"
echo ""
echo "Script will now end, triggering the EXIT trap..."
echo ""

echo "=========================================="
echo "    TRAP DEMO COMPLETE"
echo "=========================================="
echo ""
echo "Trap Features Demonstrated:"
echo "✓ Setting traps for various signals (INT, TERM, HUP, EXIT, etc.)"
echo "✓ Using both signal names and numbers"
echo "✓ Displaying current traps"
echo "✓ Resetting traps with 'trap -'"
echo "✓ EXIT trap for cleanup on script termination"
echo "✓ Multiple commands in trap handlers"
echo "✓ Variable expansion in trap commands"
echo "✓ Command substitution in traps"
echo "✓ Error handling for invalid signals"
echo "✓ Integration with shell features"
echo "✓ Practical cleanup and signal handling scenarios"
echo ""
echo "The trap builtin provides comprehensive signal handling"
echo "following POSIX specifications for robust shell scripting."