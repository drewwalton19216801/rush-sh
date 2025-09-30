#!/usr/bin/env rush-sh

# Parameter Expansion Demo for Rush Shell
# This script demonstrates all POSIX parameter expansion features
# supported by the Rush shell implementation

echo "=========================================="
echo "    RUSH SHELL PARAMETER EXPANSION DEMO"
echo "=========================================="
echo ""

# =============================================================================
# BASIC VARIABLE EXPANSION
# =============================================================================

echo "=== BASIC VARIABLE EXPANSION ==="
echo ""

# Set some test variables
FRUIT="apple"
COUNT=""
MESSAGE="Hello World!"

echo "Original variables:"
echo "  FRUIT='$FRUIT'"
echo "  COUNT='$COUNT' (empty)"
echo "  MESSAGE='$MESSAGE'"
echo ""

# Basic expansion with $VAR syntax
echo "Basic expansion with \$VAR syntax:"
echo "  \$FRUIT = $FRUIT"
echo "  \$COUNT = '$COUNT'"
echo "  \$MESSAGE = $MESSAGE"
echo ""

# Basic expansion with ${VAR} syntax (more explicit)
echo "Basic expansion with \${VAR} syntax:"
echo "  \${FRUIT} = ${FRUIT}"
echo "  \${COUNT} = '${COUNT}'"
echo "  \${MESSAGE} = ${MESSAGE}"
echo ""

# =============================================================================
# DEFAULT VALUE MODIFIERS
# =============================================================================

echo "=== DEFAULT VALUE MODIFIERS ==="
echo ""

# ${VAR:-default} - use default if VAR is unset or null
echo "Default value with \${VAR:-default} (VAR is null/empty):"
echo "  \${COUNT:-10} = ${COUNT:-10}"
echo "  \${UNDEFINED_VAR:-hello} = ${UNDEFINED_VAR:-hello}"
echo ""

# Set COUNT to demonstrate difference
COUNT="5"
echo "After setting COUNT='5':"
echo "  \${COUNT:-10} = ${COUNT:-10}"
echo ""

# ${VAR:=default} - assign default if VAR is unset or null
echo "Assign default with \${VAR:=default} (VAR is null/empty):"
echo "Before: COUNT='$COUNT'"
echo "  \${COUNT:=10} = ${COUNT:=10}"
echo "After: COUNT='$COUNT'"
echo ""

# =============================================================================
# ALTERNATIVE VALUE MODIFIER
# =============================================================================

echo "=== ALTERNATIVE VALUE MODIFIER ==="
echo ""

# ${VAR:+alternative} - use alternative if VAR is set and not null
echo "Alternative value with \${VAR:+alternative} (when VAR is set):"
echo "  \${FRUIT:+orange} = ${FRUIT:+orange}"
echo "  \${COUNT:+zero} = ${COUNT:+zero}"
echo ""

# Test with empty variable
EMPTY_VAR=""
echo "With empty variable:"
echo "  \${EMPTY_VAR:+replacement} = ${EMPTY_VAR:+replacement}"
echo ""

# =============================================================================
# ERROR HANDLING MODIFIER
# =============================================================================

echo "=== ERROR HANDLING MODIFIER ==="
echo ""

# ${VAR:?error} - display error if VAR is unset or null
echo "Error handling with \${VAR:?error} (when VAR is set):"
echo "  \${FRUIT:?error message} = ${FRUIT:?error message}"
echo ""

# This will produce an error for unset variable
echo "Error handling with unset variable (will show error):"
# Note: This will cause the script to exit with an error
# Uncomment to test: echo "  \${UNDEFINED_VAR:?Variable not set} = ${UNDEFINED_VAR:?Variable not set}"
echo "  (Comment shows: \${UNDEFINED_VAR:?Variable not set})"
echo ""

# =============================================================================
# SUBSTRING EXTRACTION
# =============================================================================

echo "=== SUBSTRING EXTRACTION ==="
echo ""

# ${VAR:offset} - substring starting at offset
echo "Substring with \${VAR:offset}:"
echo "  MESSAGE='$MESSAGE'"
echo "  \${MESSAGE:6} = ${MESSAGE:6}"
echo "  \${MESSAGE:0} = ${MESSAGE:0}"
echo "  \${MESSAGE:20} = ${MESSAGE:20}"  # Beyond string length
echo ""

# ${VAR:offset:length} - substring with length
echo "Substring with \${VAR:offset:length}:"
echo "  \${MESSAGE:0:5} = ${MESSAGE:0:5}"
echo "  \${MESSAGE:6:5} = ${MESSAGE:6:5}"
echo "  \${MESSAGE:7:100} = ${MESSAGE:7:100}"  # Longer than remaining string
echo ""

# =============================================================================
# PATTERN REMOVAL FROM BEGINNING
# =============================================================================

