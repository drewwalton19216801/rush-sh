#!/bin/bash
# Quick test script for benchmarks with reduced iterations

echo "Running quick benchmark test (10 iterations)..."
cd "$(dirname "$0")/.."

# Create target directory if it doesn't exist
mkdir -p target

# Run benchmarks with timeout to catch infinite loops
timeout 30s cargo run -p rush-benchmarks --release 2>&1 | head -100

# Check if files were created
if [ -f "target/benchmark_report.html" ]; then
    echo "✅ HTML report created successfully"
    ls -lh target/benchmark_report.html
else
    echo "❌ HTML report not found"
fi

if [ -f "target/benchmark_results.json" ]; then
    echo "✅ JSON results created successfully"
    ls -lh target/benchmark_results.json
else
    echo "❌ JSON results not found"
fi