use crate::builtins::Builtin;
use crate::parser::ShellCommand;
use crate::state::{JobStatus, ShellState};
use std::io::Write;

pub struct FgBuiltin;

impl FgBuiltin {
    /// Bring a job to the foreground
    fn foreground_job(job_id: usize, shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        // Get the job
        let job = {
            let job_table = shell_state.job_table.borrow();
            match job_table.get_job(job_id) {
                Some(j) => j.clone(),
                None => {
                    let _ = writeln!(output_writer, "fg: %{}: no such job", job_id);
                    return 1;
                }
            }
        };

        // Check if job is already done
        if let JobStatus::Done(code) = job.status {
            let _ = writeln!(output_writer, "fg: job has terminated");
            // Remove the completed job from the table
            shell_state.job_table.borrow_mut().remove_job(job_id);
            return code;
        }

        // Print the command being foregrounded
        let _ = writeln!(output_writer, "{}", job.command);

        // Only perform terminal control in interactive mode
        if shell_state.interactive {
            // Get the process group ID
            let pgid = match job.pgid {
                Some(pgid) => pgid as libc::pid_t,
                None => {
                    // For builtin jobs without PGID, we can't foreground them
                    let _ = writeln!(output_writer, "fg: job does not have a process group");
                    return 1;
                }
            };

            // Get terminal file descriptor
            let terminal_fd = match shell_state.terminal_fd {
                Some(fd) => fd,
                None => {
                    let _ = writeln!(output_writer, "fg: no controlling terminal");
                    return 1;
                }
            };

            // Give the job's process group control of the terminal
            // SAFETY: tcsetpgrp is a standard POSIX function that requires a valid fd and pgid
            let result = unsafe { libc::tcsetpgrp(terminal_fd, pgid) };
            if result == -1 {
                let err = std::io::Error::last_os_error();
                let _ = writeln!(output_writer, "fg: failed to set terminal process group: {}", err);
                return 1;
            }

            // If the job was stopped, send SIGCONT to resume it
            if job.status == JobStatus::Stopped {
                for pid in &job.pids {
                    // SAFETY: kill is a standard POSIX function; we're sending SIGCONT to resume the process
                    let _ = unsafe { libc::kill(*pid as libc::pid_t, libc::SIGCONT) };
                }
                // Update job status to running
                shell_state.job_table.borrow_mut().get_job_mut(job_id).unwrap().update_status(JobStatus::Running);
            }

            // Wait for the job to complete
            let exit_code = Self::wait_for_job(&job, shell_state);

            // Restore terminal control to the shell
            // SAFETY: getpgrp and tcsetpgrp are standard POSIX functions
            let shell_pgid = unsafe { libc::getpgrp() };
            let _ = unsafe { libc::tcsetpgrp(terminal_fd, shell_pgid) };

            // Remove completed job from table or update status
            match shell_state.job_table.borrow().get_job(job_id) {
                Some(j) if matches!(j.status, JobStatus::Done(_)) => {
                    shell_state.job_table.borrow_mut().remove_job(job_id);
                }
                _ => {}
            }

            exit_code
        } else {
            // Non-interactive mode: just wait for the job
            // Send SIGCONT if stopped
            if job.status == JobStatus::Stopped {
                for pid in &job.pids {
                    // SAFETY: kill is a standard POSIX function; we're sending SIGCONT to resume the process
                    let _ = unsafe { libc::kill(*pid as libc::pid_t, libc::SIGCONT) };
                }
                shell_state.job_table.borrow_mut().get_job_mut(job_id).unwrap().update_status(JobStatus::Running);
            }

            let exit_code = Self::wait_for_job(&job, shell_state);

            // Remove completed job from table
            match shell_state.job_table.borrow().get_job(job_id) {
                Some(j) if matches!(j.status, JobStatus::Done(_)) => {
                    shell_state.job_table.borrow_mut().remove_job(job_id);
                }
                _ => {}
            }

            exit_code
        }
    }

