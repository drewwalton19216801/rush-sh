//! Rush Shell Library
//!
//! This library provides the core functionality of the Rush shell,
//! exposing modules for external use such as benchmarking and testing.

pub mod arithmetic;
pub mod brace_expansion;
pub mod builtins;
pub mod completion;
pub mod executor;
pub mod lexer;
pub mod parameter_expansion;
pub mod parser;
pub mod script_engine;
pub mod state;

// Re-export main types for convenience
pub use executor::execute;
pub use lexer::Token;
pub use parser::{Ast, ShellCommand};
pub use state::ShellState;

// Global test synchronization mutexes
// These are always available but only used in tests
#[doc(hidden)]
pub mod test_sync {
    use std::sync::Mutex;
    
    /// Mutex to serialize tests that modify job control state
    /// Job control tests MUST use this mutex to prevent race conditions
    /// when accessing the global job table through ShellState
    pub static JOB_CONTROL_LOCK: Mutex<()> = Mutex::new(());
    
    /// Mutex to serialize tests that modify environment variables
    pub static ENV_LOCK: Mutex<()> = Mutex::new(());
    
    /// Mutex to serialize tests that change the current directory
    pub static DIR_CHANGE_LOCK: Mutex<()> = Mutex::new(());
}
