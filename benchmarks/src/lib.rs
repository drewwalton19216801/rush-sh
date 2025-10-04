//! Rush Shell Benchmark Library
//!
//! This library provides comprehensive performance benchmarking capabilities
//! for the Rush shell implementation.

pub mod benchmark;
pub mod report;
pub mod test_cases;

// Re-export main types for convenience
pub use benchmark::{Benchmark, BenchmarkResult, BenchmarkSuite};
pub use report::{
    RegressionInfo, compare_with_baseline, generate_report, load_results, save_results,
};
