//! Test case definitions and utilities for benchmarking
//!
//! This module contains a comprehensive library of test cases and utilities for
//! benchmarking the Rush shell implementation. Many items are marked with
//! `#[allow(dead_code)]` because they are part of a reusable test case library
//! designed for future benchmark expansion.
//!
//! ## Dead Code Rationale
//!
//! The following items are intentionally unused in the current benchmark suite
//! but are retained for future use:
//!
//! - **Test case constants**: Pre-defined test cases organized by category
//!   (lexer, parser, executor, expansion, control structures, pipelines) that
//!   can be easily referenced when adding new benchmarks.
//!
//! - **Utility functions**: Helper functions for test setup, cleanup, and
//!   system information gathering that will be useful for more advanced
//!   benchmark scenarios.
//!
//! - **Generator functions**: Functions like `generate_large_input()` and
//!   `generate_complexity_variants()` that can create dynamic test cases
//!   for stress testing and complexity analysis.
//!
//! These items form a reusable library that makes it easy to add new benchmarks
//! without duplicating test case definitions throughout the codebase.

/// Predefined test cases for different benchmark categories
#[allow(dead_code)]
pub mod lexer_tests {
    pub const BASIC_TOKENS: &[&str] = &[
        "ls -la",
        "echo hello world",
        "printf 'test'",
        "cat file.txt",
        "grep pattern file.txt",
    ];

    pub const COMPLEX_TOKENS: &[&str] = &[
        "echo \"hello world\"",
        "printf 'single quotes'",
        "echo $(date) and $(whoami)",
        "echo $((2 + 3 * 4))",
        "echo ${VAR:-default}",
    ];

    pub const LARGE_INPUT: &str = "echo hello world\n";
}

#[allow(dead_code)]
pub mod parser_tests {
    pub const BASIC_COMMANDS: &[&str] = &[
        "ls -la",
        "echo hello world",
        "printf test",
        "cat file.txt",
    ];

    pub const COMPLEX_STRUCTURES: &[&str] = &[
        "if true; then echo yes; fi",
        "for i in 1 2 3; do echo $i; done",
        // Removed "while true; do echo loop; done" to avoid infinite loop during execution
        "case $var in pattern) echo match;; esac",
    ];

    pub const FUNCTION_DEFINITIONS: &[&str] = &[
        "myfunc() { echo hello; }",
        "complex_func() { if true; then echo yes; else echo no; fi; }",
        "nested_func() { outer() { echo nested; }; }",
    ];
}

#[allow(dead_code)]
pub mod executor_tests {
    pub const BUILTIN_COMMANDS: &[&str] = &[
        "true",
        "false",
        "echo hello world",
        "printf test",
    ];

    pub const EXTERNAL_COMMANDS: &[&str] = &[
        "date",
        "pwd",
        "whoami",
        "id",
    ];

    pub const VARIABLE_OPERATIONS: &[&str] = &[
        "MY_VAR=test_value",
        "echo $MY_VAR",
        "printf ${MY_VAR:-default}",
        "COUNT=$((1 + 2))",
    ];
}

#[allow(dead_code)]
pub mod expansion_tests {
    pub const VARIABLE_EXPANSION: &[&str] = &[
        "echo $VAR1 $VAR2 $VAR3",
        "printf '${VAR1}${VAR2}${VAR3}'",
        "echo ${VAR1:-default} ${VAR2:-default}",
    ];

    pub const ARITHMETIC_EXPANSION: &[&str] = &[
        "echo $((1 + 2 * 3))",
        "echo $((100 / 5 + 20))",
        "echo $((2 ** 8))",
        "echo $((100 % 7))",
    ];

    pub const COMMAND_SUBSTITUTION: &[&str] = &[
        "echo $(date)",
        "echo $(pwd)",
        "echo $(whoami)",
        "echo $(echo 'nested substitution')",
    ];
}

#[allow(dead_code)]
pub mod control_tests {
    pub const IF_STATEMENTS: &[&str] = &[
        "if true; then echo yes; fi",
        "if false; then echo no; else echo yes; fi",
        "if echo condition; then echo success; fi",
    ];

    pub const LOOPS: &[&str] = &[
        "for i in 1 2 3 4 5; do echo $i; done",
        // Removed "while echo condition; do echo body; done" - could cause infinite loop
        // since "echo condition" always succeeds (exit code 0)
    ];

