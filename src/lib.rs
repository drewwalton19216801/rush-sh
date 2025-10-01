//! Rush Shell Library
//!
//! This library provides the core functionality of the Rush shell,
//! exposing modules for external use such as benchmarking and testing.

pub mod lexer;
pub mod parser;
pub mod executor;
pub mod state;
pub mod arithmetic;
pub mod parameter_expansion;
pub mod brace_expansion;
pub mod builtins;
pub mod completion;

// Re-export main types for convenience
pub use state::ShellState;
pub use lexer::Token;
pub use parser::{Ast, ShellCommand};
pub use executor::execute;