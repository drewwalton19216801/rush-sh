#!/bin/bash
# Demo script showing the condensed cwd feature

echo "=== Rush Shell Condensed CWD Demo ==="
echo

echo "1. Default behavior (condensed enabled):"
echo "   RUSH_CONDENSED=true rush-sh"
echo "   # Shows: user@host:/t/d/n/d $"
echo

echo "2. Full path display:"
echo "   RUSH_CONDENSED=false rush-sh"
echo "   # Shows: user@host:/tmp/test/deep/nested/directory $"
echo

echo "3. Runtime toggle in existing shell:"
echo "   set_condensed off    # Disable condensed display"
echo "   set_condensed on     # Re-enable condensed display"
echo "   set_condensed status # Show current setting"
echo

echo "4. Environment variable values:"
echo "   RUSH_CONDENSED=true   # Enable condensed (default)"
echo "   RUSH_CONDENSED=false  # Disable condensed"
echo "   RUSH_CONDENSED=1      # Enable condensed"
echo "   RUSH_CONDENSED=0      # Disable condensed"
echo

echo "=== Demo Complete ==="