use crate::builtins::Builtin;
use crate::parser::ShellCommand;
use crate::state::ShellState;
use std::io::Write;

pub struct JobsBuiltin;

impl JobsBuiltin {
    /// Parse jobspec argument to job ID
    ///
    /// Supports:
    /// - %n: Job number n
    /// - %: Current job
    /// - %+: Current job
    /// - %-: Previous job
    /// - %string: Job whose command begins with string
    /// - %?string: Job whose command contains string
    /// - n: Job number n (direct number)
    fn parse_jobspec(jobspec: &str, shell_state: &ShellState) -> Result<usize, String> {
        if jobspec.starts_with('%') {
            let spec = &jobspec[1..];
            
            // %+ or % - current job
            if spec.is_empty() || spec == "+" {
                return shell_state
                    .job_table
                    .borrow()
                    .get_current_job()
                    .ok_or_else(|| "jobs: no current job".to_string());
            }
            
            // %- - previous job
            if spec == "-" {
                return shell_state
                    .job_table
                    .borrow()
                    .get_previous_job()
                    .ok_or_else(|| "jobs: no previous job".to_string());
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
                return Err(format!("jobs: {}: no such job", jobspec));
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
            
            Err(format!("jobs: {}: no such job", jobspec))
        } else {
            // Direct job number
            jobspec
                .parse::<usize>()
                .map_err(|_| format!("jobs: {}: no such job", jobspec))
        }
    }

    /// Format job marker (+ for current, - for previous, space otherwise)
    fn get_job_marker(job_id: usize, shell_state: &ShellState) -> char {
        let job_table = shell_state.job_table.borrow();
        if Some(job_id) == job_table.get_current_job() {
            '+'
        } else if Some(job_id) == job_table.get_previous_job() {
            '-'
        } else {
            ' '
        }
    }

    /// Display jobs in default format: [job_id]marker status    command
    fn display_default(shell_state: &ShellState, output: &mut dyn Write, job_ids: Option<Vec<usize>>) -> i32 {
        let job_table = shell_state.job_table.borrow();
        
        let jobs = if let Some(ids) = job_ids {
            // Display specific jobs
            let mut specific_jobs = Vec::new();
            for id in ids {
                if let Some(job) = job_table.get_job(id) {
                    specific_jobs.push(job.clone());
                } else {
                    drop(job_table);
                    let _ = writeln!(output, "jobs: {}: no such job", id);
                    return 1;
                }
            }
            specific_jobs
        } else {
            // Display all jobs
            job_table.get_all_jobs().iter().map(|j| (*j).clone()).collect()
        };
        
        drop(job_table);

        for job in jobs {
            let marker = Self::get_job_marker(job.job_id, shell_state);
            let status = job.status.to_string();
            let _ = writeln!(
                output,
                "[{}]{} {:<10} {}",
                job.job_id,
                marker,
                status,
                job.command
            );
        }

        0
    }

    /// Display jobs with PIDs: [job_id]marker status pid command
    fn display_with_pids(shell_state: &ShellState, output: &mut dyn Write, job_ids: Option<Vec<usize>>) -> i32 {
        let job_table = shell_state.job_table.borrow();
        
        let jobs = if let Some(ids) = job_ids {
            // Display specific jobs
            let mut specific_jobs = Vec::new();
            for id in ids {
                if let Some(job) = job_table.get_job(id) {
                    specific_jobs.push(job.clone());
                } else {
                    drop(job_table);
                    let _ = writeln!(output, "jobs: {}: no such job", id);
                    return 1;
                }
            }
            specific_jobs
        } else {
            // Display all jobs
            job_table.get_all_jobs().iter().map(|j| (*j).clone()).collect()
        };
        
        drop(job_table);

        for job in jobs {
            let marker = Self::get_job_marker(job.job_id, shell_state);
            let status = job.status.to_string();
            
            // Display with first PID (or all PIDs for pipelines)
            if job.pids.is_empty() {
                // No PIDs (builtin job)
                let _ = writeln!(
                    output,
                    "[{}]{} {:<10} {}",
                    job.job_id,
                    marker,
                    status,
                    job.command
                );
            } else if job.pids.len() == 1 {
                // Single process
                let _ = writeln!(
                    output,
                    "[{}]{} {:<10} {} {}",
                    job.job_id,
                    marker,
                    status,
                    job.pids[0],
                    job.command
                );
            } else {
                // Pipeline - show all PIDs
                let pids_str: String = job.pids
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                let _ = writeln!(
                    output,
                    "[{}]{} {:<10} {} {}",
                    job.job_id,
                    marker,
                    status,
                    pids_str,
                    job.command
                );
            }
        }

        0
    }

    /// Display only PIDs (one per line)
    fn display_pids_only(shell_state: &ShellState, output: &mut dyn Write, job_ids: Option<Vec<usize>>) -> i32 {
        let job_table = shell_state.job_table.borrow();
        
        let jobs = if let Some(ids) = job_ids {
            // Display specific jobs
            let mut specific_jobs = Vec::new();
            for id in ids {
                if let Some(job) = job_table.get_job(id) {
                    specific_jobs.push(job.clone());
                } else {
                    drop(job_table);
                    let _ = writeln!(output, "jobs: {}: no such job", id);
                    return 1;
                }
            }
            specific_jobs
        } else {
            // Display all jobs
            job_table.get_all_jobs().iter().map(|j| (*j).clone()).collect()
        };
        
        drop(job_table);

        for job in jobs {
            // Display all PIDs for this job, one per line
            for pid in &job.pids {
                let _ = writeln!(output, "{}", pid);
            }
        }

        0
    }
}

impl Builtin for JobsBuiltin {
    fn name(&self) -> &'static str {
        "jobs"
    }

