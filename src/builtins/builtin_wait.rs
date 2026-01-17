use crate::builtins::Builtin;
use crate::parser::ShellCommand;
use crate::state::{JobStatus, ShellState};
use std::io::Write;

pub struct WaitBuiltin;

impl WaitBuiltin {
    /// Parse jobspec or PID argument
    ///
    /// Supports:
    /// - %n: Job number n
    /// - %: Current job
    /// - %+: Current job
    /// - %-: Previous job
    /// - %string: Job whose command begins with string
    /// - %?string: Job whose command contains string
    /// - n: Process ID n (direct number)
    fn parse_argument(arg: &str, shell_state: &ShellState) -> Result<WaitTarget, String> {
        if arg.starts_with('%') {
            // Jobspec - delegate to JobTable::parse_jobspec
            let job_id = shell_state
                .job_table
                .borrow()
                .parse_jobspec(arg, "wait")?;
            Ok(WaitTarget::JobId(job_id))
        } else {
            // Direct PID
            arg.parse::<u32>()
                .map(WaitTarget::Pid)
                .map_err(|_| format!("wait: {}: arguments must be process or job IDs", arg))
        }
    }

    /// Wait for a specific job by ID
    fn wait_for_job(job_id: usize, shell_state: &mut ShellState) -> Result<i32, String> {
        // Get the job
        let job = {
            let job_table = shell_state.job_table.borrow();
            job_table
                .get_job(job_id)
                .cloned()
                .ok_or_else(|| format!("wait: %{}: no such job", job_id))?
        };

        // If job is already done, return its exit code
        if let JobStatus::Done(code) = job.status {
            return Ok(code);
        }

        // Wait for all PIDs in the job
        let mut last_exit_code = 0;
        for pid in &job.pids {
            match Self::wait_for_pid(*pid, shell_state) {
                Ok(code) => last_exit_code = code,
                Err(e) => return Err(e),
            }
        }

        Ok(last_exit_code)
    }

    /// Wait for a specific PID
    fn wait_for_pid(pid: u32, shell_state: &mut ShellState) -> Result<i32, String> {
        // Check if this PID is in a job and if it's already done
        {
            let job_table = shell_state.job_table.borrow();
            if let Some(job) = job_table.find_job_by_pid(pid)
                && let JobStatus::Done(code) = job.status {
                    return Ok(code);
                }
        }

        // Use waitpid to wait for the process
        // Retry on EINTR (interrupted system call)
        let mut status: libc::c_int = 0;
        let result = loop {
            let res = unsafe { libc::waitpid(pid as libc::pid_t, &mut status, 0) };
            
            if res == -1 {
                let err = std::io::Error::last_os_error();
                // Retry on EINTR, break on other errors
                if err.raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                break res;
            }
            break res;
        };

        if result == -1 {
            let err = std::io::Error::last_os_error();
            // ECHILD means the process doesn't exist or isn't a child
            if err.raw_os_error() == Some(libc::ECHILD) {
                return Err(format!("wait: pid {}: not a child of this shell", pid));
            }
            return Err(format!("wait: pid {}: {}", pid, err));
        }

        // Extract exit code from status
        let exit_code = if libc::WIFEXITED(status) {
            libc::WEXITSTATUS(status)
        } else if libc::WIFSIGNALED(status) {
            128 + libc::WTERMSIG(status)
        } else {
            0
        };

        // Update job status if this PID is in a job
        {
            let mut job_table = shell_state.job_table.borrow_mut();
            job_table.update_job_status(pid, JobStatus::Done(exit_code));
        }

        Ok(exit_code)
    }

    /// Wait for all jobs
    fn wait_for_all_jobs(shell_state: &mut ShellState) -> i32 {
        let job_ids: Vec<usize> = {
            let job_table = shell_state.job_table.borrow();
            job_table
                .get_all_jobs()
                .iter()
                .filter(|j| j.is_active())
                .map(|j| j.job_id)
                .collect()
        };

        let mut last_exit_code = 0;
        for job_id in job_ids {
            match Self::wait_for_job(job_id, shell_state) {
                Ok(code) => last_exit_code = code,
                Err(_) => {
                    // Silently ignore errors when waiting for all jobs
                    // This matches POSIX behavior
                }
            }
        }

        last_exit_code
    }
}

/// Target for wait command
#[derive(Debug, Clone, Copy)]
enum WaitTarget {
    /// Wait for a specific job ID
    JobId(usize),
    /// Wait for a specific process ID
    Pid(u32),
}

