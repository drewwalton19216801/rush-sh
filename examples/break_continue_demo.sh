#!/usr/bin/env rush-sh

# Break and Continue Demo Script
# Demonstrates POSIX-compliant loop control flow

echo "=== Break and Continue Demo ==="
echo

# ============================================================================
# Simple break in for loop
# ============================================================================
echo "1. Simple break in for loop:"
echo "   for i in 1 2 3 4 5; do"
echo "     echo \"Processing: \$i\""
echo "     if [ \$i = \"3\" ]; then"
echo "       echo \"Breaking at 3\""
echo "       break"
echo "     fi"
echo "   done"
echo
echo "Output:"
for i in 1 2 3 4 5; do
  echo "  Processing: $i"
  if [ $i = "3" ]; then
    echo "  Breaking at 3"
    break
  fi
done
echo

# ============================================================================
# Simple continue in for loop
# ============================================================================
echo "2. Simple continue in for loop:"
echo "   for i in 1 2 3 4 5; do"
echo "     if [ \$i = \"3\" ]; then"
echo "       echo \"Skipping 3\""
echo "       continue"
echo "     fi"
echo "     echo \"Processing: \$i\""
echo "   done"
echo
echo "Output:"
for i in 1 2 3 4 5; do
  if [ $i = "3" ]; then
    echo "  Skipping 3"
    continue
  fi
  echo "  Processing: $i"
done
echo

# ============================================================================
# Break in while loop
# ============================================================================
echo "3. Break in while loop:"
echo "   i=0"
echo "   while [ \$i -lt 10 ]; do"
echo "     i=\$((i + 1))"
echo "     echo \"Count: \$i\""
echo "     if [ \$i = \"5\" ]; then"
echo "       echo \"Breaking at 5\""
echo "       break"
echo "     fi"
echo "   done"
echo
echo "Output:"
i=0
while [ $i -lt 10 ]; do
  i=$((i + 1))
  echo "  Count: $i"
  if [ $i = "5" ]; then
    echo "  Breaking at 5"
    break
  fi
done
echo

# ============================================================================
# Continue in while loop
# ============================================================================
echo "4. Continue in while loop:"
echo "   i=0"
echo "   while [ \$i -lt 5 ]; do"
echo "     i=\$((i + 1))"
echo "     if [ \$i = \"3\" ]; then"
echo "       echo \"Skipping 3\""
echo "       continue"
echo "     fi"
echo "     echo \"Count: \$i\""
echo "   done"
echo
echo "Output:"
i=0
while [ $i -lt 5 ]; do
  i=$((i + 1))
  if [ $i = "3" ]; then
    echo "  Skipping 3"
    continue
  fi
  echo "  Count: $i"
done
echo

# ============================================================================
# Nested loops with break
# ============================================================================
echo "5. Nested loops with break (breaks inner loop only):"
echo "   for i in 1 2 3; do"
echo "     echo \"Outer: \$i\""
echo "     for j in a b c; do"
echo "       echo \"  Inner: \$j\""
echo "       if [ \$j = \"b\" ]; then"
echo "         echo \"  Breaking inner loop\""
echo "         break"
echo "       fi"
echo "     done"
echo "   done"
echo
echo "Output:"
for i in 1 2 3; do
  echo "  Outer: $i"
  for j in a b c; do
    echo "    Inner: $j"
    if [ $j = "b" ]; then
      echo "    Breaking inner loop"
      break
    fi
  done
done
echo

# ============================================================================
# Nested loops with break 2
# ============================================================================
echo "6. Nested loops with break 2 (breaks both loops):"
echo "   for i in 1 2 3; do"
echo "     echo \"Outer: \$i\""
echo "     for j in a b c; do"
echo "       echo \"  Inner: \$j\""
echo "       if [ \$i = \"2\" ] && [ \$j = \"b\" ]; then"
echo "         echo \"  Breaking both loops\""
echo "         break 2"
echo "       fi"
echo "     done"
echo "   done"
echo
echo "Output:"
for i in 1 2 3; do
  echo "  Outer: $i"
  for j in a b c; do
    echo "    Inner: $j"
    if [ $i = "2" ] && [ $j = "b" ]; then
      echo "    Breaking both loops"
      break 2
    fi
  done
done
echo

# ============================================================================
# Nested loops with continue
# ============================================================================
echo "7. Nested loops with continue (continues inner loop only):"
echo "   for i in 1 2 3; do"
echo "     echo \"Outer: \$i\""
echo "     for j in a b c; do"
echo "       if [ \$j = \"b\" ]; then"
echo "         echo \"  Skipping b\""
echo "         continue"
echo "       fi"
echo "       echo \"  Inner: \$j\""
echo "     done"
echo "   done"
echo
echo "Output:"
for i in 1 2 3; do
  echo "  Outer: $i"
  for j in a b c; do
    if [ $j = "b" ]; then
      echo "    Skipping b"
      continue
    fi
    echo "    Inner: $j"
  done
done
echo

# ============================================================================
# Nested loops with continue 2
# ============================================================================
echo "8. Nested loops with continue 2 (continues outer loop):"
echo "   for i in 1 2 3; do"
echo "     echo \"Outer: \$i\""
echo "     for j in a b c; do"
echo "       echo \"  Inner: \$j\""
echo "       if [ \$i = \"2\" ] && [ \$j = \"b\" ]; then"
echo "         echo \"  Continuing outer loop\""
echo "         continue 2"
echo "       fi"
echo "     done"
echo "     echo \"  Finished inner loop for \$i\""
echo "   done"
echo
echo "Output:"
for i in 1 2 3; do
  echo "  Outer: $i"
  for j in a b c; do
    echo "    Inner: $j"
    if [ $i = "2" ] && [ $j = "b" ]; then
      echo "    Continuing outer loop"
      continue 2
    fi
  done
  echo "  Finished inner loop for $i"
done
echo

# ============================================================================
# Practical example: Processing files with error handling
# ============================================================================
echo "9. Practical example: Processing files with error handling"
echo "   files=\"file1.txt file2.txt file3.txt file4.txt\""
echo "   for file in \$files; do"
echo "     if [ ! -f \"\$file\" ]; then"
echo "       echo \"Skipping missing file: \$file\""
echo "       continue"
echo "     fi"
echo "     echo \"Processing: \$file\""
echo "     # Simulate processing..."
echo "   done"
echo
echo "Output:"
files="file1.txt file2.txt file3.txt file4.txt"
for file in $files; do
  if [ ! -f "$file" ]; then
    echo "  Skipping missing file: $file"
    continue
  fi
  echo "  Processing: $file"
  # Simulate processing...
done
echo

# ============================================================================
# Practical example: Search with early exit
# ============================================================================
echo "10. Practical example: Search with early exit"
echo "    items=\"apple banana cherry date elderberry\""
echo "    target=\"cherry\""
echo "    found=0"
echo "    for item in \$items; do"
echo "      echo \"Checking: \$item\""
echo "      if [ \"\$item\" = \"\$target\" ]; then"
echo "        echo \"Found: \$target\""
echo "        found=1"
echo "        break"
echo "      fi"
echo "    done"
echo "    if [ \$found = \"0\" ]; then"
echo "      echo \"Not found: \$target\""
echo "    fi"
echo
echo "Output:"
items="apple banana cherry date elderberry"
target="cherry"
found=0
for item in $items; do
  echo "  Checking: $item"
  if [ "$item" = "$target" ]; then
    echo "  Found: $target"
    found=1
    break
  fi
done
if [ $found = "0" ]; then
  echo "  Not found: $target"
fi
echo

echo "=== Demo Complete ==="