    fn names(&self) -> Vec<&'static str> {
        vec!["jobs"]
    }

    fn description(&self) -> &'static str {
        "Display status of jobs in the current shell"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        let args = &cmd.args;

        // Parse options
        let mut show_pids = false;
        let mut pids_only = false;
        let mut jobspecs = Vec::new();
        let mut i = 1;

        while i < args.len() {
            let arg = &args[i];
            
            if arg.starts_with('-') && arg.len() > 1 {
                // Parse options
                for ch in arg[1..].chars() {
                    match ch {
                        'l' => show_pids = true,
                        'p' => pids_only = true,
                        _ => {
                            let _ = writeln!(output_writer, "jobs: -{}: invalid option", ch);
                            let _ = writeln!(output_writer, "jobs: usage: jobs [-lp] [jobspec ...]");
                            return 1;
                        }
                    }
                }
            } else {
                // Parse jobspec
                match Self::parse_jobspec(arg, shell_state) {
                    Ok(job_id) => jobspecs.push(job_id),
                    Err(e) => {
                        let _ = writeln!(output_writer, "{}", e);
                        return 1;
                    }
                }
            }
            
            i += 1;
        }

        // Determine which jobs to display
        let job_ids = if jobspecs.is_empty() {
            None
        } else {
            Some(jobspecs)
        };

        // Display jobs based on options
        if pids_only {
            Self::display_pids_only(shell_state, output_writer, job_ids)
        } else if show_pids {
            Self::display_with_pids(shell_state, output_writer, job_ids)
        } else {
            Self::display_default(shell_state, output_writer, job_ids)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Job, JobStatus};

    #[test]
    fn test_jobs_empty_table() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["jobs".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert_eq!(output_str, "");
    }

    #[test]
    fn test_jobs_default_display() {
        let mut shell_state = ShellState::new();
        
        // Add some jobs
        let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Job 1 should have - marker (previous)
        assert!(output_str.contains("[1]-"));
        assert!(output_str.contains("Running"));
        assert!(output_str.contains("sleep 10 &"));
        
        // Job 2 should have + marker (current)
        assert!(output_str.contains("[2]+"));
        assert!(output_str.contains("sleep 20 &"));
    }

    #[test]
    fn test_jobs_with_pids() {
        let mut shell_state = ShellState::new();
        
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "-l".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        assert!(output_str.contains("[1]+"));
        assert!(output_str.contains("Running"));
        assert!(output_str.contains("1234"));
        assert!(output_str.contains("sleep 10 &"));
    }

    #[test]
    fn test_jobs_pids_only() {
        let mut shell_state = ShellState::new();
        
        let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "-p".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should only contain PIDs, one per line
        assert_eq!(output_str, "1234\n1235\n");
    }

    #[test]
    fn test_jobs_pipeline_pids() {
        let mut shell_state = ShellState::new();
        
        // Pipeline with multiple PIDs
        let job = Job::new(
            1,
            Some(1234),
            "cat file | grep pattern | sort &".to_string(),
            vec![1234, 1235, 1236],
            false,
        );
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "-l".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should show all PIDs
        assert!(output_str.contains("1234 1235 1236"));
    }

    #[test]
    fn test_jobs_pipeline_pids_only() {
        let mut shell_state = ShellState::new();
        
        // Pipeline with multiple PIDs
        let job = Job::new(
            1,
            Some(1234),
            "cat file | grep pattern | sort &".to_string(),
            vec![1234, 1235, 1236],
            false,
        );
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "-p".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should show all PIDs, one per line
        assert_eq!(output_str, "1234\n1235\n1236\n");
    }

    #[test]
    fn test_jobs_different_statuses() {
        let mut shell_state = ShellState::new();
        
        let mut job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Running);
        
        let mut job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
        job2.update_status(JobStatus::Stopped);
        
        let mut job3 = Job::new(3, Some(1236), "sleep 30 &".to_string(), vec![1236], false);
        job3.update_status(JobStatus::Done(0));
        
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);
        shell_state.job_table.borrow_mut().add_job(job3);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        assert!(output_str.contains("Running"));
        assert!(output_str.contains("Stopped"));
        assert!(output_str.contains("Done"));
    }

    #[test]
    fn test_jobs_specific_jobspec() {
        let mut shell_state = ShellState::new();
        
        let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "%1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should only show job 1
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("sleep 10 &"));
        assert!(!output_str.contains("sleep 20 &"));
    }

    #[test]
    fn test_jobs_current_jobspec() {
        let mut shell_state = ShellState::new();
        
        let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "%".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should show current job (job 2)
        assert!(output_str.contains("[2]"));
        assert!(output_str.contains("sleep 20 &"));
        assert!(!output_str.contains("sleep 10 &"));
    }

    #[test]
    fn test_jobs_previous_jobspec() {
        let mut shell_state = ShellState::new();
        
        let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "%-".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should show previous job (job 1)
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("sleep 10 &"));
        assert!(!output_str.contains("sleep 20 &"));
    }

    #[test]
    fn test_jobs_invalid_jobspec() {
        let mut shell_state = ShellState::new();
        
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "%99".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no such job"));
    }

    #[test]
    fn test_jobs_invalid_option() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "-x".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("invalid option"));
    }

    #[test]
    fn test_jobs_multiple_options() {
        let mut shell_state = ShellState::new();
        
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "-lp".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // -p takes precedence, should only show PIDs
        assert_eq!(output_str, "1234\n");
    }

    #[test]
    fn test_jobs_no_current_job() {
        let mut shell_state = ShellState::new();
        let mut output = Vec::new();

        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "%".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no current job"));
    }

    #[test]
    fn test_jobs_no_previous_job() {
        let mut shell_state = ShellState::new();
        
        // Add only one job (no previous)
        let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "%-".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("no previous job"));
    }

    #[test]
    fn test_jobs_builtin_without_pids() {
        let mut shell_state = ShellState::new();
        
        // Builtin job with no PIDs
        let job = Job::new(1, None, "sleep 10 &".to_string(), vec![], true);
        shell_state.job_table.borrow_mut().add_job(job);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "-l".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should show job without PID
        assert!(output_str.contains("[1]+"));
        assert!(output_str.contains("Running"));
        assert!(output_str.contains("sleep 10 &"));
    }

    #[test]
    fn test_jobs_direct_number_jobspec() {
        let mut shell_state = ShellState::new();
        
        let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should only show job 1
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("sleep 10 &"));
        assert!(!output_str.contains("sleep 20 &"));
    }

    #[test]
    fn test_jobs_multiple_jobspecs() {
        let mut shell_state = ShellState::new();
        
        let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
        let job3 = Job::new(3, Some(1236), "sleep 30 &".to_string(), vec![1236], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);
        shell_state.job_table.borrow_mut().add_job(job3);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "1".to_string(), "3".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should show jobs 1 and 3, but not 2
        assert!(output_str.contains("sleep 10 &"));
        assert!(output_str.contains("sleep 30 &"));
        assert!(!output_str.contains("sleep 20 &"));
    }

    #[test]
    fn test_jobs_command_prefix_match() {
        let shell_state = ShellState::new();
        
        // Add jobs with different commands - both running so prefix match works
        let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "grep pattern file &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        // Should match job 1 (sleep)
        let result = JobsBuiltin::parse_jobspec("%sleep", &shell_state);
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_jobs_command_contains_match() {
        let shell_state = ShellState::new();
        
        // Add jobs with different commands - both running so contains match works
        let job1 = Job::new(1, Some(1234), "cat file.txt &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "grep pattern file &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        // Should match job 2 (contains "pattern")
        let result = JobsBuiltin::parse_jobspec("%?pattern", &shell_state);
        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn test_jobs_command_prefix_skips_completed_jobs() {
        let shell_state = ShellState::new();
        
        // Add a completed job with "sleep" command
        let mut job1 = Job::new(1, Some(1234), "sleep 5 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Done(0));
        
        // Add a running job with "sleep" command
        let job2 = Job::new(2, Some(1235), "sleep 10 &".to_string(), vec![1235], false);
        
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        // Should match job 2 (running), not job 1 (completed)
        let result = JobsBuiltin::parse_jobspec("%sleep", &shell_state);
        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn test_jobs_command_contains_skips_completed_jobs() {
        let shell_state = ShellState::new();
        
        // Add a completed job containing "pattern"
        let mut job1 = Job::new(1, Some(1234), "grep pattern file1 &".to_string(), vec![1234], false);
        job1.update_status(JobStatus::Done(0));
        
        // Add a running job containing "pattern"
        let job2 = Job::new(2, Some(1235), "grep pattern file2 &".to_string(), vec![1235], false);
        
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        // Should match job 2 (running), not job 1 (completed)
        let result = JobsBuiltin::parse_jobspec("%?pattern", &shell_state);
        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn test_jobs_command_prefix_with_jobspec() {
        let mut shell_state = ShellState::new();
        
        // Add jobs with different commands
        let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "grep pattern file &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "%sleep".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should only show job 1
        assert!(output_str.contains("[1]"));
        assert!(output_str.contains("sleep 10 &"));
        assert!(!output_str.contains("grep pattern file &"));
    }

    #[test]
    fn test_jobs_command_contains_with_jobspec() {
        let mut shell_state = ShellState::new();
        
        // Add jobs with different commands
        let job1 = Job::new(1, Some(1234), "cat file.txt &".to_string(), vec![1234], false);
        let job2 = Job::new(2, Some(1235), "grep pattern file &".to_string(), vec![1235], false);
        shell_state.job_table.borrow_mut().add_job(job1);
        shell_state.job_table.borrow_mut().add_job(job2);

        let mut output = Vec::new();
        let cmd = ShellCommand {
            args: vec!["jobs".to_string(), "%?pattern".to_string()],
            redirections: Vec::new(),
            compound: None,
        };

        let builtin = JobsBuiltin;
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        
        // Should only show job 2
        assert!(output_str.contains("[2]"));
        assert!(output_str.contains("grep pattern file &"));
        assert!(!output_str.contains("cat file.txt &"));
    }
}
