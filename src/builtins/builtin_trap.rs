use crate::builtins::Builtin;
use crate::parser::ShellCommand;
use crate::state::ShellState;
use std::io::Write;

pub struct TrapBuiltin;

// Signal name to number mapping (POSIX standard signals)
const SIGNAL_MAP: &[(&str, i32)] = &[
    ("HUP", 1),     // Hangup
    ("INT", 2),     // Interrupt (Ctrl+C)
    ("QUIT", 3),    // Quit
    ("ILL", 4),     // Illegal instruction
    ("TRAP", 5),    // Trace trap
    ("ABRT", 6),    // Abort
    ("BUS", 7),     // Bus error
    ("FPE", 8),     // Floating point exception
    ("KILL", 9),    // Kill (cannot be caught)
    ("USR1", 10),   // User-defined signal 1
    ("SEGV", 11),   // Segmentation violation
    ("USR2", 12),   // User-defined signal 2
    ("PIPE", 13),   // Broken pipe
    ("ALRM", 14),   // Alarm clock
    ("TERM", 15),   // Termination
    ("CHLD", 17),   // Child status changed
    ("CONT", 18),   // Continue
    ("STOP", 19),   // Stop (cannot be caught)
    ("TSTP", 20),   // Terminal stop
    ("TTIN", 21),   // Background read from tty
    ("TTOU", 22),   // Background write to tty
    ("URG", 23),    // Urgent condition on socket
    ("XCPU", 24),   // CPU time limit exceeded
    ("XFSZ", 25),   // File size limit exceeded
    ("VTALRM", 26), // Virtual alarm clock
    ("PROF", 27),   // Profiling alarm clock
    ("WINCH", 28),  // Window size change
    ("IO", 29),     // I/O now possible
    ("PWR", 30),    // Power failure
    ("SYS", 31),    // Bad system call
    ("EXIT", 0),    // Special: shell exit (not a real signal)
];

impl TrapBuiltin {
    /// Normalize signal name (remove SIG prefix, convert to uppercase)
    fn normalize_signal_name(signal: &str) -> String {
        let sig = signal.to_uppercase();
        if sig.starts_with("SIG") && sig.len() > 3 {
            sig[3..].to_string()
        } else {
            sig
        }
    }

    /// Convert signal name or number to canonical name
    fn signal_to_name(signal: &str) -> Result<String, String> {
        let normalized = Self::normalize_signal_name(signal);

        // Check if it's a number
        if let Ok(num) = normalized.parse::<i32>() {
            // Find signal by number
            for (name, sig_num) in SIGNAL_MAP {
                if *sig_num == num {
                    return Ok(name.to_string());
                }
            }
            return Err(format!("Invalid signal number: {}", num));
        }

        // Check if it's a valid signal name
        for (name, _) in SIGNAL_MAP {
            if *name == normalized {
                return Ok(name.to_string());
            }
        }

        Err(format!("Invalid signal name: {}", signal))
    }

    /// Check if signal can be trapped
    fn is_trappable(signal_name: &str) -> bool {
        // SIGKILL (9) and SIGSTOP (19) cannot be caught
        !matches!(signal_name, "KILL" | "STOP")
    }

    /// List all signal names
    fn list_signals(output: &mut dyn Write) -> i32 {
        for (name, num) in SIGNAL_MAP {
            if *num > 0 {
                // Skip EXIT (0)
                let _ = writeln!(output, "{}) SIG{}", num, name);
            }
        }
        0
    }

    /// Display trap for a specific signal
    fn display_trap(signal_name: &str, shell_state: &ShellState, output: &mut dyn Write) -> i32 {
        if let Some(command) = shell_state.get_trap(signal_name) {
            let _ = writeln!(output, "trap -- '{}' {}", command, signal_name);
        }
        0
    }

    /// Display all traps
    fn display_all_traps(shell_state: &ShellState, output: &mut dyn Write) -> i32 {
        let traps = shell_state.get_all_traps();
        if traps.is_empty() {
            return 0;
        }

        // Sort by signal name for consistent output
        let mut sorted_traps: Vec<_> = traps.iter().collect();
        sorted_traps.sort_by_key(|(name, _)| *name);

        for (signal_name, command) in sorted_traps {
            let _ = writeln!(output, "trap -- '{}' {}", command, signal_name);
        }
        0
    }

