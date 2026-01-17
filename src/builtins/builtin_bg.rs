use crate::builtins::Builtin;
use crate::parser::ShellCommand;
use crate::state::{JobStatus, ShellState};
use std::io::Write;

pub struct BgBuiltin;

impl BgBuiltin {
    /// Resume a job in the background
    fn background_job(job_id: usize, shell_state: &mut ShellState, output_writer: &mut dyn Write) -> i32 {
        // Get the job
        let job = {
            let job_table = shell_state.job_table.borrow();
            match job_table.get_job(job_id) {
                Some(j) => j.clone(),
                None => {
                    let _ = writeln!(output_writer, "bg: %{}: no such job", job_id);
                    return 1;
                }
            }
        };

        // Check if job is already done
        if let JobStatus::Done(_) = job.status {
            let _ = writeln!(output_writer, "bg: job has terminated");
            return 1;
        }

        // Check if job is already running
        if job.status == JobStatus::Running {
            // POSIX allows this to be a no-op or a warning
            // We'll print the job info but not send SIGCONT
            let _ = writeln!(output_writer, "[{}] {}", job.job_id, job.command);
            return 0;
        }

        // Job must be stopped - send SIGCONT to resume it
        if job.status == JobStatus::Stopped {
            // Send SIGCONT to all PIDs in the job
            for pid in &job.pids {
                // SAFETY: kill is a standard POSIX function; we're sending SIGCONT to resume the process
                let result = unsafe { libc::kill(*pid as libc::pid_t, libc::SIGCONT) };
                if result == -1 {
                    let err = std::io::Error::last_os_error();
                    // ESRCH means the process doesn't exist - this is okay, continue with others
                    if err.raw_os_error() != Some(libc::ESRCH) {
                        let _ = writeln!(output_writer, "bg: failed to send SIGCONT to PID {}: {}", pid, err);
                        return 1;
                    }
                }
            }
            // Update job status to running
            if let Some(job) = shell_state.job_table.borrow_mut().get_job_mut(job_id) {
                job.update_status(JobStatus::Running);
            } else {
                // Job disappeared between verification and update - silently continue
                // This is a rare race condition that shouldn't cause failure
            }
            
            
            // Print job information: [job_id] command &
            let _ = writeln!(output_writer, "[{}] {}", job.job_id, job.command);
        }

        0
    }
}

