#!/usr/bin/env rush
# Job Control Demonstration Script
# This script demonstrates all job control features in Rush shell

echo "=== Job Control Demo ==="
echo ""

# ============================================================================
# 1. Basic Background Job Execution
# ============================================================================
echo "1. Starting a background job with &"
echo "   Command: sleep 2 &"
sleep 2 &
echo "   Job started in background"
echo ""

# ============================================================================
# 2. Listing Jobs
# ============================================================================
echo "2. Listing all jobs with 'jobs'"
jobs
echo ""

echo "   Listing jobs with PIDs using 'jobs -l'"
jobs -l
echo ""

echo "   Listing only PIDs using 'jobs -p'"
jobs -p
echo ""

# ============================================================================
# 3. Multiple Background Jobs
# ============================================================================
echo "3. Starting multiple background jobs"
echo "   Command: sleep 3 &"
sleep 3 &
echo "   Command: sleep 4 &"
sleep 4 &
echo "   Command: sleep 5 &"
sleep 5 &
echo ""

echo "   All active jobs:"
jobs
echo ""

# ============================================================================
# 4. Job Status Tracking
# ============================================================================
echo "4. Checking specific jobs"
echo "   Current job (%):"
jobs %
echo ""

echo "   Previous job (%-):"
jobs %-
echo ""

echo "   Specific job by number (%1):"
jobs %1
echo ""

# ============================================================================
# 5. Waiting for Jobs
# ============================================================================
echo "5. Waiting for jobs to complete"
echo "   Waiting for job 1: wait %1"
wait %1
echo "   Job 1 completed with exit code: $?"
echo ""

echo "   Waiting for current job: wait %"
wait %
echo "   Current job completed with exit code: $?"
echo ""

echo "   Remaining jobs:"
jobs
echo ""

# ============================================================================
# 6. Wait for All Jobs
# ============================================================================
echo "6. Waiting for all remaining jobs"
echo "   Command: wait"
wait
echo "   All jobs completed"
echo ""

echo "   Jobs table (should be empty or only show completed jobs):"
jobs
echo ""

# ============================================================================
# 7. Background Pipelines
# ============================================================================
echo "7. Running a pipeline in the background"
echo "   Command: echo 'test data' | grep 'test' | wc -l &"
echo 'test data' | grep 'test' | wc -l &
echo "   Pipeline started in background"
echo ""

echo "   Jobs with pipeline:"
jobs -l
echo ""

# Wait for pipeline to complete
wait
echo ""

# ============================================================================
# 8. Job Control with Builtins
# ============================================================================
echo "8. Background builtin commands"
echo "   Command: echo 'Background echo' &"
echo 'Background echo' &
echo ""

echo "   Jobs:"
jobs
echo ""

wait
echo ""

# ============================================================================
# 9. Killing Jobs
# ============================================================================
echo "9. Killing jobs with 'kill'"
echo "   Starting a long-running job"
sleep 100 &
LONG_JOB_PID=$!
echo "   Job PID: $LONG_JOB_PID"
echo ""

echo "   Jobs before kill:"
jobs
echo ""

echo "   Killing job with SIGTERM: kill %"
kill %
sleep 0.1
echo ""

echo "   Jobs after kill (job should be terminated):"
jobs
echo ""

# ============================================================================
# 10. Signal Handling
# ============================================================================
echo "10. Sending different signals"
echo "    Starting another long job"
sleep 100 &
echo ""

echo "    Sending SIGTERM with -s option: kill -s TERM %"
kill -s TERM %
sleep 0.1
echo ""

echo "    Starting another job for -n option demo"
sleep 100 &
echo "    Sending SIGKILL with -n option: kill -n 9 %"
kill -n 9 %
sleep 0.1
echo ""

# ============================================================================
# 11. Jobspec Variations
# ============================================================================
echo "11. Different jobspec formats"
echo "    Starting jobs with identifiable commands"
sleep 10 &
echo "    Job 1: sleep 10 &"
echo ""