    /// Set trap handler
    fn set_trap(
        action: &str,
        signals: &[String],
        shell_state: &mut ShellState,
        output: &mut dyn Write,
    ) -> i32 {
        let mut exit_code = 0;

        for signal in signals {
            match Self::signal_to_name(signal) {
                Ok(signal_name) => {
                    // Check if signal can be trapped
                    if !Self::is_trappable(&signal_name) {
                        let _ = writeln!(output, "trap: {}: cannot trap signal", signal);
                        exit_code = 1;
                        continue;
                    }

                    if action.is_empty() {
                        // Empty action means ignore the signal
                        shell_state.set_trap(&signal_name, String::new());
                    } else {
                        // Set the trap handler
                        shell_state.set_trap(&signal_name, action.to_string());
                    }
                }
                Err(e) => {
                    let _ = writeln!(output, "trap: {}", e);
                    exit_code = 1;
                }
            }
        }

        exit_code
    }

    /// Reset trap handlers
    fn reset_traps(
        signals: &[String],
        shell_state: &mut ShellState,
        output: &mut dyn Write,
    ) -> i32 {
        let mut exit_code = 0;

        for signal in signals {
            match Self::signal_to_name(signal) {
                Ok(signal_name) => {
                    shell_state.remove_trap(&signal_name);
                }
                Err(e) => {
                    let _ = writeln!(output, "trap: {}", e);
                    exit_code = 1;
                }
            }
        }

        exit_code
    }
}

