#!/bin/bash
# Phase 2: Redirection Order Semantics - POSIX Compliance Verification
# This script tests Rush shell against bash to verify correct redirection order behavior

# Don't exit on error - we want to run all tests
# set -e

RUSH_BIN="${RUSH_BIN:-./target/debug/rush-sh}"

# Check if rush binary exists
if [ ! -f "$RUSH_BIN" ]; then
    echo "Error: Rush binary not found at $RUSH_BIN"
    echo "Please build it first with: cargo build --release"
    exit 1
fi
TEST_DIR="/tmp/rush_phase2_test_$$"
mkdir -p "$TEST_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PASSED=0
FAILED=0

cleanup() {
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

# Test helper function
test_case() {
    local name="$1"
    local command="$2"
    local description="$3"
    
    echo -e "\n${YELLOW}Test: $name${NC}"
    echo "Description: $description"
    echo "Command: $command"
    
    # Use unique filenames for each test
    local test_id="$$_${name}_$(date +%N)"
    bash_out="$TEST_DIR/bash_out_${test_id}"
    bash_err="$TEST_DIR/bash_err_${test_id}"
    rush_out="$TEST_DIR/rush_out_${test_id}"
    rush_err="$TEST_DIR/rush_err_${test_id}"
    
    # Run in bash
    bash -c "$command" >"$bash_out" 2>"$bash_err" || true
    
    # Run in rush
    "$RUSH_BIN" -c "$command" >"$rush_out" 2>"$rush_err" || true
    
    # Compare outputs
    if diff -q "$bash_out" "$rush_out" >/dev/null 2>&1; then
        echo -e "${GREEN}✓ PASS${NC}: Output matches bash"
        ((PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}: Output differs from bash"
        echo "Bash output:"
        cat "$bash_out"
        echo "Rush output:"
        cat "$rush_out"
        ((FAILED++))
    fi
    
    # Cleanup test files
    rm -f "$bash_out" "$bash_err" "$rush_out" "$rush_err"
}

echo "========================================="
echo "Phase 2: Redirection Order Semantics Tests"
echo "========================================="

# Test 1: Multiple output redirections (last wins)
test_case "multiple_output_redirections" \
    "echo hello >$TEST_DIR/file1 >$TEST_DIR/file2; cat $TEST_DIR/file2" \
    "Multiple redirections to same FD - last one wins"

# Test 2: stderr to stdout, then stdout redirect (using sh -c for compound commands)
test_case "stderr_then_stdout" \
    "sh -c 'echo stdout; echo stderr >&2' 2>&1 1>$TEST_DIR/file; cat $TEST_DIR/file" \
    "2>&1 captures old stdout location before 1>file redirect"

# Test 3: stdout redirect, then stderr dup (using sh -c for compound commands)
test_case "stdout_then_stderr" \
    "sh -c 'echo stdout; echo stderr >&2' 1>$TEST_DIR/file 2>&1; cat $TEST_DIR/file" \
    "Both stdout and stderr go to file (2>&1 after 1>file)"

# Test 4: FD duplication chain (using sh -c for compound commands)
test_case "fd_chain" \
    "sh -c 'echo test >&3; echo test2 >&4' 3>$TEST_DIR/file 4>&3; cat $TEST_DIR/file" \
    "FD 4 duplicates FD 3, both go to file"

# Test 5: Append redirection override
echo "initial" > "$TEST_DIR/file1"
echo "initial" > "$TEST_DIR/file2"
test_case "append_override" \
    "echo appended >>$TEST_DIR/file1 >>$TEST_DIR/file2; cat $TEST_DIR/file2" \
    "Append to file2 only (last redirection wins)"

# Test 6: Complex redirection sequence (using sh -c for compound commands)
test_case "complex_sequence" \
    "sh -c 'echo out; echo err >&2' 2>$TEST_DIR/file 1>&2 2>&1; cat $TEST_DIR/file" \
    "Complex chain: 2>file, 1>&2, 2>&1"

# Test 7: Input FD override
echo "content1" > "$TEST_DIR/input1"
echo "content2" > "$TEST_DIR/input2"
test_case "input_override" \
    "cat 3<$TEST_DIR/input1 3<$TEST_DIR/input2 <&3" \
    "Read from file2 (last input redirection wins)"

# Test 8: Close and reopen (using sh -c for compound commands)
test_case "close_reopen" \
    "sh -c 'echo test >&2' 2>&- 2>$TEST_DIR/file; cat $TEST_DIR/file" \
    "Close FD 2, then reopen to file"

echo ""
echo "========================================="
echo "Results:"
echo "========================================="
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo "Total: $((PASSED + FAILED))"

if [ $FAILED -eq 0 ]; then
    echo -e "\n${GREEN}✓ All Phase 2 tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}✗ Some tests failed${NC}"
    exit 1
fi