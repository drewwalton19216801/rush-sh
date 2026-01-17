use crate::builtins::Builtin;
use crate::parser::ShellCommand;
use crate::state::ShellState;
use std::io::Write;

pub struct KillBuiltin;

impl KillBuiltin {
    /// Parse signal specification from command line argument
    ///
    /// Supports:
    /// - Signal names: TERM, KILL, INT, HUP, STOP, CONT, etc.
    /// - Signal names with SIG prefix: SIGTERM, SIGKILL, etc.
    /// - Non-negative signal numbers: 0 and higher
    /// - Returns signal number on success, error message on failure
    fn parse_signal(signal_str: &str) -> Result<i32, String> {
        // Try to parse as number first
        if let Ok(num) = signal_str.parse::<i32>() {
            if num >= 0 {
                return Ok(num);
            } else {
                return Err(format!("kill: {}: invalid signal specification", signal_str));
            }
        }

        // Parse as signal name (with or without SIG prefix)
        let name = if signal_str.starts_with("SIG") {
            &signal_str[3..]
        } else {
            signal_str
        };

        // Map common signal names to numbers
        match name {
            "HUP" => Ok(libc::SIGHUP),
            "INT" => Ok(libc::SIGINT),
            "QUIT" => Ok(libc::SIGQUIT),
            "ILL" => Ok(libc::SIGILL),
            "TRAP" => Ok(libc::SIGTRAP),
            "ABRT" => Ok(libc::SIGABRT),
            "BUS" => Ok(libc::SIGBUS),
            "FPE" => Ok(libc::SIGFPE),
            "KILL" => Ok(libc::SIGKILL),
            "USR1" => Ok(libc::SIGUSR1),
            "SEGV" => Ok(libc::SIGSEGV),
            "USR2" => Ok(libc::SIGUSR2),
            "PIPE" => Ok(libc::SIGPIPE),
            "ALRM" => Ok(libc::SIGALRM),
            "TERM" => Ok(libc::SIGTERM),
            "CHLD" => Ok(libc::SIGCHLD),
            "CONT" => Ok(libc::SIGCONT),
            "STOP" => Ok(libc::SIGSTOP),
            "TSTP" => Ok(libc::SIGTSTP),
            "TTIN" => Ok(libc::SIGTTIN),
            "TTOU" => Ok(libc::SIGTTOU),
            "URG" => Ok(libc::SIGURG),
            "XCPU" => Ok(libc::SIGXCPU),
            "XFSZ" => Ok(libc::SIGXFSZ),
            "VTALRM" => Ok(libc::SIGVTALRM),
            "PROF" => Ok(libc::SIGPROF),
            "WINCH" => Ok(libc::SIGWINCH),
            "IO" | "POLL" => Ok(libc::SIGIO),
            "SYS" => Ok(libc::SIGSYS),
            _ => Err(format!("kill: {}: invalid signal specification", signal_str)),
        }
    }

    /// List all available signals
    fn list_signals(output_writer: &mut dyn Write) -> i32 {
        let signals = vec![
            (libc::SIGHUP, "HUP"),
            (libc::SIGINT, "INT"),
            (libc::SIGQUIT, "QUIT"),
            (libc::SIGILL, "ILL"),
            (libc::SIGTRAP, "TRAP"),
            (libc::SIGABRT, "ABRT"),
            (libc::SIGBUS, "BUS"),
            (libc::SIGFPE, "FPE"),
            (libc::SIGKILL, "KILL"),
            (libc::SIGUSR1, "USR1"),
            (libc::SIGSEGV, "SEGV"),
            (libc::SIGUSR2, "USR2"),
            (libc::SIGPIPE, "PIPE"),
            (libc::SIGALRM, "ALRM"),
            (libc::SIGTERM, "TERM"),
            (libc::SIGCHLD, "CHLD"),
            (libc::SIGCONT, "CONT"),
            (libc::SIGSTOP, "STOP"),
            (libc::SIGTSTP, "TSTP"),
            (libc::SIGTTIN, "TTIN"),
            (libc::SIGTTOU, "TTOU"),
            (libc::SIGURG, "URG"),
            (libc::SIGXCPU, "XCPU"),
            (libc::SIGXFSZ, "XFSZ"),
            (libc::SIGVTALRM, "VTALRM"),
            (libc::SIGPROF, "PROF"),
            (libc::SIGWINCH, "WINCH"),
            (libc::SIGIO, "IO"),
            (libc::SIGSYS, "SYS"),
        ];

        for (num, name) in signals {
            let _ = writeln!(output_writer, "{}) SIG{}", num, name);
        }

        0
    }