    /// Wait for a job to complete
    fn wait_for_job(job: &crate::state::Job, shell_state: &mut ShellState) -> i32 {
        let mut last_exit_code = 0;

        // Wait for all PIDs in the job
        for pid in &job.pids {
            let mut status: libc::c_int = 0;
            
            // Use WUNTRACED to detect if the job is stopped again
            // SAFETY: waitpid is a standard POSIX function for waiting on child processes
            let result = unsafe {
                libc::waitpid(*pid as libc::pid_t, &mut status, libc::WUNTRACED)
            };

            if result == -1 {
                let err = std::io::Error::last_os_error();
                // ECHILD means the process doesn't exist or isn't a child
                if err.raw_os_error() != Some(libc::ECHILD) {
                    eprintln!("fg: waitpid failed: {}", err);
                }
                continue;
            }

            // Extract exit code or signal from status
            // SAFETY: These macros are safe to use on a valid status from waitpid
            let exit_code = if libc::WIFEXITED(status) {
                libc::WEXITSTATUS(status)
            } else if libc::WIFSIGNALED(status) {
                128 + libc::WTERMSIG(status)
            } else if libc::WIFSTOPPED(status) {
                // Job was stopped (e.g., by Ctrl-Z)
                let signal = libc::WSTOPSIG(status);
                
                // Update job status to stopped
                {
                    let mut job_table = shell_state.job_table.borrow_mut();
                    if let Some(job) = job_table.get_job_mut(job.job_id) {
                        job.update_status(JobStatus::Stopped);
                    }
                }
                
                // Return 128 + signal number for stopped jobs
                128 + signal
            } else {
                0
            };

            last_exit_code = exit_code;
        }

        // Update job status if all processes completed
        {
            let mut job_table = shell_state.job_table.borrow_mut();
            if let Some(job) = job_table.get_job_mut(job.job_id) {
                if job.status != JobStatus::Stopped {
                    job.update_status(JobStatus::Done(last_exit_code));
                }
            }
        }

        last_exit_code
    }
}

