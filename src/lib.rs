//! Rush Shell Library
//!
//! This library provides the core functionality of the Rush shell,
//! exposing modules for external use such as benchmarking and testing.

pub mod arithmetic;
pub mod brace_expansion;
pub mod builtins;
pub mod completion;
pub mod executor;
pub mod fd_manager;
pub mod lexer;
pub mod parameter_expansion;
pub mod parser;
pub mod state;

// Re-export main types for convenience
pub use executor::execute;
pub use lexer::Token;
pub use parser::{Ast, ShellCommand};
pub use state::ShellState;
