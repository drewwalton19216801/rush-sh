//! Rush Shell Benchmark Library
//!
//! This library provides comprehensive performance benchmarking capabilities
//! for the Rush shell implementation.

pub mod benchmark;
pub mod report;
pub mod test_cases;

// Re-export main types for convenience
pub use benchmark::{Benchmark, BenchmarkResult, BenchmarkSuite};
pub use report::{generate_report, save_results, load_results, compare_with_baseline, RegressionInfo};