impl Builtin for FgBuiltin {
    fn name(&self) -> &'static str {
        "fg"
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["fg"]
    }

    fn description(&self) -> &'static str {
        "Move job to the foreground"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        let args = &cmd.args;

        // Parse jobspec (default to current job if no argument)
        let job_id = if args.len() == 1 {
            // No argument - use current job
            match shell_state.job_table.borrow().get_current_job() {
                Some(id) => id,
                None => {
                    let _ = writeln!(output_writer, "fg: no current job");
                    return 1;
                }
            }
        } else if args.len() == 2 {
            // Parse jobspec argument
            match shell_state.job_table.borrow().parse_jobspec(&args[1], "fg") {
                Ok(id) => id,
                Err(e) => {
                    let _ = writeln!(output_writer, "{}", e);
                    return 1;
                }
            }
        } else {
            // Too many arguments
            let _ = writeln!(output_writer, "fg: usage: fg [job_spec]");
            return 1;
        };

        // Verify the job exists
        {
            let job_table = shell_state.job_table.borrow();
            if job_table.get_job(job_id).is_none() {
                let _ = writeln!(output_writer, "fg: %{}: no such job", job_id);
                return 1;
            }
        }

        // Bring the job to the foreground
        Self::foreground_job(job_id, shell_state, output_writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Job;

    #[test]
    fn test_fg_no_jobs() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["fg".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = FgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no current job"));
    }

    #[test]
    fn test_fg_invalid_jobspec() {
        let mut shell_state = ShellState::new();
        
        // Add a job
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["fg".to_string(), "%99".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = FgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no such job"));
    }

    #[test]
    fn test_fg_completed_job() {
        let mut shell_state = ShellState::new();
        
        // Add a completed job
        let mut job = Job::new(1, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Done(0));
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["fg".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = FgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("job has terminated"));
    }

    #[test]
    fn test_fg_current_jobspec() {
        let mut shell_state = ShellState::new();
        
        // Add a completed job as current
        let mut job = Job::new(1, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Done(0));
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["fg".to_string(), "%".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = FgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("job has terminated"));
    }

    #[test]
    fn test_fg_previous_jobspec() {
        let mut shell_state = ShellState::new();
        
        // Add two jobs
        let mut job1 = Job::new(1, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Done(0));
        let job2 = Job::new(2, Some(1235), "sleep 2 &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["fg".to_string(), "%-".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = FgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("job has terminated"));
    }

    #[test]
    fn test_fg_no_current_job() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["fg".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = FgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no current job"));
    }

    #[test]
    fn test_fg_no_previous_job() {
        let mut shell_state = ShellState::new();
        
        // Add only one job (no previous)
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["fg".to_string(), "%-".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = FgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no previous job"));
    }

    #[test]
    fn test_fg_direct_number_jobspec() {
        let mut shell_state = ShellState::new();
        
        // Add a completed job
        let mut job = Job::new(5, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Done(0));
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["fg".to_string(), "5".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = FgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("job has terminated"));
    }

    #[test]
    fn test_fg_too_many_arguments() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["fg".to_string(), "%1".to_string(), "%2".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = FgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("usage"));
    }

    #[test]
    fn test_fg_command_prefix_match() {
        let shell_state = ShellState::new();
        
        // Add jobs with different commands - both running so prefix match works
        let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "grep pattern file &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        // Should match job 1 (sleep)
        let result = shell_state.job_table.borrow().parse_jobspec("%sleep", "fg");
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_fg_command_contains_match() {
        let shell_state = ShellState::new();
        
        // Add jobs with different commands - both running so contains match works
        let job1 = Job::new(1, Some(1234), "cat file.txt &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "grep pattern file &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        // Should match job 2 (contains "pattern")
        let result = shell_state.job_table.borrow().parse_jobspec("%?pattern", "fg");
        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn test_fg_builtin_job_without_pgid() {
        let mut shell_state = ShellState::new();
        shell_state.interactive = true;
        shell_state.terminal_fd = Some(libc::STDIN_FILENO);
        
        // Add a builtin job without PGID
        let job = Job::new(1, None, "sleep 10 &".to_string(), vec![], true);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["fg".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = FgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("does not have a process group"));
    }

    #[test]
    fn test_fg_exit_code_propagation() {
        let mut shell_state = ShellState::new();
        
        // Add a completed job with non-zero exit code
        let mut job = Job::new(1, Some(1234), "false &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Done(1));
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["fg".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = FgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_fg_removes_completed_job() {
        let mut shell_state = ShellState::new();
        
        // Add a completed job
        let mut job = Job::new(1, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Done(0));
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["fg".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = FgBuiltin;
        let _ = builtin.run(&cmd, &mut shell_state, &mut output);

        // Job should be removed from table
        let job_table = shell_state.job_table.borrow();
        assert!(job_table.get_job(1).is_none());
    }

    #[test]
    fn test_fg_command_prefix_skips_completed_jobs() {
        let shell_state = ShellState::new();
        
        // Add a completed job with "sleep" command
        let mut job1 = Job::new(1, Some(1234), "sleep 5 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Done(0));
        
        // Add a running job with "sleep" command
        let job2 = Job::new(2, Some(1235), "sleep 10 &".to_string(), vec![1235], false);
        
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        // Should match job 2 (running), not job 1 (completed)
        let result = shell_state.job_table.borrow().parse_jobspec("%sleep", "fg");
        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn test_fg_command_contains_skips_completed_jobs() {
        let shell_state = ShellState::new();
        
        // Add a completed job containing "pattern"
        let mut job1 = Job::new(1, Some(1234), "grep pattern file1 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Done(0));
        
        // Add a running job containing "pattern"
        let job2 = Job::new(2, Some(1235), "grep pattern file2 &".to_string(), vec![1235], false);
        
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        // Should match job 2 (running), not job 1 (completed)
        let result = shell_state.job_table.borrow().parse_jobspec("%?pattern", "fg");
        assert_eq!(result.unwrap(), 2);
    }
}
