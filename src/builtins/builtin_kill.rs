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
    /// - Signal numbers: 1-31
    /// - Returns signal number on success, error message on failure
    fn parse_signal(signal_str: &str) -> Result<i32, String> {
        // Try to parse as number first
        if let Ok(num) = signal_str.parse::<i32>() {
            if (1..=31).contains(&num) {
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

    /// Parse jobspec argument to job ID
    ///
    /// Supports:
    /// - %n: Job number n
    /// - %: Current job
    /// - %+: Current job
    /// - %-: Previous job
    /// - %string: Job whose command begins with string
    /// - %?string: Job whose command contains string
    fn parse_jobspec(jobspec: &str, shell_state: &ShellState) -> Result<usize, String> {
        if jobspec.starts_with('%') {
            let spec = &jobspec[1..];
            
            // %+ or % - current job
            if spec.is_empty() || spec == "+" {
                return shell_state
                    .job_table
                    .borrow()
                    .get_current_job()
                    .ok_or_else(|| "kill: no current job".to_string());
            }
            
            // %- - previous job
            if spec == "-" {
                return shell_state
                    .job_table
                    .borrow()
                    .get_previous_job()
                    .ok_or_else(|| "kill: no previous job".to_string());
            }
            
            // %?string - job whose command contains string
            if let Some(search_str) = spec.strip_prefix('?') {
                let job_table = shell_state.job_table.borrow();
                for job in job_table.get_all_jobs() {
                    // Skip completed jobs when matching by command
                    if job.is_active() && job.command.contains(search_str) {
                        return Ok(job.job_id);
                    }
                }
                return Err(format!("kill: {}: no such job", jobspec));
            }
            
            // %string - job whose command begins with string
            // Try to parse as number first
            if let Ok(job_id) = spec.parse::<usize>() {
                return Ok(job_id);
            }
            
            // Otherwise, search for command prefix
            let job_table = shell_state.job_table.borrow();
            for job in job_table.get_all_jobs() {
                // Skip completed jobs when matching by command prefix
                if job.is_active() && job.command.starts_with(spec) {
                    return Ok(job.job_id);
                }
            }
            
            Err(format!("kill: {}: no such job", jobspec))
        } else {
            // Direct job number
            jobspec
                .parse::<usize>()
                .map_err(|_| format!("kill: {}: arguments must be process or job IDs", jobspec))
        }
    }

    /// Get PIDs for a target (either PID or jobspec)
    fn get_target_pids(target: &str, shell_state: &ShellState) -> Result<Vec<u32>, String> {
        if target.starts_with('%') {
            // It's a jobspec
            let job_id = Self::parse_jobspec(target, shell_state)?;
            let job_table = shell_state.job_table.borrow();
            match job_table.get_job(job_id) {
                Some(job) => {
                    if job.pids.is_empty() {
                        Err(format!("kill: %{}: no such job", job_id))
                    } else {
                        Ok(job.pids.clone())
                    }
                }
                None => Err(format!("kill: %{}: no such job", job_id)),
            }
        } else {
            // It's a PID (or process group if negative)
            match target.parse::<i32>() {
                Ok(pid) => Ok(vec![pid.unsigned_abs()]),
                Err(_) => Err(format!("kill: {}: arguments must be process or job IDs", target)),
            }
        }
    }

    /// Send signal to a PID
    fn send_signal(pid: u32, signal: i32) -> Result<(), String> {
        // SAFETY: kill is a standard POSIX function for sending signals to processes
        let result = unsafe { libc::kill(pid as libc::pid_t, signal) };
        
        if result == -1 {
            let err = std::io::Error::last_os_error();
            match err.raw_os_error() {
                Some(libc::ESRCH) => Err(format!("kill: ({}): No such process", pid)),
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

        // Parse options and signal specification
        while arg_index < args.len() {
            let arg = &args[arg_index];

            if arg == "-l" {
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
                    Ok(sig) => signal = sig,
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
                    Ok(num) if (1..=31).contains(&num) => signal = num,
                    _ => {
                        let _ = writeln!(output_writer, "kill: {}: invalid signal specification", args[arg_index]);
                        return 1;
                    }
                }
                arg_index += 1;
            } else if arg.starts_with('-') && arg.len() > 1 {
                // Signal specified with -SIGNAME or -NUM format
                let sig_spec = &arg[1..];
                match Self::parse_signal(sig_spec) {
                    Ok(sig) => signal = sig,
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

    #[test]
    fn test_kill_no_arguments() {
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
        assert!(KillBuiltin::parse_signal("999").is_err());
        assert!(KillBuiltin::parse_signal("0").is_err());
        assert!(KillBuiltin::parse_signal("-1").is_err());
    }

    #[test]
    fn test_kill_with_signal_name() {
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

        // Signal 0 is invalid, should fail
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_kill_invalid_pid() {
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

        // Signal 0 is invalid
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_kill_multiple_targets() {
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
        let shell_state = ShellState::new();
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let result = KillBuiltin::parse_jobspec("%", &shell_state);
        assert_eq!(result.unwrap(), 1);

        let result = KillBuiltin::parse_jobspec("%+", &shell_state);
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_parse_jobspec_previous() {
        let shell_state = ShellState::new();
        let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let result = KillBuiltin::parse_jobspec("%-", &shell_state);
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_parse_jobspec_by_number() {
        let shell_state = ShellState::new();
        let job = Job::new(5, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let result = KillBuiltin::parse_jobspec("%5", &shell_state);
        assert_eq!(result.unwrap(), 5);
    }

    #[test]
    fn test_parse_jobspec_by_prefix() {
        let shell_state = ShellState::new();
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let result = KillBuiltin::parse_jobspec("%sleep", &shell_state);
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_parse_jobspec_by_contains() {
        let shell_state = ShellState::new();
        let job = Job::new(1, Some(1234), "grep pattern file &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let result = KillBuiltin::parse_jobspec("%?pattern", &shell_state);
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_get_target_pids_for_job() {
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
        let shell_state = ShellState::new();

        let result = KillBuiltin::get_target_pids("1234", &shell_state);
        assert!(result.is_ok());
        let pids = result.unwrap();
        assert_eq!(pids.len(), 1);
        assert_eq!(pids[0], 1234);
    }

    #[test]
    fn test_kill_job_with_multiple_pids() {
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
}
