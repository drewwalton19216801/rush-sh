#!/usr/bin/env rush-sh

# Positional Parameters Demo for Rush shell
# This script demonstrates the use of positional parameters ($1, $2, $*, $@, $#)
# and the shift command

echo "=== Positional Parameters Demo ==="
echo ""

# Display script name and all arguments
echo "Script name: $0"
echo "Number of arguments: $#"
echo "All arguments as single string (\$*): $*"
echo "All arguments as single string (\$@): $@"
echo ""

# Display individual arguments
echo "Individual arguments:"
echo "  \$1 = $1"
echo "  \$2 = $2"
echo "  \$3 = $3"
echo "  \$4 = $4"
echo "  \$5 = $5"
echo ""

# Demonstrate shift command
if [ $# -gt 0 ]; then
    echo "Shifting arguments by 1..."
    shift
    echo "After shift:"
    echo "  Number of arguments: $#"
    echo "  \$1 = $1"
    echo "  \$2 = $2"
    echo "  \$3 = $3"
    echo ""
fi

# Demonstrate shift with custom count
if [ $# -gt 1 ]; then
    echo "Shifting arguments by 2..."
    shift 2
    echo "After shift 2:"
    echo "  Number of arguments: $#"
    echo "  \$1 = $1"
    echo "  \$2 = $2"
    echo ""
fi

echo "=== Demo completed ==="