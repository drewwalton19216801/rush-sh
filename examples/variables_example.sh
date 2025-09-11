#!/usr/bin/env rush

# Variables example for Rush shell
# This script tests environment variable expansion

echo "Testing variables in Rush shell"

# Set a variable
MY_VAR="Hello from Rush"
echo "MY_VAR: $MY_VAR"

# Use in command
echo "Greeting: $MY_VAR"

# Environment variables
echo "User: $USER"
echo "Home: $HOME"
echo "Path: $PATH"

# Curly braces for clarity
echo "Using braces: ${MY_VAR}"

# Variable in pipes
echo "$MY_VAR" | grep "Rush"

# Setting and using multiple
NAME="World"
echo "Hello $NAME"

echo "Variables test completed"