echo "=== PATTERN REMOVAL FROM BEGINNING ==="
echo ""

# Set up test string
FILENAME="/usr/local/bin/rush-sh"
echo "Test string: FILENAME='$FILENAME'"
echo ""

# ${VAR#pattern} - remove shortest match from beginning
echo "Remove shortest prefix with \${VAR#pattern}:"
echo "  \${FILENAME#/usr} = ${FILENAME#/usr}"
echo "  \${FILENAME#/usr/local} = ${FILENAME#/usr/local}"
echo ""

# ${VAR##pattern} - remove longest match from beginning
echo "Remove longest prefix with \${VAR##pattern}:"
echo "  \${FILENAME##*/} = ${FILENAME##*/}"
echo "  \${FILENAME##/*/*/} = ${FILENAME##/*/*/}"
echo ""

# =============================================================================
# PATTERN REMOVAL FROM END
# =============================================================================

echo "=== PATTERN REMOVAL FROM END ==="
echo ""

# ${VAR%pattern} - remove shortest match from end
echo "Remove shortest suffix with \${VAR%pattern}:"
echo "  \${FILENAME%/*} = ${FILENAME%/*}"
echo "  \${FILENAME%/bin/*} = ${FILENAME%/bin/*}"
echo ""

# ${VAR%%pattern} - remove longest match from end
echo "Remove longest suffix with \${VAR%%pattern}:"
echo "  \${FILENAME%%/*} = ${FILENAME%%/*}"
echo "  \${FILENAME%%bin/*} = ${FILENAME%%bin/*}"
echo ""

# =============================================================================
# PATTERN SUBSTITUTION
# =============================================================================

echo "=== PATTERN SUBSTITUTION ==="
echo ""

# ${VAR/pattern/replacement} - substitute first match
echo "Substitute first match with \${VAR/pattern/replacement}:"
echo "  Original: '$MESSAGE'"
echo "  \${MESSAGE/World/Universe} = ${MESSAGE/World/Universe}"
echo "  \${MESSAGE/l/L} = ${MESSAGE/l/L}"
echo ""

# ${VAR//pattern/replacement} - substitute all matches
echo "Substitute all matches with \${VAR//pattern/replacement}:"
echo "  Original: '$MESSAGE'"
echo "  \${MESSAGE//l/L} = ${MESSAGE//l/L}"
echo "  \${MESSAGE//o/e} = ${MESSAGE//o/e}"
echo ""

# =============================================================================
# INDIRECT EXPANSION
# =============================================================================

echo "=== INDIRECT EXPANSION ==="
echo ""

# Set up some variables for indirect expansion
echo "Setting up variables for indirect expansion:"
MY_PREFIX_VAR="FRUIT"
echo "  MY_PREFIX_VAR='$MY_PREFIX_VAR'"
echo ""

# ${!prefix*} - names of variables starting with prefix
echo "Indirect expansion with \${!prefix*} (Note: Currently returns empty in this implementation):"
echo "  \${!MY_} = ${!MY_}"
echo "  \${!FRUIT*} = ${!FRUIT*}"
echo ""

# =============================================================================
# PRACTICAL EXAMPLES
# =============================================================================

echo "=== PRACTICAL EXAMPLES ==="
echo ""

# Example 1: Safe directory handling
echo "Example 1 - Safe directory handling:"
USER_DIR=""
echo "  \${USER_DIR:-/tmp} = ${USER_DIR:-/tmp}"
echo ""

# Example 2: Configuration with defaults
echo "Example 2 - Configuration with defaults:"
TIMEOUT=""
DEBUG=""
echo "  TIMEOUT=\${TIMEOUT:-30}"
echo "  DEBUG=\${DEBUG:-false}"
echo "  Configuration: timeout=${TIMEOUT:-30}, debug=${DEBUG:-false}"
echo ""

# Example 3: File extension handling
echo "Example 3 - File extension handling:"
FILENAME="document.txt"
echo "  Original: '$FILENAME'"
echo "  Without extension: \${FILENAME%.txt} = ${FILENAME%.txt}"
echo "  Just extension: \${FILENAME##*.} = ${FILENAME##*.}"
echo ""

# Example 4: Path manipulation
echo "Example 4 - Path manipulation:"
FULL_PATH="/home/user/projects/rush-sh/src/main.rs"
echo "  Original: '$FULL_PATH'"
echo "  Directory only: \${FULL_PATH%/*} = ${FULL_PATH%/*}"
echo "  Filename only: \${FULL_PATH##*/} = ${FULL_PATH##*/}"
echo ""

echo "=========================================="
echo "    PARAMETER EXPANSION DEMO COMPLETE"
echo "=========================================="
echo ""
echo "All POSIX parameter expansion features have been demonstrated!"