echo "    Listing job by command prefix: jobs %sleep"
jobs %sleep
echo ""

# Kill it
kill %sleep
sleep 0.1
echo ""

# ============================================================================
# 12. Multiple Concurrent Jobs
# ============================================================================
echo "12. Managing multiple concurrent jobs"
echo "    Starting 5 background jobs"
for i in 1 2 3 4 5; do
    sleep $i &
    echo "    Started job $i (sleep $i)"
done
echo ""

echo "    All jobs:"
jobs
echo ""

echo "    Waiting for specific jobs: wait %1 %2"
wait %1 %2
echo "    Jobs 1 and 2 completed"
echo ""

echo "    Remaining jobs:"
jobs
echo ""

echo "    Waiting for all remaining jobs"
wait
echo "    All jobs completed"
echo ""

# ============================================================================
# 13. Job Exit Codes
# ============================================================================
echo "13. Checking job exit codes"
echo "    Starting a job that succeeds"
true &
wait %
echo "    Exit code: $?"
echo ""

echo "    Starting a job that fails"
false &
wait %
echo "    Exit code: $?"
echo ""

# ============================================================================
# 14. Real-World Use Case: Parallel Processing
# ============================================================================
echo "14. Real-world example: Parallel file processing"
echo "    Creating test files"
echo "data1" > /tmp/rush_test_1.txt
echo "data2" > /tmp/rush_test_2.txt
echo "data3" > /tmp/rush_test_3.txt
echo ""

echo "    Processing files in parallel"
cat /tmp/rush_test_1.txt | wc -l > /tmp/rush_result_1.txt &
cat /tmp/rush_test_2.txt | wc -l > /tmp/rush_result_2.txt &
cat /tmp/rush_test_3.txt | wc -l > /tmp/rush_result_3.txt &
echo ""

echo "    Jobs running:"
jobs
echo ""

echo "    Waiting for all processing to complete"
wait
echo "    Processing complete"
echo ""

echo "    Results:"
cat /tmp/rush_result_1.txt
cat /tmp/rush_result_2.txt
cat /tmp/rush_result_3.txt
echo ""

# Cleanup
rm -f /tmp/rush_test_*.txt /tmp/rush_result_*.txt
echo "    Cleaned up test files"
echo ""

# ============================================================================
# 15. Job Control Best Practices
# ============================================================================
echo "15. Best practices demonstration"
echo ""

echo "    a) Always check if jobs exist before operating on them"
if jobs % > /dev/null 2>&1; then
    echo "       Current job exists"
else
    echo "       No current job (expected)"
fi
echo ""

echo "    b) Use wait to ensure jobs complete before proceeding"
sleep 1 &
JOB_PID=$!
echo "       Started job with PID: $JOB_PID"
wait $JOB_PID
echo "       Job completed, safe to proceed"
echo ""

echo "    c) Handle job failures gracefully"
false &
wait %
EXIT_CODE=$?
if [ $EXIT_CODE -ne 0 ]; then
    echo "       Job failed with exit code: $EXIT_CODE"
    echo "       Handling failure gracefully"
fi
echo ""

# ============================================================================
# Summary
# ============================================================================
echo "=== Job Control Demo Complete ==="
echo ""
echo "Key features demonstrated:"
echo "  - Background job execution with &"
echo "  - Job listing with jobs, jobs -l, jobs -p"
echo "  - Job status tracking (%, %-, %n)"
echo "  - Waiting for jobs with wait"
echo "  - Killing jobs with kill"
echo "  - Signal handling (-s, -n, -SIGNAME)"
echo "  - Jobspec variations (%n, %, %-, %string, %?string)"
echo "  - Multiple concurrent jobs"
echo "  - Pipeline background execution"
echo "  - Exit code handling"
echo "  - Real-world parallel processing"
echo "  - Best practices"
echo ""
echo "For more information, see the Rush documentation."
