//! Rush Shell Performance Benchmark Suite
//!
//! This benchmark suite performs comprehensive performance regression testing
//! on the Rush shell implementation, covering all major components:
//! - Lexer (tokenization)
//! - Parser (AST construction)
//! - Executor (command execution)
//! - Shell expansions (variables, arithmetic, command substitution)
//! - Control structures (if/for/while/case)
//! - Pipelines and redirections
//! - Script execution modes

use std::fs;
use std::time::{Duration, Instant};

mod benchmark;
mod report;
mod test_cases;

use benchmark::{Benchmark, BenchmarkResult, BenchmarkSuite};
use report::{generate_report, save_results};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Rush Shell Performance Benchmark Suite");
    println!("==========================================");

    // Initialize benchmark suite
    let mut suite = BenchmarkSuite::new();

    // Register benchmark categories
    register_lexer_benchmarks(&mut suite);
    register_parser_benchmarks(&mut suite);
    register_executor_benchmarks(&mut suite);
    register_expansion_benchmarks(&mut suite);
    register_control_structure_benchmarks(&mut suite);
    register_pipeline_benchmarks(&mut suite);
    register_script_benchmarks(&mut suite);

    // Run all benchmarks
    println!("\n📊 Running benchmarks...");
    let results = suite.run_all()?;

    // Generate and save report
    println!("\n📈 Generating report...");
    let report = generate_report(&results);
    
    // Save HTML report
    fs::write("target/benchmark_report.html", &report)?;
    
    // Save JSON results
    save_results(&results, "target/benchmark_results.json")?;

    // Display summary
    println!("\n✅ Benchmark completed!");
    println!("📊 Results summary:");
    println!("   Total benchmarks: {}", results.len());

    let mut total_time = Duration::new(0, 0);
    for result in &results {
        total_time += result.duration;
    }
    println!("   Total time: {:.2}s", total_time.as_secs_f64());

    // Check for regressions (basic implementation)
    check_for_regressions(&results);

    println!("\n📋 Detailed report saved to: target/benchmark_report.html");
    println!("📋 JSON results saved to: target/benchmark_results.json");

    Ok(())
}

fn register_lexer_benchmarks(suite: &mut BenchmarkSuite) {
    println!("📝 Registering lexer benchmarks...");

    // Basic tokenization benchmarks
    suite.add_benchmark(Benchmark::new(
        "lexer_basic_tokens",
        "Basic tokenization (simple commands)",
        Box::new(|iterations| {
            let shell_state = rush_sh::state::ShellState::new();
            let test_cases = vec![
                "ls -la",
                "echo hello world",
                "printf 'test'",
                "cat file.txt",
                "grep pattern file.txt",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    let _ = rush_sh::lexer::lex(case, &shell_state);
                }
            }
            start.elapsed()
        }),
    ));

    // Complex tokenization benchmarks
    suite.add_benchmark(Benchmark::new(
        "lexer_complex_tokens",
        "Complex tokenization (quotes, variables, expansions)",
        Box::new(|iterations| {
            let mut shell_state = rush_sh::state::ShellState::new();
            shell_state.set_var("TEST_VAR", "expanded_value".to_string());

            let test_cases = vec![
                "echo \"hello $TEST_VAR world\"",
                "printf 'single quotes: $TEST_VAR'",
                "echo $(date) and $(whoami)",
                "echo $((2 + 3 * 4))",
                "echo ${TEST_VAR:-default}",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    let _ = rush_sh::lexer::lex(case, &shell_state);
                }
            }
            start.elapsed()
        }),
    ));

    // Large input tokenization
    suite.add_benchmark(Benchmark::new(
        "lexer_large_input",
        "Large input tokenization",
        Box::new(|iterations| {
            let shell_state = rush_sh::state::ShellState::new();
            let large_input = "echo hello world\n".repeat(1000);

            let start = Instant::now();
            for _ in 0..iterations {
                let _ = rush_sh::lexer::lex(&large_input, &shell_state);
            }
            start.elapsed()
        }),
    ));
}

