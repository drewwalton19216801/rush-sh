//! Core benchmarking functionality for Rush shell performance testing

use std::time::{Duration, Instant};

/// Represents a single benchmark test
pub struct Benchmark {
    pub name: String,
    pub description: String,
    pub test_fn: Box<dyn Fn(usize) -> Duration>,
}

impl Benchmark {
    pub fn new<F>(name: &str, description: &str, test_fn: F) -> Self
    where
        F: Fn(usize) -> Duration + 'static,
    {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            test_fn: Box::new(test_fn),
        }
    }

    pub fn run(&self, iterations: usize) -> BenchmarkResult {
        println!("  Running: {} ({})", self.name, self.description);

        // Warm up
        let _ = (self.test_fn)(1);

        // Actual measurement
        let start = Instant::now();
        let duration = (self.test_fn)(iterations);
        let end = Instant::now();

        let total_duration = end.duration_since(start);

        BenchmarkResult {
            name: self.name.clone(),
            description: self.description.clone(),
            iterations,
            duration,
            total_time: total_duration,
        }
    }
}

/// Result of a single benchmark run
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub description: String,
    pub iterations: usize,
    pub duration: Duration,
    pub total_time: Duration,
}

impl BenchmarkResult {
    pub fn avg_time_per_iteration(&self) -> Duration {
        Duration::from_nanos((self.duration.as_nanos() / self.iterations as u128) as u64)
    }

    pub fn iterations_per_second(&self) -> f64 {
        self.iterations as f64 / self.duration.as_secs_f64()
    }
}

/// Collection of benchmarks organized by category
pub struct BenchmarkSuite {
    pub benchmarks: Vec<Benchmark>,
}

impl BenchmarkSuite {
    pub fn new() -> Self {
        Self {
            benchmarks: Vec::new(),
        }
    }

    pub fn add_benchmark(&mut self, benchmark: Benchmark) {
        self.benchmarks.push(benchmark);
    }

    pub fn run_all(&self) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for benchmark in &self.benchmarks {
            let result = benchmark.run(100); // Default 100 iterations
            results.push(result);
        }

        Ok(results)
    }

    /// Run all benchmarks with a custom number of iterations
    ///
    /// This method is currently unused but provides flexibility for future
    /// benchmark scenarios where different iteration counts are needed.
    #[allow(dead_code)]
    pub fn run_with_iterations(
        &self,
        iterations: usize,
    ) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for benchmark in &self.benchmarks {
            let result = benchmark.run(iterations);
            results.push(result);
        }

        Ok(results)
    }
}

impl Default for BenchmarkSuite {
    fn default() -> Self {
        Self::new()
    }
}