impl Builtin for BgBuiltin {
    fn name(&self) -> &'static str {
        "bg"
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["bg"]
    }

    fn description(&self) -> &'static str {
        "Resume jobs in the background"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        let args = &cmd.args;

        // Parse jobspecs (default to current job if no arguments)
        let job_ids = if args.len() == 1 {
            // No arguments - use current job
            match shell_state.job_table.borrow().get_current_job() {
                Some(id) => vec![id],
                None => {
                    let _ = writeln!(output_writer, "bg: no current job");
                    return 1;
                }
            }
        } else {
            // Parse all jobspec arguments
            let mut ids = Vec::new();
            for arg in &args[1..] {
                match shell_state.job_table.borrow().parse_jobspec(arg, "bg") {
                    Ok(id) => ids.push(id),
                    Err(e) => {
                        let _ = writeln!(output_writer, "{}", e);
                        return 1;
                    }
                }
            }
            ids
        };

        // Verify all jobs exist before resuming any
        {
            let job_table = shell_state.job_table.borrow();
            for job_id in &job_ids {
                if job_table.get_job(*job_id).is_none() {
                    let _ = writeln!(output_writer, "bg: %{}: no such job", job_id);
                    return 1;
                }
            }
        }

        // Resume all jobs in the background
        let mut last_exit_code = 0;
        for job_id in job_ids {
            let exit_code = Self::background_job(job_id, shell_state, output_writer);
            if exit_code != 0 {
                last_exit_code = exit_code;
            }
        }

        last_exit_code
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
    fn test_bg_no_jobs() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["bg".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no current job"));
    }

    #[test]
    fn test_bg_invalid_jobspec() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add a job
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "%99".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no such job"));
    }

    #[test]
    fn test_bg_completed_job() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add a completed job
        let mut job = Job::new(1, Some(1234), "sleep 1 &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Done(0));
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("job has terminated"));
    }

    #[test]
    fn test_bg_already_running_job() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add a running job
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        // Should print job info even if already running
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("sleep 10 &"));
    }

    #[test]
    fn test_bg_stopped_job() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add a stopped job
        let mut job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Stopped);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("sleep 10 &"));
        
        // Job status should be updated to Running
        let job_table = shell_state.job_table.borrow();
        let job = job_table.get_job(1).unwrap();
        assert_eq!(job.status, JobStatus::Running);
    }

    #[test]
    fn test_bg_current_jobspec() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add a stopped job as current
        let mut job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Stopped);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "%".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("sleep 10 &"));
    }

    #[test]
    fn test_bg_previous_jobspec() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add two jobs
        let mut job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Stopped);
        let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "%-".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("sleep 10 &"));
    }

    #[test]
    fn test_bg_no_current_job() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["bg".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no current job"));
    }

    #[test]
    fn test_bg_no_previous_job() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add only one job (no previous)
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "%-".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no previous job"));
    }

    #[test]
    fn test_bg_direct_number_jobspec() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add a stopped job
        let mut job = Job::new(5, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Stopped);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "5".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("[5]"));
        assert!(output_str.contains("sleep 10 &"));
    }

    #[test]
    fn test_bg_command_prefix_match() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add jobs with different commands
        let mut job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Stopped);
        let job2 = Job::new(2, Some(1235), "grep pattern file &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "%sleep".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("sleep 10 &"));
    }

    #[test]
    fn test_bg_command_contains_match() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add jobs with different commands
        let job1 = Job::new(1, Some(1234), "cat file.txt &".to_string(), vec![1234], false);
        let mut job2 = Job::new(2, Some(1235), "grep pattern file &".to_string(), vec![1235], false);
        job2.update_status(JobStatus::Stopped);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "%?pattern".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("[2]"));
        assert!(output_str.contains("grep pattern file &"));
    }

    #[test]
    fn test_bg_multiple_jobspecs() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add multiple stopped jobs
        let mut job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Stopped);
        let mut job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
        job2.update_status(JobStatus::Stopped);
        let mut job3 = Job::new(3, Some(1236), "sleep 30 &".to_string(), vec![1236], false);
        job3.update_status(JobStatus::Stopped);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);
        shell_state.job_table.borrow_mut().add_job(job3);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "%1".to_string(), "%3".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should resume both jobs
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("sleep 10 &"));
        assert!(output_str.contains("[3]"));
        assert!(output_str.contains("sleep 30 &"));
        
        // Both jobs should be running
        let job_table = shell_state.job_table.borrow();
        assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
        assert_eq!(job_table.get_job(3).unwrap().status, JobStatus::Running);
    }

    #[test]
    fn test_bg_multiple_jobspecs_with_invalid() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add one stopped job
        let mut job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Stopped);
        shell_state.job_table.borrow_mut().add_job(job1);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "%1".to_string(), "%99".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no such job"));
        
        // First job should NOT be resumed (all-or-nothing validation)
        let job_table = shell_state.job_table.borrow();
        assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Stopped);
    }

    #[test]
    fn test_bg_pipeline_with_multiple_pids() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add a stopped pipeline with multiple PIDs
        let mut job = Job::new(
            1,
            Some(1234),
            "cat file | grep pattern | sort &".to_string(),
            vec![1234, 1235, 1236],
            false,
        );
        job.update_status(JobStatus::Stopped);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("cat file | grep pattern | sort &"));
        
        // Job should be running
        let job_table = shell_state.job_table.borrow();
        assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    }

    #[test]
    fn test_bg_no_arguments_uses_current_job() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add a stopped job as current
        let mut job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        job.update_status(JobStatus::Stopped);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("sleep 10 &"));
        
        // Job should be running
        let job_table = shell_state.job_table.borrow();
        assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    }

    #[test]
    fn test_bg_mixed_jobspec_formats() {
        let _lock = ENV_LOCK.lock().unwrap();
        let mut shell_state = ShellState::new();
        
        // Add multiple stopped jobs
        let mut job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Stopped);
        let mut job2 = Job::new(2, Some(1235), "grep pattern file &".to_string(), vec![1235], false);
        job2.update_status(JobStatus::Stopped);
        let mut job3 = Job::new(3, Some(1236), "cat file.txt &".to_string(), vec![1236], false);
        job3.update_status(JobStatus::Stopped);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);
        shell_state.job_table.borrow_mut().add_job(job3);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["bg".to_string(), "1".to_string(), "%?pattern".to_string(), "%-".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = BgBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should resume all three jobs
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("[2]"));
        // Job 2 is previous (%-) because job 3 is current
        
        // All jobs should be running
        let job_table = shell_state.job_table.borrow();
        assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
        assert_eq!(job_table.get_job(2).unwrap().status, JobStatus::Running);
    }

    #[test]
    fn test_bg_command_prefix_skips_completed_jobs() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();
        
        // Add a completed job with "sleep" command
        let mut job1 = Job::new(1, Some(1234), "sleep 5 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Done(0));
        
        // Add a stopped job with "sleep" command
        let mut job2 = Job::new(2, Some(1235), "sleep 10 &".to_string(), vec![1235], false);
        job2.update_status(JobStatus::Stopped);
        
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        // Should match job 2 (stopped), not job 1 (completed)
        let result = shell_state.job_table.borrow().parse_jobspec("%sleep", "bg");
        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn test_bg_command_contains_skips_completed_jobs() {
        let _lock = ENV_LOCK.lock().unwrap();
        let shell_state = ShellState::new();
        
        // Add a completed job containing "pattern"
        let mut job1 = Job::new(1, Some(1234), "grep pattern file1 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Done(0));
        
        // Add a stopped job containing "pattern"
        let mut job2 = Job::new(2, Some(1235), "grep pattern file2 &".to_string(), vec![1235], false);
        job2.update_status(JobStatus::Stopped);
        
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        // Should match job 2 (stopped), not job 1 (completed)
        let result = shell_state.job_table.borrow().parse_jobspec("%?pattern", "bg");
        assert_eq!(result.unwrap(), 2);
    }
}