impl Builtin for TrapBuiltin {
    fn name(&self) -> &'static str {
        "trap"
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["trap"]
    }

    fn description(&self) -> &'static str {
        "Set or display signal handlers"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        let args = &cmd.args;

        // trap with no arguments - display all traps
        if args.len() == 1 {
            return Self::display_all_traps(shell_state, output_writer);
        }

        // trap -l - list all signals
        if args.len() == 2 && args[1] == "-l" {
            return Self::list_signals(output_writer);
        }

        // trap -p [signal...] - print specific traps
        if args.len() >= 2 && args[1] == "-p" {
            if args.len() == 2 {
                // No signals specified, display all
                return Self::display_all_traps(shell_state, output_writer);
            } else {
                // Display specific signals
                let mut exit_code = 0;
                for signal in &args[2..] {
                    match Self::signal_to_name(signal) {
                        Ok(signal_name) => {
                            exit_code =
                                Self::display_trap(&signal_name, shell_state, output_writer);
                        }
                        Err(e) => {
                            let _ = writeln!(output_writer, "trap: {}", e);
                            exit_code = 1;
                        }
                    }
                }
                return exit_code;
            }
        }

        // trap - signal... - reset traps to default
        if args.len() >= 3 && args[1] == "-" {
            return Self::reset_traps(&args[2..], shell_state, output_writer);
        }

        // trap action signal... - set trap handler
        if args.len() >= 3 {
            let action = &args[1];
            let signals: Vec<String> = args[2..].to_vec();
            return Self::set_trap(action, &signals, shell_state, output_writer);
        }

        // Invalid usage
        let _ = writeln!(
            output_writer,
            "trap: usage: trap [-lp] [[action] signal...]"
        );
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_signal_name() {
        assert_eq!(TrapBuiltin::normalize_signal_name("int"), "INT");
        assert_eq!(TrapBuiltin::normalize_signal_name("INT"), "INT");
        assert_eq!(TrapBuiltin::normalize_signal_name("SIGINT"), "INT");
        assert_eq!(TrapBuiltin::normalize_signal_name("sigint"), "INT");
        assert_eq!(TrapBuiltin::normalize_signal_name("SigInt"), "INT");
    }

    #[test]
    fn test_signal_to_name() {
        assert_eq!(TrapBuiltin::signal_to_name("INT").unwrap(), "INT");
        assert_eq!(TrapBuiltin::signal_to_name("SIGINT").unwrap(), "INT");
        assert_eq!(TrapBuiltin::signal_to_name("2").unwrap(), "INT");
        assert_eq!(TrapBuiltin::signal_to_name("15").unwrap(), "TERM");
        assert_eq!(TrapBuiltin::signal_to_name("SIGTERM").unwrap(), "TERM");
        assert!(TrapBuiltin::signal_to_name("INVALID").is_err());
        assert!(TrapBuiltin::signal_to_name("999").is_err());
    }

    #[test]
    fn test_is_trappable() {
        assert!(TrapBuiltin::is_trappable("INT"));
        assert!(TrapBuiltin::is_trappable("TERM"));
        assert!(TrapBuiltin::is_trappable("HUP"));
        assert!(!TrapBuiltin::is_trappable("KILL"));
        assert!(!TrapBuiltin::is_trappable("STOP"));
    }

    #[test]
    fn test_trap_set_handler() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec![
                "trap".to_string(),
                "echo hello".to_string(),
                "INT".to_string(),
            ],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_string_content: None,
        };

        let builtin = TrapBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_trap("INT"), Some("echo hello".to_string()));
    }

    #[test]
    fn test_trap_reset_handler() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Set a trap
        shell_state.set_trap("INT", "echo hello".to_string());
        assert_eq!(shell_state.get_trap("INT"), Some("echo hello".to_string()));

        // Reset the trap
        let cmd = ShellCommand {
            args: vec!["trap".to_string(), "-".to_string(), "INT".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_string_content: None,
        };

        let builtin = TrapBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_trap("INT"), None);
    }

    #[test]
    fn test_trap_invalid_signal() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec![
                "trap".to_string(),
                "echo hello".to_string(),
                "INVALID".to_string(),
            ],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_string_content: None,
        };

        let builtin = TrapBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Invalid signal"));
    }

    #[test]
    fn test_trap_uncatchable_signal() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec![
                "trap".to_string(),
                "echo hello".to_string(),
                "KILL".to_string(),
            ],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_string_content: None,
        };

        let builtin = TrapBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("cannot trap"));
    }

    #[test]
    fn test_trap_multiple_signals() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec![
                "trap".to_string(),
                "echo signal".to_string(),
                "INT".to_string(),
                "TERM".to_string(),
                "HUP".to_string(),
            ],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_string_content: None,
        };

        let builtin = TrapBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_trap("INT"), Some("echo signal".to_string()));
        assert_eq!(
            shell_state.get_trap("TERM"),
            Some("echo signal".to_string())
        );
        assert_eq!(shell_state.get_trap("HUP"), Some("echo signal".to_string()));
    }

    #[test]
    fn test_trap_display_all() {
        let mut shell_state = ShellState::new();
        shell_state.set_trap("INT", "echo int".to_string());
        shell_state.set_trap("TERM", "echo term".to_string());

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["trap".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_string_content: None,
        };

        let builtin = TrapBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("trap -- 'echo int' INT"));
        assert!(output_str.contains("trap -- 'echo term' TERM"));
    }

    #[test]
    fn test_trap_list_signals() {
        let mut output = Vec::new();
        let mut shell_state = ShellState::new();

        let cmd = ShellCommand {
            args: vec!["trap".to_string(), "-l".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_string_content: None,
        };

        let builtin = TrapBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("2) SIGINT"));
        assert!(output_str.contains("15) SIGTERM"));
    }

    #[test]
    fn test_trap_empty_action() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["trap".to_string(), "".to_string(), "INT".to_string()],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_string_content: None,
        };

        let builtin = TrapBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_trap("INT"), Some("".to_string()));
    }

    #[test]
    fn test_trap_signal_numbers() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec![
                "trap".to_string(),
                "echo signal".to_string(),
                "2".to_string(),
            ],
            input: None,
            output: None,
            append: None,
            here_doc_delimiter: None,
            here_string_content: None,
        };

        let builtin = TrapBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        assert_eq!(shell_state.get_trap("INT"), Some("echo signal".to_string()));
    }
}