fn register_parser_benchmarks(suite: &mut BenchmarkSuite) {
    println!("🔍 Registering parser benchmarks...");

    // Basic parsing benchmarks
    suite.add_benchmark(Benchmark::new(
        "parser_basic_commands",
        "Basic command parsing",
        Box::new(|iterations| {
            let test_cases = vec![
                "ls -la",
                "echo hello world",
                "printf test",
                "cat file.txt",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        let _ = rush_sh::parser::parse(tokens);
                    }
                }
            }
            start.elapsed()
        }),
    ));

    // Complex parsing benchmarks
    suite.add_benchmark(Benchmark::new(
        "parser_complex_structures",
        "Complex structure parsing (if/for/while/case)",
        Box::new(|iterations| {
            let test_cases = vec![
                "if true; then echo yes; fi",
                "for i in 1 2 3; do echo $i; done",
                // Removed "while true; do echo loop; done" to avoid infinite loop during execution
                "case $var in pattern) echo match;; esac",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        let _ = rush_sh::parser::parse(tokens);
                    }
                }
            }
            start.elapsed()
        }),
    ));

    // Function definition parsing
    suite.add_benchmark(Benchmark::new(
        "parser_function_definitions",
        "Function definition parsing",
        Box::new(|iterations| {
            let test_cases = vec![
                "myfunc() { echo hello; }",
                "complex_func() { if true; then echo yes; else echo no; fi; }",
                "nested_func() { outer() { echo nested; }; }",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        let _ = rush_sh::parser::parse(tokens);
                    }
                }
            }
            start.elapsed()
        }),
    ));
}

