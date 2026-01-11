#!/usr/bin/env rush-sh

# Until Loop Demo Script
# Demonstrates POSIX-compliant until loop functionality

echo "=== Until Loop Demo ==="
echo

# ============================================================================
# Simple until loop
# ============================================================================
echo "1. Simple until loop:"
echo "   count=0"
echo "   until [ \$count -eq 5 ]; do"
echo "     echo \"Count: \$count\""
echo "     count=\$((count + 1))"
echo "   done"
echo
echo "Output:"
count=0
until [ $count -eq 5 ]; do
  echo "  Count: $count"
  count=$((count + 1))
done
echo

# ============================================================================
# Until loop with variable modification
# ============================================================================
echo "2. Until loop with variable modification:"
echo "   value=10"
echo "   until [ \$value -le 0 ]; do"
echo "     echo \"Countdown: \$value\""
echo "     value=\$((value - 1))"
echo "   done"
echo "   echo \"Liftoff!\""
echo
echo "Output:"
value=10
until [ $value -le 0 ]; do
  echo "  Countdown: $value"
  value=$((value - 1))
done
echo "  Liftoff!"
echo

# ============================================================================
# Until loop with break
# ============================================================================
echo "3. Until loop with break:"
echo "   i=0"
echo "   until [ \$i -eq 100 ]; do"
echo "     echo \"Iteration: \$i\""
echo "     i=\$((i + 1))"
echo "     if [ \$i -eq 5 ]; then"
echo "       echo \"Breaking at 5\""
echo "       break"
echo "     fi"
echo "   done"
echo
echo "Output:"
i=0
until [ $i -eq 100 ]; do
  echo "  Iteration: $i"
  i=$((i + 1))
  if [ $i -eq 5 ]; then
    echo "  Breaking at 5"
    break
  fi
done
echo

# ============================================================================
# Until loop with continue
# ============================================================================
echo "4. Until loop with continue:"
echo "   num=0"
echo "   until [ \$num -eq 10 ]; do"
echo "     num=\$((num + 1))"
echo "     if [ \$((num % 2)) -eq 0 ]; then"
echo "       continue"
echo "     fi"
echo "     echo \"Odd number: \$num\""
echo "   done"
echo
echo "Output:"
num=0
until [ $num -eq 10 ]; do
  num=$((num + 1))
  if [ $((num % 2)) -eq 0 ]; then
    continue
  fi
  echo "  Odd number: $num"
done
echo

# ============================================================================
# Nested until loops
# ============================================================================
echo "5. Nested until loops:"
echo "   outer=0"
echo "   until [ \$outer -eq 3 ]; do"
echo "     echo \"Outer: \$outer\""
echo "     inner=0"
echo "     until [ \$inner -eq 3 ]; do"
echo "       echo \"  Inner: \$inner\""
echo "       inner=\$((inner + 1))"
echo "     done"
echo "     outer=\$((outer + 1))"
echo "   done"
echo
echo "Output:"
outer=0
until [ $outer -eq 3 ]; do
  echo "  Outer: $outer"
  inner=0
  until [ $inner -eq 3 ]; do
    echo "    Inner: $inner"
    inner=$((inner + 1))
  done
  outer=$((outer + 1))
done
echo

# ============================================================================
# Nested until loops with break
# ============================================================================
echo "6. Nested until loops with break (breaks inner loop only):"
echo "   x=0"
echo "   until [ \$x -eq 3 ]; do"
echo "     echo \"Outer: \$x\""
echo "     y=0"
echo "     until [ \$y -eq 5 ]; do"
echo "       echo \"  Inner: \$y\""
echo "       if [ \$y -eq 2 ]; then"
echo "         echo \"  Breaking inner loop\""
echo "         break"
echo "       fi"
echo "       y=\$((y + 1))"
echo "     done"
echo "     x=\$((x + 1))"
echo "   done"
echo
echo "Output:"
x=0
until [ $x -eq 3 ]; do
  echo "  Outer: $x"
  y=0
  until [ $y -eq 5 ]; do
    echo "    Inner: $y"
    if [ $y -eq 2 ]; then
      echo "    Breaking inner loop"
      break
    fi
    y=$((y + 1))
  done
  x=$((x + 1))
