#!/usr/bin/env rush-sh

# Pipes example for Rush shell
# This script tests piping commands

echo "Testing pipes in Rush shell"

# Simple pipe: echo to grep
echo "hello world from rush" | grep "rush"

# Pipe with ls and wc
echo "Counting files in current directory:"
ls | wc -l

# Multiple pipes
echo "Listing files and counting lines:"
ls -la | grep "\.rs" | wc -l

echo "Pipes test completed"