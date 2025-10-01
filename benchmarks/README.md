# Rush Shell Performance Benchmark Suite

A comprehensive performance benchmarking suite for the Rush shell implementation, designed to detect performance regressions and track performance improvements over time.

## Overview

This benchmark suite tests all major components of the Rush shell:

- **Lexer**: Tokenization performance
- **Parser**: AST construction performance
- **Executor**: Command execution performance
- **Expansions**: Variable, arithmetic, and command substitution performance
- **Control Structures**: if/for/while/case statement performance
- **Pipelines**: Pipeline execution performance
- **Script Execution**: Full script execution performance

## Features

- 🚀 **Comprehensive Coverage**: Tests all major shell components
- 📊 **Detailed Reporting**: HTML reports with performance analysis
- 📈 **Regression Detection**: Identifies performance degradations
- 🔧 **Configurable**: Adjustable iteration counts and test parameters
- 💾 **Historical Tracking**: JSON export for trend analysis
- 🎯 **Performance Classification**: Fast/Medium/Slow categorization

## Quick Start

### Prerequisites

- Rust 1.70+
- Rush shell source code (this repository)

### Running Benchmarks

```bash
# From the repository root
cd benchmarks

# Run all benchmarks with default settings
cargo run

# Run with custom iterations (e.g., 1000 iterations)
cargo run -- --iterations 1000

# Run specific benchmark categories
cargo run -- --categories lexer,parser,executor

# Generate only report (using existing results)
cargo run -- --report-only
```

### Command Line Options

```
Usage: rush-benchmark [OPTIONS]

Options:
    --iterations <NUMBER>    Number of iterations per benchmark [default: 100]
    --categories <LIST>      Comma-separated list of benchmark categories
    --output-dir <DIR>       Output directory for results [default: target]
    --baseline <FILE>        Baseline results file for regression detection
    --report-only            Generate report from existing results
    --verbose, -v            Enable verbose output
    --help                   Show this help message
```

## Benchmark Categories

### Lexer Benchmarks

- **Basic tokenization**: Simple command parsing
- **Complex tokenization**: Quotes, variables, expansions
- **Large input**: Performance with large inputs

### Parser Benchmarks

- **Basic commands**: Simple command parsing
- **Complex structures**: Control flow parsing
- **Function definitions**: Function parsing performance

### Executor Benchmarks

- **Builtin commands**: Internal command execution
- **External commands**: System command execution
- **Variable operations**: Assignment and expansion

### Expansion Benchmarks

- **Variable expansion**: `$VAR` and `${VAR}` performance
- **Arithmetic expansion**: `$((...))` performance
- **Command substitution**: `$(...)` and `` `...` `` performance

### Control Structure Benchmarks

- **If statements**: Conditional execution performance
- **Loops**: for/while loop performance
- **Case statements**: Pattern matching performance

### Pipeline Benchmarks

- **Simple pipelines**: Basic pipe performance
- **Complex pipelines**: Multi-stage pipeline performance
- **Redirections**: I/O redirection performance

### Script Benchmarks

- **Script execution**: Full script file execution
- **Command-line execution**: `-c` flag performance

## Output and Reports

### HTML Report

The benchmark suite generates a comprehensive HTML report with:

- Performance summary statistics
- Detailed results table with timing information
- Performance classification (Fast/Medium/Slow)
- Recommendations for optimization
- Visual indicators for performance levels

### JSON Results

Raw results are saved in JSON format for:

- Historical trend analysis
- Automated regression detection
- Integration with CI/CD pipelines
- Custom analysis tools

### Example Report Structure

```json
[
  {
    "name": "lexer_basic_tokens",
    "description": "Basic tokenization (simple commands)",
    "iterations": 100,
    "duration": {"secs": 0, "nanos": 1500000},
    "total_time": {"secs": 0, "nanos": 2000000}
  }
]
```

## Performance Analysis

### Regression Detection

The benchmark suite can compare current results with historical baselines:

```bash
# Compare with previous baseline
cargo run -- --baseline target/baseline_results.json

# Save current results as baseline for future comparisons
cp target/benchmark_results.json target/baseline_results.json
```

### Performance Thresholds

- **Fast**: < 10ms average per iteration
- **Medium**: 10-100ms average per iteration
- **Slow**: ≥ 100ms average per iteration

## Integration with CI/CD

### GitHub Actions Example

```yaml
- name: Run Performance Benchmarks
  run: |
    cd benchmarks
    cargo run -- --iterations 50 --output-dir benchmark-results

- name: Upload Benchmark Results
  uses: actions/upload-artifact@v3
  with:
    name: benchmark-results
    path: benchmarks/target/benchmark_results.json
```

### Automated Regression Detection

```bash
#!/bin/bash
# Run benchmarks and check for regressions
cd benchmarks
cargo run -- --baseline baseline_results.json --iterations 100

# Exit with error if regressions detected
if grep -q "regression" benchmark_report.html; then
    echo "Performance regression detected!"
    exit 1
fi
```

## Development and Extension

### Adding New Benchmarks

1. **Define the benchmark function**:

```rust
fn my_benchmark(iterations: usize) -> Duration {
    let start = Instant::now();
    for _ in 0..iterations {
        // Your benchmark code here
    }
    start.elapsed()
}
```

2. **Register the benchmark**:

```rust
suite.add_benchmark(Benchmark::new(
    "my_benchmark_name",
    "Description of what this benchmark tests",
    Box::new(my_benchmark),
));
```

3. **Add test cases** (optional):

```rust
// In test_cases.rs
pub const MY_TEST_CASES: &[&str] = &[
    "test command 1",
    "test command 2",
];
```

### Custom Analysis

The JSON results can be analyzed with custom tools:

```python
import json
import matplotlib.pyplot as plt

# Load benchmark results
with open('target/benchmark_results.json', 'r') as f:
    results = json.load(f)

# Extract timing data
names = [r['name'] for r in results]
times = [r['duration']['nanos'] / 1e6 for r in results]  # Convert to ms

# Create visualization
plt.bar(names, times)
plt.xticks(rotation=45)
plt.ylabel('Time (ms)')
plt.title('Rush Shell Benchmark Results')
plt.tight_layout()
plt.savefig('benchmark_chart.png')
```

## Troubleshooting

### Common Issues

1. **Missing external commands**:

   ```bash
   # Install required system tools
   sudo apt-get install coreutils  # For date, cat, etc.
   ```

2. **Permission errors**:

   ```bash
   # Fix permissions for temp files
   mkdir -p /tmp/rush_benchmarks
   chmod 755 /tmp/rush_benchmarks
   ```

3. **Build errors**:

   ```bash
   # Clean and rebuild
   cd benchmarks
   cargo clean
   cargo build
   ```

### Performance Investigation

If benchmarks show unexpected slowness:

1. **Profile the code**:

   ```bash
   cargo flamegraph --bin rush-benchmark
   ```

2. **Check system resources**:

   ```bash
   # Monitor system during benchmarks
   htop
   ```

3. **Isolate components**:

   ```bash
   # Test individual components
   cargo run -- --categories lexer
   ```

## Contributing

When adding new benchmarks or modifying existing ones:

1. Ensure benchmarks are **repeatable** and **isolated**
2. Add appropriate **test cases** for new functionality
3. Update **documentation** for new benchmark categories
4. Test on **multiple systems** for consistent results
5. Consider **warm-up** periods for accurate measurements

## License

This benchmark suite is part of the Rush shell project and follows the same MIT license.