    pub const CASE_STATEMENTS: &[&str] = &[
        "case test in pattern) echo match;; esac",
        "case $var in pat*) echo pattern;; *) echo default;; esac",
    ];
}

#[allow(dead_code)]
pub mod pipeline_tests {
    pub const SIMPLE_PIPELINES: &[&str] = &[
        "echo hello | cat",
        "printf test | wc -c",
        "date | cut -d' ' -f1",
    ];

    pub const COMPLEX_PIPELINES: &[&str] = &[
        "cat /dev/null | grep pattern | sort | uniq",
        "find . -name '*.rs' | wc -l",
        "ps aux | grep rush | wc -l",
    ];

    pub const REDIRECTIONS: &[&str] = &[
        "echo test > /tmp/test_output.txt",
        "cat < /tmp/test_output.txt > /tmp/test_output2.txt",
        "printf hello >> /tmp/append_test.txt",
    ];
}

/// Script content for script execution benchmarks
#[allow(dead_code)]
pub const BENCHMARK_SCRIPT: &str = r#"#!/usr/bin/env rush-sh
echo "Script execution test"
for i in 1 2 3; do
    echo "Iteration $i"
done
MY_VAR="test_value"
echo "Variable: $MY_VAR"
echo "Arithmetic: $((2 + 3))"
echo "Command substitution: $(date)"
if true; then
    echo "If statement works"
fi
"#;

/// Command-line execution test cases
#[allow(dead_code)]
pub const COMMAND_LINE_TESTS: &[&str] = &[
    "echo 'Command line test'",
    "printf 'Hello World'",
    "date",
    "pwd",
];

/// Generate a large test input for stress testing
#[allow(dead_code)]
pub fn generate_large_input(size: usize) -> String {
    let mut input = String::with_capacity(size);
    for i in 0..size {
        input.push_str(&format!("echo \"line_{}\"\n", i));
    }
    input
}

/// Generate test cases with varying complexity levels
#[allow(dead_code)]
pub fn generate_complexity_variants(base_command: &str, levels: usize) -> Vec<String> {
    let mut variants = Vec::new();

    for level in 0..levels {
        let variant = match level {
            0 => format!("{} {}", base_command, level),
            1 => format!("echo $({} {})", base_command, level),
            2 => format!("for i in $(seq {}); do {} $i; done", level + 1, base_command),
            3 => format!("if [ $({} {}) -eq {} ]; then echo 'match'; fi", base_command, level, level),
            _ => format!("{} {} | wc -l", base_command, level),
        };
        variants.push(variant);
    }

    variants
}

/// Utility functions for benchmark setup and cleanup
#[allow(dead_code)]
pub mod utils {
    use std::fs;
    use std::process::Command;

    /// Create temporary files for benchmark testing
    pub fn create_temp_files() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut files = Vec::new();
        let temp_dir = "/tmp/rush_benchmarks";

        // Create temp directory
        fs::create_dir_all(temp_dir)?;

        // Create test files
        for i in 0..5 {
            let filename = format!("{}/test_file_{}.txt", temp_dir, i);
            let content = format!("Test file {} content\nLine 2\nLine 3", i);
            fs::write(&filename, content)?;
            files.push(filename);
        }

        Ok(files)
    }

    /// Clean up temporary files
    pub fn cleanup_temp_files(files: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        for file in files {
            if fs::metadata(file).is_ok() {
                fs::remove_file(file)?;
            }
        }
        Ok(())
    }

    /// Check if required external commands are available
    pub fn check_dependencies() -> Result<(), Box<dyn std::error::Error>> {
        let required_commands = ["date", "pwd", "whoami", "cat", "echo", "printf"];

        for cmd in &required_commands {
            if !is_command_available(cmd) {
                return Err(format!("Required command '{}' not found in PATH", cmd).into());
            }
        }

        Ok(())
    }

    /// Check if a command is available in PATH
    fn is_command_available(command: &str) -> bool {
        Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get system information for benchmark context
    pub fn get_system_info() -> std::collections::HashMap<String, String> {
        let mut info = std::collections::HashMap::new();

        // CPU info
        if let Ok(output) = Command::new("uname").arg("-a").output() {
            info.insert("system".to_string(), String::from_utf8_lossy(&output.stdout).trim().to_string());
        }

        // Memory info (Linux specific)
        if let Ok(output) = Command::new("free").arg("-h").output() {
            info.insert("memory".to_string(), String::from_utf8_lossy(&output.stdout).trim().to_string());
        }

        info
    }
}