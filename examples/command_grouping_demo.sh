#!/bin/bash
# Command Grouping Demo
# Demonstrates usage of POSIX command grouping { ... } in Rush

# 1. Basic Grouping
echo "--- Basic Grouping ---"
{
    echo "This is inside a group"
    echo "Commands are executed sequentially"
}

# 2. Shared State (Variables)
echo -e "\n--- Variable Persistence ---"
x=1
echo "Before group: x=$x"
{
    x=2
    echo "Inside group: x=$x"
}
echo "After group: x=$x"
echo "(Unlike subshells, variables modified in a group persist)"

# 3. Redirection
echo -e "\n--- Group Redirection ---"
{
    echo "This output"
    echo "is redirected"
    echo "to a file"
} > grouping_output.txt

echo "Content of grouping_output.txt:"
cat grouping_output.txt
rm grouping_output.txt

# 4. Nested Grouping
echo -e "\n--- Nested Grouping ---"
{
    echo "Outer Start"
    {
        echo "  Inner Group"
    }
    echo "Outer End"
}

# 5. Pipeline Interaction
echo -e "\n--- Pipeline Interaction ---"
echo "Piping group output to wc -l:"
{
    echo "line 1"
    echo "line 2"
    echo "line 3"
} | wc -l
echo "Command groups can be part of pipelines just like individual commands."