    /// Get PIDs for a target (either PID or jobspec)
    ///
    /// Returns signed PIDs to support POSIX process group signaling:
    /// - Positive PID: signal a single process
    /// - Zero: signal all processes in current process group
    /// - -1: signal all processes (with permissions)
    /// - Negative PID < -1: signal all processes in process group |PID|
    fn get_target_pids(target: &str, shell_state: &ShellState) -> Result<Vec<i32>, String> {
        if target.starts_with('%') {
            // It's a jobspec - convert u32 PIDs to i32
            let job_id = shell_state.job_table.borrow().parse_jobspec(target, "kill")?;
            let job_table = shell_state.job_table.borrow();
            match job_table.get_job(job_id) {
                Some(job) => {
                    if job.pids.is_empty() {
                        Err(format!("kill: %{}: no such job", job_id))
                    } else {
                        // Convert u32 PIDs to i32 (job PIDs are always positive)
                        Ok(job.pids.iter().map(|&pid| pid as i32).collect())
                    }
                }
                None => Err(format!("kill: %{}: no such job", job_id)),
            }
        } else {
            // It's a PID (or process group if negative)
            // Preserve the sign to support POSIX process group signaling
            match target.parse::<i32>() {
                Ok(pid) => Ok(vec![pid]),
                Err(_) => Err(format!("kill: {}: arguments must be process or job IDs", target)),
            }
        }
    }

    /// Send signal to a PID or process group
    ///
    /// Supports POSIX process group signaling:
    /// - pid > 0: Signal the process with ID pid
    /// - pid == 0: Signal all processes in current process group
    /// - pid == -1: Signal all processes (with permissions)
    /// - pid < -1: Signal all processes in process group |pid|
    fn send_signal(pid: i32, signal: i32) -> Result<(), String> {
        // SAFETY: kill is a standard POSIX function for sending signals to processes
        // Pass pid directly to libc::kill to preserve sign for process group signaling
        let result = unsafe { libc::kill(pid as libc::pid_t, signal) };
        
        if result == -1 {
            let err = std::io::Error::last_os_error();
            match err.raw_os_error() {
                Some(libc::ESRCH) => {
                    if pid < -1 {
                        Err(format!("kill: ({}): No such process group", pid.abs()))
                    } else {
                        Err(format!("kill: ({}): No such process", pid))
                    }
                }
                Some(libc::EPERM) => Err(format!("kill: ({}): Operation not permitted", pid)),
                _ => Err(format!("kill: ({}): {}", pid, err)),
            }
        } else {
            Ok(())
        }
    }
}