impl Builtin for WaitBuiltin {
    fn name(&self) -> &'static str {
        "wait"
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["wait"]
    }

    fn description(&self) -> &'static str {
        "Wait for background jobs to complete"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        let args = &cmd.args;

        // If no arguments, wait for all jobs
        if args.len() == 1 {
            return Self::wait_for_all_jobs(shell_state);
        }

        // Parse all arguments first to validate them
        let mut targets = Vec::new();
        for arg in &args[1..] {
            match Self::parse_argument(arg, shell_state) {
                Ok(target) => targets.push(target),
                Err(e) => {
                    let _ = writeln!(output_writer, "{}", e);
                    return 127; // POSIX: 127 for invalid arguments
                }
            }
        }

        // Wait for each target
        let mut last_exit_code = 0;
        for target in targets {
            let result = match target {
                WaitTarget::JobId(job_id) => Self::wait_for_job(job_id, shell_state),
                WaitTarget::Pid(pid) => Self::wait_for_pid(pid, shell_state),
            };

            match result {
                Ok(code) => last_exit_code = code,
                Err(e) => {
                    let _ = writeln!(output_writer, "{}", e);
                    return 127; // POSIX: 127 for errors
                }
            }
        }

        last_exit_code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Job;
    use std::process::{Command, Stdio};

    #[test]
    fn test_wait_no_jobs() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["wait".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert_eq!(output_str, "");
    }

    #[test]
    fn test_wait_invalid_jobspec() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["wait".to_string(), "%99".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 127);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no such job"));
    }

    #[test]
    fn test_wait_no_current_job() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["wait".to_string(), "%".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 127);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no current job"));
    }

    #[test]
    fn test_wait_no_previous_job() {
        let mut shell_state = ShellState::new();
        
        // Add only one job (no previous)
        let job = Job::new(1, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["wait".to_string(), "%-".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 127);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no previous job"));
    }

    #[test]
    fn test_wait_invalid_pid() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["wait".to_string(), "abc".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 127);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("arguments must be process or job IDs"));
    }

    #[test]
    fn test_wait_completed_job() {
        let mut shell_state = ShellState::new();
        
        // Add a completed job
        let mut job = Job::new(1, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Done(0));
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["wait".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert_eq!(output_str, "");
    }

    #[test]
    fn test_wait_completed_job_with_error() {
        let mut shell_state = ShellState::new();
        
        // Add a completed job with non-zero exit code
        let mut job = Job::new(1, Some(1234), "false &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Done(1));
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["wait".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert_eq!(output_str, "");
    }

    #[test]
    fn test_wait_current_jobspec() {
        let mut shell_state = ShellState::new();
        
        // Add a completed job as current
        let mut job = Job::new(1, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Done(0));
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["wait".to_string(), "%".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_wait_previous_jobspec() {
        let mut shell_state = ShellState::new();
        
        // Add two jobs
        let mut job1 = Job::new(1, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Done(0));
        let mut job2 = Job::new(2, Some(1235), "sleep 2 &".to_string(), vec![1235], false);
        job2.update_status(JobStatus::Done(0));
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["wait".to_string(), "%-".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_wait_multiple_jobs() {
        let mut shell_state = ShellState::new();
        
        // Add multiple completed jobs
        let mut job1 = Job::new(1, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Done(0));
        let mut job2 = Job::new(2, Some(1235), "sleep 2 &".to_string(), vec![1235], false);
        job2.update_status(JobStatus::Done(1));
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["wait".to_string(), "%1".to_string(), "%2".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // Should return exit code of last job (job 2)
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_wait_real_process() {
        let mut shell_state = ShellState::new();

        // Spawn a real background process
        let child = Command::new("sleep")
            .arg("0.1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn sleep");

        let pid = child.id();

        // Add job to job table
        let job = Job::new(1, Some(pid), "sleep 0.1 &".to_string(), vec![pid], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["wait".to_string(), pid.to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert_eq!(output_str, "");

        // Verify the job status was updated
        let job_table = shell_state.job_table.borrow();
        let job = job_table.get_job(1).unwrap();
        assert_eq!(job.status, JobStatus::Done(0));
    }

    #[test]
    fn test_wait_nonexistent_pid() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        // Use a very high PID that's unlikely to exist
        let cmd = ShellCommand {
            args: vec!["wait".to_string(), "999999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 127);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("not a child of this shell") || output_str.contains("No child processes"));
    }

    #[test]
    fn test_wait_all_jobs_empty() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["wait".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_wait_all_jobs_with_completed() {
        let mut shell_state = ShellState::new();
        
        // Add multiple completed jobs
        let mut job1 = Job::new(1, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Done(0));
        let mut job2 = Job::new(2, Some(1235), "false &".to_string(), vec![1235], false);
        job2.update_status(JobStatus::Done(1));
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["wait".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        // When all jobs are already done, wait returns 0 (no jobs to wait for)
        // This matches POSIX behavior - wait only waits for running jobs
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_wait_job_with_multiple_pids() {
        let mut shell_state = ShellState::new();
        
        // Add a job with multiple PIDs (pipeline)
        let mut job = Job::new(
            1,
            Some(1234),
            "cat | grep | sort &".to_string(),
            vec![1234, 1235, 1236],
            false,
        );
        job.update_status(JobStatus::Done(0));
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["wait".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_wait_direct_job_number() {
        let mut shell_state = ShellState::new();
        
        // Add a completed job
        let mut job = Job::new(5, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Done(0));
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["wait".to_string(), "1234".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = WaitBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
    }
}