done
echo

# ============================================================================
# Nested until loops with break 2
# ============================================================================
echo "7. Nested until loops with break 2 (breaks both loops):"
echo "   a=0"
echo "   until [ \$a -eq 5 ]; do"
echo "     echo \"Outer: \$a\""
echo "     b=0"
echo "     until [ \$b -eq 5 ]; do"
echo "       echo \"  Inner: \$b\""
echo "       if [ \$a -eq 2 ] && [ \$b -eq 2 ]; then"
echo "         echo \"  Breaking both loops\""
echo "         break 2"
echo "       fi"
echo "       b=\$((b + 1))"
echo "     done"
echo "     a=\$((a + 1))"
echo "   done"
echo
echo "Output:"
a=0
until [ $a -eq 5 ]; do
  echo "  Outer: $a"
  b=0
  until [ $b -eq 5 ]; do
    echo "    Inner: $b"
    if [ $a -eq 2 ] && [ $b -eq 2 ]; then
      echo "    Breaking both loops"
      break 2
    fi
    b=$((b + 1))
  done
  a=$((a + 1))
done
echo

# ============================================================================
# Nested until loops with continue
# ============================================================================
echo "8. Nested until loops with continue (continues inner loop only):"
echo "   m=0"
echo "   until [ \$m -eq 3 ]; do"
echo "     echo \"Outer: \$m\""
echo "     n=0"
echo "     until [ \$n -eq 4 ]; do"
echo "       n=\$((n + 1))"
echo "       if [ \$n -eq 2 ]; then"
echo "         echo \"  Skipping 2\""
echo "         continue"
echo "       fi"
echo "       echo \"  Inner: \$n\""
echo "     done"
echo "     m=\$((m + 1))"
echo "   done"
echo
echo "Output:"
m=0
until [ $m -eq 3 ]; do
  echo "  Outer: $m"
  n=0
  until [ $n -eq 4 ]; do
    n=$((n + 1))
    if [ $n -eq 2 ]; then
      echo "    Skipping 2"
      continue
    fi
    echo "    Inner: $n"
  done
  m=$((m + 1))
done
echo

# ============================================================================
# Nested until loops with continue 2
# ============================================================================
echo "9. Nested until loops with continue 2 (continues outer loop):"
echo "   p=0"
echo "   until [ \$p -eq 3 ]; do"
echo "     echo \"Outer: \$p\""
echo "     q=0"
echo "     until [ \$q -eq 4 ]; do"
echo "       echo \"  Inner: \$q\""
echo "       if [ \$p -eq 1 ] && [ \$q -eq 2 ]; then"
echo "         echo \"  Continuing outer loop\""
echo "         p=\$((p + 1))"
echo "         continue 2"
echo "       fi"
echo "       q=\$((q + 1))"
echo "     done"
echo "     echo \"  Finished inner loop for \$p\""
echo "     p=\$((p + 1))"
echo "   done"
echo
echo "Output:"
p=0
until [ $p -eq 3 ]; do
  echo "  Outer: $p"
  q=0
  until [ $q -eq 4 ]; do
    echo "    Inner: $q"
    if [ $p -eq 1 ] && [ $q -eq 2 ]; then
      echo "    Continuing outer loop"
      p=$((p + 1))
      continue 2
    fi
    q=$((q + 1))
  done
  echo "  Finished inner loop for $p"
  p=$((p + 1))
done
echo