fn register_executor_benchmarks(suite: &mut BenchmarkSuite) {
    println!("⚡ Registering executor benchmarks...");

    // Builtin command execution
    suite.add_benchmark(Benchmark::new(
        "executor_builtin_commands",
        "Builtin command execution",
        Box::new(|iterations| {
            let test_cases = vec![
                "true",
                "false",
                "echo hello world",
                "printf test",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut rush_sh::state::ShellState::new());
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));

    // External command execution
    suite.add_benchmark(Benchmark::new(
        "executor_external_commands",
        "External command execution",
        Box::new(|iterations| {
            let test_cases = vec![
                "date",
                "pwd",
                "whoami",
                "id",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut rush_sh::state::ShellState::new());
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));

    // Variable operations
    suite.add_benchmark(Benchmark::new(
        "executor_variable_operations",
        "Variable assignment and expansion",
        Box::new(|iterations| {
            let test_cases = vec![
                "MY_VAR=test_value",
                "echo $MY_VAR",
                "printf ${MY_VAR:-default}",
                "COUNT=$((1 + 2))",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut rush_sh::state::ShellState::new());
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));
}

fn register_expansion_benchmarks(suite: &mut BenchmarkSuite) {
    println!("🔄 Registering expansion benchmarks...");

    // Variable expansion benchmarks
    suite.add_benchmark(Benchmark::new(
        "expansion_variables",
        "Variable expansion performance",
        Box::new(|iterations| {
            let mut shell_state = rush_sh::state::ShellState::new();
            shell_state.set_var("VAR1", "value1".to_string());
            shell_state.set_var("VAR2", "value2".to_string());
            shell_state.set_var("VAR3", "value3".to_string());

            let test_cases = vec![
                "echo $VAR1 $VAR2 $VAR3",
                "printf '${VAR1}${VAR2}${VAR3}'",
                "echo ${VAR1:-default} ${VAR2:-default}",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &shell_state) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut shell_state);
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));

    // Arithmetic expansion benchmarks
    suite.add_benchmark(Benchmark::new(
        "expansion_arithmetic",
        "Arithmetic expansion performance",
        Box::new(|iterations| {
            let test_cases = vec![
                "echo $((1 + 2 * 3))",
                "echo $((100 / 5 + 20))",
                "echo $((2 ** 8))",
                "echo $((100 % 7))",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut rush_sh::state::ShellState::new());
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));

    // Command substitution benchmarks
    suite.add_benchmark(Benchmark::new(
        "expansion_command_substitution",
        "Command substitution performance",
        Box::new(|iterations| {
            let test_cases = vec![
                "echo $(date)",
                "echo $(pwd)",
                "echo $(whoami)",
                "echo $(echo 'nested substitution')",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut rush_sh::state::ShellState::new());
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));
}

fn register_control_structure_benchmarks(suite: &mut BenchmarkSuite) {
    println!("🏗️  Registering control structure benchmarks...");

    // If statement benchmarks
    suite.add_benchmark(Benchmark::new(
        "control_if_statements",
        "If statement execution",
        Box::new(|iterations| {
            let test_cases = vec![
                "if true; then echo yes; fi",
                "if false; then echo no; else echo yes; fi",
                "if echo condition; then echo success; fi",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut rush_sh::state::ShellState::new());
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));

    // Loop benchmarks
    suite.add_benchmark(Benchmark::new(
        "control_loops",
        "Loop execution (for/while)",
        Box::new(|iterations| {
            let test_cases = vec![
                "for i in 1 2 3 4 5; do echo $i; done",
                // Note: Removed while loop to avoid potential infinite loop issues
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut rush_sh::state::ShellState::new());
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));

    // Case statement benchmarks
    suite.add_benchmark(Benchmark::new(
        "control_case_statements",
        "Case statement execution",
        Box::new(|iterations| {
            let test_cases = vec![
                "case test in pattern) echo match;; esac",
                "case $var in pat*) echo pattern;; *) echo default;; esac",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut rush_sh::state::ShellState::new());
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));
}

fn register_pipeline_benchmarks(suite: &mut BenchmarkSuite) {
    println!("🔗 Registering pipeline benchmarks...");

    // Simple pipeline benchmarks
    suite.add_benchmark(Benchmark::new(
        "pipeline_simple",
        "Simple pipeline execution",
        Box::new(|iterations| {
            let test_cases = vec![
                "echo hello | cat",
                "printf test | wc -c",
                "date | cut -d' ' -f1",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut rush_sh::state::ShellState::new());
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));

    // Complex pipeline benchmarks
    suite.add_benchmark(Benchmark::new(
        "pipeline_complex",
        "Complex pipeline execution",
        Box::new(|iterations| {
            let test_cases = vec![
                "cat /dev/null | grep pattern | sort | uniq",
                "find . -name '*.rs' | wc -l",
                "ps aux | grep rush | wc -l",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut rush_sh::state::ShellState::new());
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));

    // Redirection benchmarks
    suite.add_benchmark(Benchmark::new(
        "pipeline_redirections",
        "Pipeline with redirections",
        Box::new(|iterations| {
            let test_cases = vec![
                "echo test > /tmp/test_output.txt",
                "cat < /tmp/test_output.txt > /tmp/test_output2.txt",
                "printf hello >> /tmp/append_test.txt",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for case in &test_cases {
                    if let Ok(tokens) = rush_sh::lexer::lex(case, &rush_sh::state::ShellState::new()) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut rush_sh::state::ShellState::new());
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));
}

fn register_script_benchmarks(suite: &mut BenchmarkSuite) {
    println!("📜 Registering script benchmarks...");

    // Script execution mode benchmarks
    suite.add_benchmark(Benchmark::new(
        "script_execution",
        "Script file execution (multi-line script)",
        Box::new(|iterations| {
            // Multi-line script content to execute
            let script_lines = vec![
                "echo \"Script execution test\"",
                "for i in 1 2 3; do echo \"Iteration $i\"; done",
                "MY_VAR=\"test_value\"",
                "echo \"Variable: $MY_VAR\"",
                "echo \"Arithmetic: $((2 + 3))\"",
                "if true; then echo \"If statement works\"; fi",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                let mut shell_state = rush_sh::state::ShellState::new();
                
                // Execute each line of the script
                for line in &script_lines {
                    if let Ok(tokens) = rush_sh::lexer::lex(line, &shell_state) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut shell_state);
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));

    // Command-line execution mode
    suite.add_benchmark(Benchmark::new(
        "command_line_execution",
        "Command-line execution mode (single commands)",
        Box::new(|iterations| {
            let commands = vec![
                "echo 'Command line test'",
                "printf 'Hello World'",
                "date",
                "pwd",
            ];

            let start = Instant::now();
            for _ in 0..iterations {
                for cmd in &commands {
                    let mut shell_state = rush_sh::state::ShellState::new();
                    if let Ok(tokens) = rush_sh::lexer::lex(cmd, &shell_state) {
                        if let Ok(ast) = rush_sh::parser::parse(tokens) {
                            let _ = rush_sh::executor::execute(ast, &mut shell_state);
                        }
                    }
                }
            }
            start.elapsed()
        }),
    ));
}

fn check_for_regressions(results: &[BenchmarkResult]) {
    println!("\n🔍 Checking for performance regressions...");

    // Basic regression detection - in a real implementation,
    // this would compare against historical baselines
    let mut regressions = 0;

    for result in results {
        // Simple heuristic: flag anything over 100ms as potentially slow
        if result.duration.as_millis() > 100 {
            println!("⚠️  Potentially slow: {} ({:.2}ms)",
                    result.name, result.duration.as_millis());
            regressions += 1;
        }
    }

    if regressions == 0 {
        println!("✅ No performance regressions detected");
    } else {
        println!("⚠️  Found {} potential performance issues", regressions);
    }
}