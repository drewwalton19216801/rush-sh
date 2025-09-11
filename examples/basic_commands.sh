#!/usr/bin/env rush

# Basic commands example for Rush shell
# This script tests built-in commands

echo "Testing built-in commands in Rush shell"

# Print working directory
echo "Current directory:"
pwd

# List environment variables
echo "Environment variables:"
env

# Change directory (to /tmp if exists, else stay)
echo "Changing to /tmp"
cd /tmp
pwd

# Echo with variables
echo "Home directory: $HOME"

# Exit (but since it's a script, it will exit the script)
echo "Script completed successfully"