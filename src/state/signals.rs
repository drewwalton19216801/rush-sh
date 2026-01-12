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
//! 1. Signal handler thread (via signal_hook) enqueues signal events into `SIGNAL_QUEUE`
//! 2. Main execution thread processes pending signals at safe points
//! 3. Queue has a maximum size to prevent memory exhaustion
//! 4. Oldest signals are dropped when queue is full
//!
//! # Thread Safety vs Async-Signal-Safety
//!
//! **IMPORTANT**: The signal queue uses `Arc<Mutex<VecDeque<SignalEvent>>>` which provides
//! thread safety but is **NOT async-signal-safe**. This means:
//!
//! - ✅ Safe to call from dedicated signal-handling threads (like signal_hook provides)
//! - ✅ Safe to call from normal application threads
//! - ❌ **NOT safe** to call directly from POSIX signal handlers (sigaction)
//!
//! The `Mutex::lock()` and `eprintln!()` operations used in this module can deadlock or
//! cause undefined behavior if called from a POSIX signal handler context.
//!
//! ## Recommended Signal Handling Approaches
//!
//! For true async-signal-safety, consider these alternatives:
//!
//! 1. **signal_hook's iterator pattern** (current approach):
//!    - Uses a dedicated thread to receive signals
//!    - Calls `enqueue_signal` from that thread (safe)
//!
//! 2. **Self-pipe trick**:
//!    - Signal handler writes to a pipe (async-signal-safe)
//!    - Main thread reads from pipe and processes signals
//!
//! 3. **Lock-free atomic buffers**:
//!    - Use atomic operations instead of mutexes
//!    - Requires careful implementation to avoid race conditions
//!
//! 4. **signalfd (Linux-specific)**:
//!    - Converts signals to file descriptor events
//!    - Can be integrated with event loops
//!
//! # Usage
//!
//! ```rust,no_run
//! use rush_sh::state::{enqueue_signal, process_pending_signals, ShellState};
//!
//! // In signal handler thread (via signal_hook):
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
    ///
    /// # Safety Warning
    ///
    /// This queue uses `Mutex` which is **NOT async-signal-safe**. It must only be accessed from:
    /// - Dedicated signal-handling threads (like signal_hook provides)
    /// - Normal application threads
    ///
    /// **Never** access this queue directly from a POSIX signal handler (sigaction) as it can
    /// cause deadlocks or undefined behavior.
    pub static ref SIGNAL_QUEUE: Arc<Mutex<VecDeque<SignalEvent>>> =
        Arc::new(Mutex::new(VecDeque::new()));
}

/// Maximum number of signals to queue before dropping old ones
///
/// This prevents unbounded memory growth if signals arrive faster than they can be processed.
/// When the queue is full, the oldest signal is dropped to make room for new ones.
///
/// # Implementation Note
///
/// The overflow handling in [`enqueue_signal`] uses `eprintln!()` which is not async-signal-safe.
/// This is acceptable because `enqueue_signal` is designed to be called from a dedicated
/// signal-handling thread, not from POSIX signal handlers.
const MAX_SIGNAL_QUEUE_SIZE: usize = 100;

