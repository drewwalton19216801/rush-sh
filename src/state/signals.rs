//! Signal Management Module
//!
//! This module provides signal handling infrastructure for the Rush shell, including:
//! - Signal event queuing and processing
//! - Thread-safe signal queue with overflow protection
//! - Integration with trap handlers
//!
//! # Signal Queue Mechanism
//!
//! Signals are handled asynchronously using a global queue:
//! 1. Signal handler thread enqueues signal events into `SIGNAL_QUEUE`
//! 2. Main execution thread processes pending signals at safe points
//! 3. Queue has a maximum size to prevent memory exhaustion
//! 4. Oldest signals are dropped when queue is full
//!
//! # Thread Safety
//!
//! The signal queue uses `Arc<Mutex<VecDeque<SignalEvent>>>` to ensure:
//! - Thread-safe access from signal handler and main thread
//! - Proper synchronization between enqueue and dequeue operations
//! - No data races or memory corruption
//!
//! # Usage
//!
//! ```rust,no_run
//! use rush_sh::state::{enqueue_signal, process_pending_signals, ShellState};
//!
//! // In signal handler thread:
//! enqueue_signal("INT", 2);
//!
//! // In main execution loop:
//! let mut shell_state = ShellState::new();
//! process_pending_signals(&mut shell_state);
//! ```

use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::ShellState;

lazy_static! {
    /// Global queue for pending signal events
    ///
    /// Signals are enqueued by the signal handler thread and dequeued by the main thread.
    /// This provides a thread-safe mechanism for asynchronous signal handling.
    pub static ref SIGNAL_QUEUE: Arc<Mutex<VecDeque<SignalEvent>>> =
        Arc::new(Mutex::new(VecDeque::new()));
}

/// Maximum number of signals to queue before dropping old ones
///
/// This prevents unbounded memory growth if signals arrive faster than they can be processed.
/// When the queue is full, the oldest signal is dropped to make room for new ones.
const MAX_SIGNAL_QUEUE_SIZE: usize = 100;

/// Represents a signal event that needs to be processed
///
/// Signal events are created when a signal is received and queued for later processing
/// by the main execution thread. Each event captures the signal name, number, and timestamp.
#[derive(Debug, Clone)]
pub struct SignalEvent {
    /// Signal name (e.g., "INT", "TERM", "HUP")
    pub signal_name: String,
    /// Signal number (e.g., 2 for SIGINT, 15 for SIGTERM)
    pub signal_number: i32,
    /// When the signal was received (for debugging and ordering)
    pub timestamp: Instant,
}

impl SignalEvent {
    /// Create a new signal event with the current timestamp
    ///
    /// # Arguments
    ///
    /// * `signal_name` - The name of the signal (e.g., "INT", "TERM")
    /// * `signal_number` - The numeric signal value (e.g., 2, 15)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rush_sh::state::signals::SignalEvent;
    ///
    /// let event = SignalEvent::new("INT".to_string(), 2);
    /// assert_eq!(event.signal_name, "INT");
    /// assert_eq!(event.signal_number, 2);
    /// ```
    pub fn new(signal_name: String, signal_number: i32) -> Self {
        Self {
            signal_name,
            signal_number,
            timestamp: Instant::now(),
        }
    }
}

/// Enqueue a signal event for later processing
///
/// This function is called by the signal handler thread when a signal is received.
/// If the queue is full, the oldest event is dropped to make room for the new one.
///
/// # Arguments
///
/// * `signal_name` - The name of the signal (e.g., "INT", "TERM")
/// * `signal_number` - The numeric signal value (e.g., 2, 15)
///
/// # Thread Safety
///
/// This function is thread-safe and can be called from signal handlers.
/// It uses a mutex to protect the signal queue from concurrent access.
///
/// # Examples
///
/// ```rust,no_run
/// use rush_sh::state::enqueue_signal;
///
/// // Called from signal handler
/// enqueue_signal("INT", 2);
/// ```
pub fn enqueue_signal(signal_name: &str, signal_number: i32) {
    if let Ok(mut queue) = SIGNAL_QUEUE.lock() {
        // If queue is full, remove oldest event
        if queue.len() >= MAX_SIGNAL_QUEUE_SIZE {
            queue.pop_front();
            eprintln!("Warning: Signal queue overflow, dropping oldest signal");
        }

        queue.push_back(SignalEvent::new(signal_name.to_string(), signal_number));
    }
}

/// Process all pending signals in the queue
///
/// This function should be called at safe points during command execution to handle
/// any signals that have been received. For each signal, if a trap handler is set,
/// the handler is executed.
///
/// # Arguments
///
/// * `shell_state` - Mutable reference to the shell state for trap handler execution
///
/// # Behavior
///
/// - Processes all signals currently in the queue
/// - Executes trap handlers for signals that have them
/// - Preserves exit codes as per POSIX requirements
/// - Displays signal information when colors are enabled
///
/// # Examples
///
/// ```rust,no_run
/// use rush_sh::state::{ShellState, process_pending_signals};
///
/// let mut shell_state = ShellState::new();
/// process_pending_signals(&mut shell_state);
/// ```
pub fn process_pending_signals(shell_state: &mut ShellState) {
    // Try to lock the queue with a timeout to avoid blocking
    if let Ok(mut queue) = SIGNAL_QUEUE.lock() {
        // Process all pending signals
        while let Some(signal_event) = queue.pop_front() {
            // Check if a trap is set for this signal
            if let Some(trap_cmd) = shell_state.get_trap(&signal_event.signal_name)
                && !trap_cmd.is_empty()
            {
                // Display signal information for debugging/tracking
                if shell_state.colors_enabled {
                    eprintln!(
                        "{}Signal {} (signal {}) received at {:?}\x1b[0m",
                        shell_state.color_scheme.builtin,
                        signal_event.signal_name,
                        signal_event.signal_number,
                        signal_event.timestamp
                    );
                } else {
                    eprintln!(
                        "Signal {} (signal {}) received at {:?}",
                        signal_event.signal_name,
                        signal_event.signal_number,
                        signal_event.timestamp
                    );
                }

                // Execute the trap handler
                // Note: This preserves the exit code as per POSIX requirements
                crate::executor::execute_trap_handler(&trap_cmd, shell_state);
            }
        }
    }
}