impl Builtin for KillBuiltin {
    fn name(&self) -> &'static str {
        "kill"
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["kill"]
    }

    fn description(&self) -> &'static str {
        "Send a signal to a job or process"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        let args = &cmd.args;

        // Need at least one argument
        if args.len() < 2 {
            let _ = writeln!(output_writer, "kill: usage: kill [-s sigspec | -n signum | -sigspec] pid | jobspec ...");
            let _ = writeln!(output_writer, "       kill -l [sigspec]");
            return 1;
        }

        let mut signal = libc::SIGTERM; // Default signal
        let mut arg_index = 1;
        let mut targets = Vec::new();
        let mut signal_specified_with_option = false; // Track if signal was specified with -s or -n

        // Parse options and signal specification
        while arg_index < args.len() {
            let arg = &args[arg_index];

            if arg == "--" {
                // End of options marker - remaining arguments are targets
                arg_index += 1;
                break;
            } else if arg == "-l" {
                // List signals
                return Self::list_signals(output_writer);
            } else if arg == "-s" {
                // Signal specified with -s option
                arg_index += 1;
                if arg_index >= args.len() {
                    let _ = writeln!(output_writer, "kill: -s: option requires an argument");
                    return 1;
                }
                match Self::parse_signal(&args[arg_index]) {
                    Ok(sig) => {
                        signal = sig;
                        signal_specified_with_option = true;
                    }
                    Err(e) => {
                        let _ = writeln!(output_writer, "{}", e);
                        return 1;
                    }
                }
                arg_index += 1;
            } else if arg == "-n" {
                // Signal specified with -n option (number)
                arg_index += 1;
                if arg_index >= args.len() {
                    let _ = writeln!(output_writer, "kill: -n: option requires an argument");
                    return 1;
                }
                match args[arg_index].parse::<i32>() {
                    Ok(num) if num >= 0 => {
                        signal = num;
                        signal_specified_with_option = true;
                    }
                    _ => {
                        let _ = writeln!(output_writer, "kill: {}: invalid signal specification", args[arg_index]);
                        return 1;
                    }
                }
                arg_index += 1;
            } else if !signal_specified_with_option && arg.starts_with('-') && arg.len() > 1 {
                // Signal specified with -SIGNAME or -NUM format (only if not already specified with -s/-n)
                let sig_spec = &arg[1..];
                match Self::parse_signal(sig_spec) {
                    Ok(sig) => {
                        signal = sig;
                        signal_specified_with_option = true;
                    }
                    Err(e) => {
                        let _ = writeln!(output_writer, "{}", e);
                        return 1;
                    }
                }
                arg_index += 1;
            } else {
                // This and remaining arguments are targets
                break;
            }
        }

        // Collect all targets
        while arg_index < args.len() {
            targets.push(args[arg_index].clone());
            arg_index += 1;
        }

        // Need at least one target
        if targets.is_empty() {
            let _ = writeln!(output_writer, "kill: usage: kill [-s sigspec | -n signum | -sigspec] pid | jobspec ...");
            return 1;
        }

        // Send signal to all targets
        let mut exit_code = 0;
        for target in targets {
            match Self::get_target_pids(&target, shell_state) {
                Ok(pids) => {
                    for pid in pids {
                        if let Err(e) = Self::send_signal(pid, signal) {
                            let _ = writeln!(output_writer, "{}", e);
                            exit_code = 1;
                        }
                    }
                }
                Err(e) => {
                    let _ = writeln!(output_writer, "{}", e);
                    exit_code = 1;
                }
            }
        }

        exit_code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Job;
    use std::sync::Mutex;

    // Mutex to serialize tests that modify environment variables
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_kill_no_arguments() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["kill".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("usage"));
    }

    #[test]
    fn test_kill_list_signals() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-l".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("SIGTERM"));
        assert!(output_str.contains("SIGKILL"));
        assert!(output_str.contains("SIGINT"));
    }

    #[test]
    fn test_parse_signal_by_name() {
        assert_eq!(KillBuiltin::parse_signal("TERM").unwrap(), libc::SIGTERM);
        assert_eq!(KillBuiltin::parse_signal("KILL").unwrap(), libc::SIGKILL);
        assert_eq!(KillBuiltin::parse_signal("INT").unwrap(), libc::SIGINT);
        assert_eq!(KillBuiltin::parse_signal("HUP").unwrap(), libc::SIGHUP);
    }

    #[test]
    fn test_parse_signal_with_sig_prefix() {
        assert_eq!(KillBuiltin::parse_signal("SIGTERM").unwrap(), libc::SIGTERM);
        assert_eq!(KillBuiltin::parse_signal("SIGKILL").unwrap(), libc::SIGKILL);
        assert_eq!(KillBuiltin::parse_signal("SIGINT").unwrap(), libc::SIGINT);
    }

    #[test]
    fn test_parse_signal_by_number() {
        assert_eq!(KillBuiltin::parse_signal("9").unwrap(), 9);
        assert_eq!(KillBuiltin::parse_signal("15").unwrap(), 15);
        assert_eq!(KillBuiltin::parse_signal("1").unwrap(), 1);
    }

    #[test]
    fn test_parse_signal_invalid() {
        assert!(KillBuiltin::parse_signal("INVALID").is_err());
        assert!(KillBuiltin::parse_signal("-1").is_err());
    }

    #[test]
    fn test_kill_with_signal_name() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Try to kill the current process with signal 0 (test if process exists)
        let pid = std::process::id();
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-s".to_string(), "0".to_string(), pid.to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Signal 0 is now valid (null signal to test if process exists), should succeed
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_kill_invalid_pid() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("No such process") || output_str.contains("Operation not permitted"));
    }

    #[test]
    fn test_kill_jobspec_no_jobs() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no such job"));
    }

    #[test]
    fn test_kill_jobspec_current() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add a job with the current process PID (so we can test without actually killing)
        let pid = std::process::id();
        let job = Job::new(1, Some(pid), "sleep 10 &".to_string(), vec![pid], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-s".to_string(), "0".to_string(), "%".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Signal 0 is now valid (null signal to test if process exists), should succeed
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_kill_multiple_targets() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Use invalid PIDs that don't exist
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "999998".to_string(), "999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        // Should have error messages for both PIDs
        assert!(output_str.contains("999998") || output_str.contains("999999"));
    }

    #[test]
    fn test_kill_with_dash_signal() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Use an invalid PID instead of our own process
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-TERM".to_string(), "999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Should fail because PID doesn't exist
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("No such process") || output_str.contains("Operation not permitted"));
    }

    #[test]
    fn test_kill_with_dash_number() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-9".to_string(), "999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("No such process") || output_str.contains("Operation not permitted"));
    }

    #[test]
    fn test_kill_with_n_option() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-n".to_string(), "15".to_string(), "999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("No such process") || output_str.contains("Operation not permitted"));
    }

    #[test]
    fn test_kill_missing_signal_argument() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-s".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("option requires an argument"));
    }

    #[test]
    fn test_kill_no_targets_after_signal() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-TERM".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("usage"));
    }

    #[test]
    fn test_parse_jobspec_current() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let result = shell_state.job_table.borrow().parse_jobspec("%", "kill");
        assert_eq!(result.unwrap(), 1);

        let result = shell_state.job_table.borrow().parse_jobspec("%+", "kill");
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_parse_jobspec_previous() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();
        let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let result = shell_state.job_table.borrow().parse_jobspec("%-", "kill");
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_parse_jobspec_by_number() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();
        let job = Job::new(5, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let result = shell_state.job_table.borrow().parse_jobspec("%5", "kill");
        assert_eq!(result.unwrap(), 5);
    }

    #[test]
    fn test_parse_jobspec_by_prefix() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let result = shell_state.job_table.borrow().parse_jobspec("%sleep", "kill");
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_parse_jobspec_by_contains() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();
        let job = Job::new(1, Some(1234), "grep pattern file &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let result = shell_state.job_table.borrow().parse_jobspec("%?pattern", "kill");
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_get_target_pids_for_job() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234, 1235], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let result = KillBuiltin::get_target_pids("%1", &shell_state);
        assert!(result.is_ok());
        let pids = result.unwrap();
        assert_eq!(pids.len(), 2);
        assert!(pids.contains(&1234));
        assert!(pids.contains(&1235));
    }

    #[test]
    fn test_get_target_pids_for_pid() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();

        let result = KillBuiltin::get_target_pids("1234", &shell_state);
        assert!(result.is_ok());
        let pids = result.unwrap();
        assert_eq!(pids.len(), 1);
        assert_eq!(pids[0], 1234);
    }

    #[test]
    fn test_kill_job_with_multiple_pids() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add a job with multiple PIDs (use invalid PIDs so we don't actually kill anything)
        let job = Job::new(1, Some(999998), "sleep 10 | cat &".to_string(), vec![999998, 999999], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        // Should have error messages for both PIDs
        assert!(output_str.contains("999998") || output_str.contains("999999"));
    }

    #[test]
    fn test_get_target_pids_negative_pid() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();

        // Test negative PID (process group)
        let result = KillBuiltin::get_target_pids("-123", &shell_state);
        assert!(result.is_ok());
        let pids = result.unwrap();
        assert_eq!(pids.len(), 1);
        assert_eq!(pids[0], -123);
    }

    #[test]
    fn test_get_target_pids_zero() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();

        // Test PID 0 (current process group)
        let result = KillBuiltin::get_target_pids("0", &shell_state);
        assert!(result.is_ok());
        let pids = result.unwrap();
        assert_eq!(pids.len(), 1);
        assert_eq!(pids[0], 0);
    }

    #[test]
    fn test_get_target_pids_minus_one() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();

        // Test PID -1 (all processes)
        let result = KillBuiltin::get_target_pids("-1", &shell_state);
        assert!(result.is_ok());
        let pids = result.unwrap();
        assert_eq!(pids.len(), 1);
        assert_eq!(pids[0], -1);
    }

    #[test]
    fn test_get_target_pids_positive_pid() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();

        // Test positive PID (single process)
        let result = KillBuiltin::get_target_pids("1234", &shell_state);
        assert!(result.is_ok());
        let pids = result.unwrap();
        assert_eq!(pids.len(), 1);
        assert_eq!(pids[0], 1234);
    }

    #[test]
    fn test_send_signal_preserves_negative_pid() {
        // Test that send_signal correctly passes negative PIDs to libc::kill
        // We use a non-existent process group to avoid actually signaling anything
        let result = KillBuiltin::send_signal(-999999, libc::SIGTERM);
        
        // Should fail with ESRCH (no such process group)
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("No such process group") || err_msg.contains("999999"));
    }

    #[test]
    fn test_send_signal_zero_pid() {
        // Test that send_signal handles PID 0 (current process group)
        // This should succeed if we have permission to signal our own process group
        let result = KillBuiltin::send_signal(0, 0);
        
        // Signal 0 is a null signal used to check if we can signal the process
        // This should succeed for our own process group
        assert!(result.is_ok());
    }

    #[test]
    fn test_kill_negative_pid_process_group() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Try to kill a non-existent process group
        // Use -s to specify signal explicitly, then negative PID
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-s".to_string(), "TERM".to_string(), "-999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Should fail because process group doesn't exist
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("No such process group") || output_str.contains("999999"));
    }

    #[test]
    fn test_kill_zero_current_process_group() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Signal 0 to current process group (should succeed)
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-s".to_string(), "0".to_string(), "0".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Signal 0 is now valid (null signal), should succeed
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_kill_minus_one_all_processes() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Try to signal all processes (will likely fail due to permissions)
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-s".to_string(), "0".to_string(), "-1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Signal 0 is now valid, but -1 may fail due to permissions
        // Accept either success (0) or permission error (1)
        assert!(exit_code == 0 || exit_code == 1);
    }

    #[test]
    fn test_parse_signal_accepts_zero() {
        // Signal 0 should now be accepted (null signal to test process existence)
        let result = KillBuiltin::parse_signal("0");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_kill_multiple_negative_pids() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Try to kill multiple non-existent process groups
        // Use default signal (TERM) and specify negative PIDs explicitly with -s
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-s".to_string(), "TERM".to_string(), "-999998".to_string(), "-999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Should fail for both process groups
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        // Should have error messages for both process groups (check for "process group" or the PIDs)
        assert!(output_str.contains("process group") || output_str.contains("999998") || output_str.contains("999999"));
    }

    #[test]
    fn test_kill_mixed_positive_and_negative_pids() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Mix of positive PID and negative PID (process group) - use -- for negative PID
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "--".to_string(), "999998".to_string(), "-999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Should fail for both
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("999998") || output_str.contains("999999"));
    }

    #[test]
    fn test_get_target_pids_preserves_sign_for_large_negative() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();

        // Test large negative PID
        let result = KillBuiltin::get_target_pids("-32768", &shell_state);
        assert!(result.is_ok());
        let pids = result.unwrap();
        assert_eq!(pids.len(), 1);
        assert_eq!(pids[0], -32768);
    }

    #[test]
    fn test_kill_double_dash_separator() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Use -- to separate options from arguments
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-s".to_string(), "TERM".to_string(), "--".to_string(), "-999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Should fail because process group doesn't exist
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("No such process group") || output_str.contains("999999"));
    }

    #[test]
    fn test_kill_double_dash_with_default_signal() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Use -- with default signal (TERM)
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "--".to_string(), "-999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Should fail because process group doesn't exist
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("No such process group") || output_str.contains("999999"));
    }

    #[test]
    fn test_kill_negative_pid_without_double_dash() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Without --, negative numbers are treated as signal specifications
        // -999999 is not a valid signal, so it should fail with invalid signal error
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Should fail with invalid signal specification
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("invalid signal specification") || output_str.contains("usage"));
    }

    #[test]
    fn test_kill_ambiguous_negative_number_as_signal() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // -9 should be interpreted as signal 9 (SIGKILL), not PID -9
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-9".to_string(), "999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Should fail because PID doesn't exist
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("No such process") || output_str.contains("999999"));
    }

    #[test]
    fn test_kill_negative_pid_after_explicit_signal() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // After explicit signal with -s, negative numbers should be PIDs
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-s".to_string(), "TERM".to_string(), "-999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Should fail because process group doesn't exist
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("No such process group") || output_str.contains("999999"));
    }

    #[test]
    fn test_kill_double_dash_multiple_negative_pids() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Multiple negative PIDs after --
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "--".to_string(), "-999998".to_string(), "-999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Should fail for both process groups
        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("process group") || output_str.contains("999998") || output_str.contains("999999"));
    }

    #[test]
    fn test_kill_invalid_signal_with_negative_number() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // -INVALID should fail as invalid signal, not be treated as negative PID
        let cmd = ShellCommand {
            args: vec!["kill".to_string(), "-INVALID".to_string(), "999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = KillBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("invalid signal specification"));
    }
}
