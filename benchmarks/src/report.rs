//! Report generation for benchmark results

use super::benchmark::BenchmarkResult;
use std::collections::HashMap;
use std::fs;

/// Generate a comprehensive HTML report from benchmark results
pub fn generate_report(results: &[BenchmarkResult]) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html lang=\"en\">\n");
    html.push_str("<head>\n");
    html.push_str("    <meta charset=\"UTF-8\">\n");
    html.push_str("    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str("    <title>Rush Shell Benchmark Report</title>\n");
    html.push_str("    <style>\n");
    html.push_str("        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #0f172a; }\n");
    html.push_str("        .container { max-width: 1200px; margin: 0 auto; background: #1e293b; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.3); }\n");
    html.push_str("        h1 { color: #f8fafc; text-align: center; margin-bottom: 30px; }\n");
    html.push_str("        h2 { color: #cbd5e1; border-bottom: 2px solid #334155; padding-bottom: 10px; }\n");
    html.push_str("        .summary { background: #334155; padding: 20px; border-radius: 5px; margin-bottom: 30px; }\n");
    html.push_str("        .summary-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; }\n");
    html.push_str("        .summary-item { text-align: center; }\n");
    html.push_str("        .summary-value { font-size: 2em; font-weight: bold; color: #34d399; }\n");
    html.push_str("        .summary-label { color: #94a3b8; font-size: 0.9em; }\n");
    html.push_str("        table { width: 100%; border-collapse: collapse; margin-top: 20px; }\n");
    html.push_str("        th, td { padding: 12px; text-align: left; border-bottom: 1px solid #475569; color: #f8fafc; }\n");
    html.push_str("        th { background-color: #334155; font-weight: 600; color: #f8fafc; }\n");
    html.push_str("        tr:hover { background-color: #475569; }\n");
    html.push_str("        .duration { font-family: 'Courier New', monospace; }\n");
    html.push_str("        .performance-indicator { display: inline-block; padding: 4px 8px; border-radius: 4px; font-size: 0.8em; font-weight: bold; }\n");
    html.push_str("        .fast { background-color: #064e3b; color: #34d399; }\n");
    html.push_str("        .medium { background-color: #451a03; color: #fbbf24; }\n");
    html.push_str("        .slow { background-color: #7f1d1d; color: #f87171; }\n");
    html.push_str("        .timestamp { color: #94a3b8; font-size: 0.9em; margin-bottom: 20px; }\n");
    html.push_str("    </style>\n");
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    html.push_str("    <div class=\"container\">\n");
    html.push_str("        <h1>🚀 Rush Shell Performance Benchmark Report</h1>\n");

    // Timestamp
    html.push_str(&format!(
        "        <div class=\"timestamp\">Generated on: {}</div>\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    // Summary statistics
    let total_benchmarks = results.len();
    let total_time: std::time::Duration = results.iter().map(|r| r.total_time).sum();
    let avg_time_per_benchmark = total_time / total_benchmarks as u32;

    html.push_str("        <div class=\"summary\">\n");
    html.push_str("            <h2>Summary</h2>\n");
    html.push_str("            <div class=\"summary-grid\">\n");
    html.push_str(&format!(
        "                <div class=\"summary-item\">\n                    <div class=\"summary-value\">{}</div>\n                    <div class=\"summary-label\">Total Benchmarks</div>\n                </div>\n",
        total_benchmarks
    ));
    html.push_str(&format!(
        "                <div class=\"summary-item\">\n                    <div class=\"summary-value\">{:.2}s</div>\n                    <div class=\"summary-label\">Total Time</div>\n                </div>\n",
        total_time.as_secs_f64()
    ));
    html.push_str(&format!(
        "                <div class=\"summary-item\">\n                    <div class=\"summary-value\">{:.2}ms</div>\n                    <div class=\"summary-label\">Avg per Benchmark</div>\n                </div>\n",
        avg_time_per_benchmark.as_millis()
    ));
    html.push_str("            </div>\n");
    html.push_str("        </div>\n");

    // Detailed results table
    html.push_str("        <h2>Detailed Results</h2>\n");
    html.push_str("        <table>\n");
    html.push_str("            <thead>\n");
    html.push_str("                <tr>\n");
    html.push_str("                    <th>Benchmark</th>\n");
    html.push_str("                    <th>Description</th>\n");
    html.push_str("                    <th>Iterations</th>\n");
    html.push_str("                    <th>Total Time</th>\n");
    html.push_str("                    <th>Avg Time/Iteration</th>\n");
    html.push_str("                    <th>Iterations/sec</th>\n");
    html.push_str("                    <th>Performance</th>\n");
    html.push_str("                </tr>\n");
    html.push_str("            </thead>\n");
    html.push_str("            <tbody>\n");

    for result in results {
        let avg_per_iter = result.avg_time_per_iteration();
        let iter_per_sec = result.iterations_per_second();
        let performance_class = get_performance_class(&result.duration);

        html.push_str("                <tr>\n");
        html.push_str(&format!(
            "                    <td><strong>{}</strong></td>\n",
            result.name
        ));
        html.push_str(&format!(
            "                    <td>{}</td>\n",
            result.description
        ));
        html.push_str(&format!(
            "                    <td>{}</td>\n",
            result.iterations
        ));
        html.push_str(&format!(
            "                    <td class=\"duration\">{:.3}ms</td>\n",
            result.duration.as_millis()
        ));
        html.push_str(&format!(
            "                    <td class=\"duration\">{:.3}ms</td>\n",
            avg_per_iter.as_millis()
        ));
        html.push_str(&format!(
            "                    <td class=\"duration\">{:.0}</td>\n",
            iter_per_sec
        ));
        html.push_str(&format!(
            "                    <td><span class=\"performance-indicator {}\">{}</span></td>\n",
            performance_class,
            get_performance_label(&result.duration)
        ));
        html.push_str("                </tr>\n");
    }

    html.push_str("            </tbody>\n");
    html.push_str("        </table>\n");

    // Performance analysis
    html.push_str("        <h2>Performance Analysis</h2>\n");
    html.push_str("        <div class=\"summary\">\n");

    let fast_count = results.iter().filter(|r| r.duration.as_millis() < 10).count();
    let medium_count = results.iter().filter(|r| r.duration.as_millis() >= 10 && r.duration.as_millis() < 100).count();
    let slow_count = results.iter().filter(|r| r.duration.as_millis() >= 100).count();

    html.push_str(&format!(
        "            <p style=\"color: #f8fafc;\"><strong>Performance Distribution:</strong></p>\n            <ul style=\"color: #f8fafc;\">\n                <li>Fast (<10ms): {} benchmarks</li>\n                <li>Medium (10-100ms): {} benchmarks</li>\n                <li>Slow (≥100ms): {} benchmarks</li>\n            </ul>\n",
        fast_count, medium_count, slow_count
    ));

    // Recommendations
    html.push_str("            <h3 style=\"color: #f8fafc;\">Recommendations</h3>\n");
    html.push_str("            <ul style=\"color: #f8fafc;\">\n");

    if slow_count > 0 {
        html.push_str("                <li>⚠️ Some benchmarks are running slowly. Consider optimizing the slowest components.</li>\n");
    }

    if fast_count > total_benchmarks * 8 / 10 {
        html.push_str("                <li>✅ Overall performance looks good! Most benchmarks are running efficiently.</li>\n");
    }

    html.push_str("                <li>💡 Run benchmarks regularly to track performance trends over time.</li>\n");
    html.push_str("                <li>🔍 Focus optimization efforts on the slowest benchmarks identified above.</li>\n");
    html.push_str("            </ul>\n");

    html.push_str("        </div>\n");

    html.push_str("    </div>\n");
    html.push_str("</body>\n");
    html.push_str("</html>\n");

    html
}

fn get_performance_class(duration: &std::time::Duration) -> &'static str {
    match duration.as_millis() {
        0..=10 => "fast",
        11..=100 => "medium",
        _ => "slow",
    }
}

fn get_performance_label(duration: &std::time::Duration) -> &'static str {
    match duration.as_millis() {
        0..=10 => "Fast",
        11..=100 => "Medium",
        _ => "Slow",
    }
}

