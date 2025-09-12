#!/usr/bin/env rush-sh

# Elif example for Rush shell

echo "Testing elif in Rush shell"

if false; then
    echo "This should not print"
elif true; then
    echo "This should print from elif"
else
    echo "This should not print"
fi

echo "Elif test completed"