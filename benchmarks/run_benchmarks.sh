#!/bin/bash
# Rush Shell Benchmark Runner
# Simple script to run benchmarks with common configurations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Rush Shell Performance Benchmark Suite${NC}"
echo "=========================================="

# Default values
ITERATIONS=100
CATEGORIES=""
BASELINE=""
VERBOSE=false
REPORT_ONLY=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --iterations)
            ITERATIONS="$2"
            shift 2
            ;;
        --categories)
            CATEGORIES="$2"
            shift 2
            ;;
        --baseline)
            BASELINE="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --report-only)
            REPORT_ONLY=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --iterations NUM    Number of iterations per benchmark (default: 100)"
            echo "  --categories LIST   Comma-separated list of benchmark categories"
            echo "  --baseline FILE     Baseline results file for regression detection"
            echo "  --verbose          Enable verbose output"
            echo "  --report-only       Generate report from existing results"
            echo "  --help             Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                                    # Run all benchmarks"
            echo "  $0 --iterations 1000                  # Run with 1000 iterations"
            echo "  $0 --categories lexer,parser          # Run specific categories"
            echo "  $0 --baseline results.json            # Compare with baseline"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Build arguments for cargo
CARGO_ARGS=()
if [[ "$ITERATIONS" != "100" ]]; then
    CARGO_ARGS+=(--iterations "$ITERATIONS")
fi

if [[ -n "$CATEGORIES" ]]; then
    CARGO_ARGS+=(--categories "$CATEGORIES")
fi

if [[ -n "$BASELINE" ]]; then
    CARGO_ARGS+=(--baseline "$BASELINE")
fi

if [[ "$REPORT_ONLY" == "true" ]]; then
    CARGO_ARGS+=(--report-only)
fi

if [[ "$VERBOSE" == "true" ]]; then
    CARGO_ARGS+=(-v)
fi

# Check if we need to build first
if [[ ! -f "target/debug/rush-benchmark" ]] || [[ "benchmarks/src/main.rs" -nt "target/debug/rush-benchmark" ]]; then
    echo -e "${YELLOW}🔨 Building benchmark suite...${NC}"
    cargo build --bin rush-benchmark
fi

# Check dependencies
echo -e "${YELLOW}🔍 Checking dependencies...${NC}"
if ! command -v date >/dev/null 2>&1; then
    echo -e "${RED}❌ Error: 'date' command not found${NC}"
    exit 1
fi

if ! command -v cat >/dev/null 2>&1; then
    echo -e "${RED}❌ Error: 'cat' command not found${NC}"
    exit 1
fi

# Create output directory
mkdir -p target

# Run benchmarks
echo -e "${GREEN}📊 Running benchmarks...${NC}"
echo "Configuration:"
echo "  Iterations: $ITERATIONS"
echo "  Categories: ${CATEGORIES:-all}"
echo "  Baseline: ${BASELINE:-none}"
echo ""

if [[ "$VERBOSE" == "true" ]]; then
    cargo run --bin rush-benchmark "${CARGO_ARGS[@]}"
else
    cargo run --bin rush-benchmark "${CARGO_ARGS[@]}" --quiet
fi

# Check if results were generated
if [[ -f "target/benchmark_results.json" ]]; then
    echo -e "${GREEN}✅ Benchmarks completed successfully!${NC}"
    echo ""
    echo "📋 Results:"
    echo "  - HTML Report: target/benchmark_report.html"
    echo "  - JSON Data: target/benchmark_results.json"
    echo ""
    echo -e "${BLUE}💡 Tip: Open target/benchmark_report.html in your browser for detailed analysis${NC}"
else
    echo -e "${RED}❌ Error: Benchmark results not found${NC}"
    exit 1
fi