/// Save benchmark results to JSON file
pub fn save_results(
    results: &[BenchmarkResult],
    filename: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let json_data = serde_json::to_string_pretty(results)?;
    fs::write(filename, json_data)?;
    Ok(())
}

/// Load benchmark results from JSON file
///
/// This function is reserved for future use when implementing historical
/// benchmark comparison and regression tracking features.
#[allow(dead_code)]
pub fn load_results(filename: &str) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(filename)?;
    let results: Vec<BenchmarkResult> = serde_json::from_str(&data)?;
    Ok(results)
}

/// Compare current results with baseline and identify regressions
///
/// This function will be used to implement automated regression detection
/// by comparing current benchmark results against historical baselines.
#[allow(dead_code)]
pub fn compare_with_baseline(
    current: &[BenchmarkResult],
    baseline: &[BenchmarkResult]
) -> Vec<RegressionInfo> {
    let mut regressions = Vec::new();

    // Create lookup map for baseline results
    let baseline_map: HashMap<String, &BenchmarkResult> = baseline
        .iter()
        .map(|r| (r.name.clone(), r))
        .collect();

    for current_result in current {
        if let Some(baseline_result) = baseline_map.get(&current_result.name) {
            // Check if current is significantly slower (more than 20% slower)
            let baseline_avg = baseline_result.avg_time_per_iteration();
            let current_avg = current_result.avg_time_per_iteration();

            let ratio = current_avg.as_nanos() as f64 / baseline_avg.as_nanos() as f64;

            if ratio > 1.2 {
                regressions.push(RegressionInfo {
                    benchmark_name: current_result.name.clone(),
                    baseline_time: baseline_avg,
                    current_time: current_avg,
                    slowdown_ratio: ratio,
                });
            }
        }
    }

    regressions
}

/// Information about a performance regression
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RegressionInfo {
    pub benchmark_name: String,
    pub baseline_time: std::time::Duration,
    pub current_time: std::time::Duration,
    pub slowdown_ratio: f64,
}