/// Represents a signal event that needs to be processed
///
/// Signal events are created when a signal is received and queued for later processing
/// by the main execution thread. Each event captures the signal name, number, and timestamp.
///
/// # Safety Note
///
/// Creating a `SignalEvent` calls `Instant::now()` which may not be async-signal-safe on all
/// platforms. This struct should only be instantiated from safe contexts (dedicated signal
/// threads, not POSIX signal handlers).
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
/// This function is called by a dedicated signal handler thread when a signal is received.
/// If the queue is full, the oldest event is dropped to make room for the new one.
///
/// # Arguments
///
/// * `signal_name` - The name of the signal (e.g., "INT", "TERM")
/// * `signal_number` - The numeric signal value (e.g., 2, 15)
///
/// # Safety Warning - NOT Async-Signal-Safe
///
/// **CRITICAL**: This function is **NOT async-signal-safe** and must **NOT** be called directly
/// from POSIX signal handlers (sigaction). It uses:
/// - `Mutex::lock()` - can deadlock if called from signal handler
/// - `eprintln!()` - not async-signal-safe, can cause undefined behavior
///
/// ## Safe Usage Contexts
///
/// ✅ **Safe to call from**:
/// - Dedicated signal-handling threads (like signal_hook's iterator pattern)
/// - Normal application threads
/// - After receiving signals via safe relay mechanisms (signalfd, self-pipe)
///
/// ❌ **NEVER call from**:
/// - POSIX signal handlers registered with sigaction
/// - Any context where async-signal-safety is required
///
/// ## Recommended Patterns
///
/// If you need true async-signal-safety, use one of these approaches instead:
///
/// 1. **signal_hook iterator** (current approach):
///    ```rust,no_run
///    use rush_sh::state::enqueue_signal;
///    use signal_hook::consts::{SIGINT, SIGTERM};
///    use signal_hook::iterator::Signals;
///    use std::thread;
///
///    let mut signals = Signals::new(&[SIGINT, SIGTERM]).unwrap();
///    thread::spawn(move || {
///        for sig in signals.forever() {
///            // Map signal number to name
///            let signal_name = match sig {
///                SIGINT => "INT",
///                SIGTERM => "TERM",
///                _ => continue, // Skip unknown signals
///            };
///            enqueue_signal(signal_name, sig); // Safe: called from dedicated thread
///        }
///    });
///    ```
///
/// 2. **Self-pipe trick**:
///    - Signal handler writes signal number to pipe (async-signal-safe)
///    - Main thread reads from pipe and calls `enqueue_signal`
///
/// 3. **Lock-free atomic buffer**:
///    - Replace `Mutex<VecDeque>` with atomic operations
///    - More complex but truly async-signal-safe
///
/// # Examples
///
/// ```rust,no_run
/// use rush_sh::state::enqueue_signal;
///
/// // ✅ SAFE: Called from signal_hook's dedicated thread
/// enqueue_signal("INT", 2);
///
/// // ❌ UNSAFE: Never do this in a sigaction handler!
/// // extern "C" fn signal_handler(sig: i32) {
/// //     enqueue_signal("INT", sig); // DEADLOCK RISK!
/// // }
/// ```
pub fn enqueue_signal(signal_name: &str, signal_number: i32) {
    match SIGNAL_QUEUE.lock() {
        Ok(mut queue) => {
            // If queue is full, remove oldest event
            if queue.len() >= MAX_SIGNAL_QUEUE_SIZE {
                queue.pop_front();
                eprintln!("Warning: Signal queue overflow, dropping oldest signal");
            }

            queue.push_back(SignalEvent::new(signal_name.to_string(), signal_number));
        }
        Err(_) => {
            // Lock poisoned - another thread panicked while holding the lock
            // Cannot safely enqueue; signal is dropped to avoid cascading failures
            #[cfg(debug_assertions)]
            eprintln!("Warning: Signal queue lock poisoned, dropping signal {}", signal_name);
        }
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
    // Drain all pending signals into a local collection while holding the lock
    let pending_signals = {
        if let Ok(mut queue) = SIGNAL_QUEUE.lock() {
            // Drain all signals from the queue into a local Vec
            let mut signals = Vec::new();
            while let Some(signal_event) = queue.pop_front() {
                signals.push(signal_event);
            }
            signals
        } else {
            // If we can't acquire the lock, return early
            return;
        }
    }; // Lock is dropped here

    // Process all signals without holding the lock
    // This prevents deadlock if a trap handler enqueues a signal
    for signal_event in pending_signals {
        // Check if a trap is set for this signal
        if let Some(trap_cmd) = shell_state.get_trap(&signal_event.signal_name) {
            if !trap_cmd.is_empty() {
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