# ============================================================================
# Practical example: Waiting for a condition
# ============================================================================
echo "10. Practical example: Waiting for a condition"
echo "    attempts=0"
echo "    max_attempts=5"
echo "    success=0"
echo "    until [ \$success -eq 1 ] || [ \$attempts -ge \$max_attempts ]; do"
echo "      attempts=\$((attempts + 1))"
echo "      echo \"Attempt \$attempts of \$max_attempts...\""
echo "      # Simulate random success (succeed on attempt 3)"
echo "      if [ \$attempts -eq 3 ]; then"
echo "        success=1"
echo "        echo \"Success!\""
echo "      fi"
echo "    done"
echo "    if [ \$success -eq 0 ]; then"
echo "      echo \"Failed after \$max_attempts attempts\""
echo "    fi"
echo
echo "Output:"
attempts=0
max_attempts=5
success=0
until [ $success -eq 1 ] || [ $attempts -ge $max_attempts ]; do
  attempts=$((attempts + 1))
  echo "  Attempt $attempts of $max_attempts..."
  # Simulate random success (succeed on attempt 3)
  if [ $attempts -eq 3 ]; then
    success=1
    echo "  Success!"
  fi
done
if [ $success -eq 0 ]; then
  echo "  Failed after $max_attempts attempts"
fi
echo

# ============================================================================
# Practical example: Countdown timer
# ============================================================================
echo "11. Practical example: Countdown timer"
echo "    seconds=5"
echo "    echo \"Starting countdown from \$seconds...\""
echo "    until [ \$seconds -le 0 ]; do"
echo "      echo \"\$seconds...\""
echo "      seconds=\$((seconds - 1))"
echo "    done"
echo "    echo \"Time's up!\""
echo
echo "Output:"
seconds=5
echo "  Starting countdown from $seconds..."
until [ $seconds -le 0 ]; do
  echo "  $seconds..."
  seconds=$((seconds - 1))
done
echo "  Time's up!"
echo

# ============================================================================
# Practical example: Retry logic with exponential backoff
# ============================================================================
echo "12. Practical example: Retry logic with exponential backoff"
echo "    retry=0"
echo "    max_retries=4"
echo "    delay=1"
echo "    connected=0"
echo "    until [ \$connected -eq 1 ] || [ \$retry -ge \$max_retries ]; do"
echo "      retry=\$((retry + 1))"
echo "      echo \"Connection attempt \$retry (delay: \${delay}s)...\""
echo "      # Simulate connection (succeed on attempt 3)"
echo "      if [ \$retry -eq 3 ]; then"
echo "        connected=1"
echo "        echo \"Connected!\""
echo "      else"
echo "        echo \"Failed. Retrying in \${delay}s...\""
echo "        delay=\$((delay * 2))"
echo "      fi"
echo "    done"
echo
echo "Output:"
retry=0
max_retries=4
delay=1
connected=0
until [ $connected -eq 1 ] || [ $retry -ge $max_retries ]; do
  retry=$((retry + 1))
  echo "  Connection attempt $retry (delay: ${delay}s)..."
  # Simulate connection (succeed on attempt 3)
  if [ $retry -eq 3 ]; then
    connected=1
    echo "  Connected!"
  else
    echo "  Failed. Retrying in ${delay}s..."
    delay=$((delay * 2))
  fi
done
echo

# ============================================================================
# Comparison: while vs until
# ============================================================================
echo "13. Comparison: while vs until"
echo
echo "While loop (continues while condition is true):"
echo "   i=0"
echo "   while [ \$i -lt 3 ]; do"
echo "     echo \"i = \$i\""
echo "     i=\$((i + 1))"
echo "   done"
echo
echo "Output:"
i=0
while [ $i -lt 3 ]; do
  echo "  i = $i"
  i=$((i + 1))
done
echo
echo "Until loop (continues until condition becomes true):"
echo "   j=0"
echo "   until [ \$j -eq 3 ]; do"
echo "     echo \"j = \$j\""
echo "     j=\$((j + 1))"
echo "   done"
echo
echo "Output:"
j=0
until [ $j -eq 3 ]; do
  echo "  j = $j"
  j=$((j + 1))
done
echo

echo "=== Demo Complete ==="