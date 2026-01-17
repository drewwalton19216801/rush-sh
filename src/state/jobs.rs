/// Job management module for background job control
///
/// This module provides types and functionality for managing background jobs,
/// including job status tracking, process group management, and job table operations.

use std::collections::HashMap;

/// Status of a job in the job table
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    /// Job is currently running
    Running,
    /// Job has been stopped (e.g., via SIGTSTP)
    Stopped,
    /// Job has completed with the given exit code
    Done(i32),
}

impl JobStatus {
    /// Returns a human-readable string representation of the job status
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::JobStatus;
    ///
    /// assert_eq!(JobStatus::Running.to_string(), "Running");
    /// assert_eq!(JobStatus::Stopped.to_string(), "Stopped");
    /// assert_eq!(JobStatus::Done(0).to_string(), "Done");
    /// assert_eq!(JobStatus::Done(1).to_string(), "Done(1)");
    /// ```
    pub fn to_string(&self) -> String {
        match self {
            JobStatus::Running => "Running".to_string(),
            JobStatus::Stopped => "Stopped".to_string(),
            JobStatus::Done(0) => "Done".to_string(),
            JobStatus::Done(code) => format!("Done({})", code),
        }
    }
}

/// Represents a job (background process or pipeline)
#[derive(Debug, Clone)]
pub struct Job {
    /// Unique job identifier (job number)
    pub job_id: usize,
    /// Process group ID for the job
    pub pgid: Option<u32>,
    /// Command string that started the job
    pub command: String,
    /// List of process IDs in this job (for pipelines)
    pub pids: Vec<u32>,
    /// Current status of the job
    pub status: JobStatus,
    /// Exit code of the job (if completed)
    pub exit_code: Option<i32>,
    /// Whether this job is a builtin command (affects job control)
    #[allow(dead_code)]
    pub is_builtin: bool,
    /// Per-PID status tracking for pipeline jobs
    /// Maps each PID to its individual status
    pid_status: HashMap<u32, JobStatus>,
}

impl Job {
    /// Creates a new job with the given parameters
    ///
    /// # Arguments
    ///
    /// * `job_id` - Unique job identifier
    /// * `pgid` - Process group ID (None for builtins)
    /// * `command` - Command string that started the job
    /// * `pids` - List of process IDs in this job
    /// * `is_builtin` - Whether this is a builtin command
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobStatus};
    ///
    /// let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// assert_eq!(job.job_id, 1);
    /// assert_eq!(job.pgid, Some(1234));
    /// assert_eq!(job.status, JobStatus::Running);
    /// ```
    pub fn new(job_id: usize, pgid: Option<u32>, command: String, pids: Vec<u32>, is_builtin: bool) -> Self {
        // Initialize per-PID status tracking - all PIDs start as Running
        let mut pid_status = HashMap::new();
        for &pid in &pids {
            pid_status.insert(pid, JobStatus::Running);
        }
        
        Self {
            job_id,
            pgid,
            command,
            pids,
            status: JobStatus::Running,
            exit_code: None,
            is_builtin,
            pid_status,
        }
    }

    /// Updates the job status
    ///
    /// # Arguments
    ///
    /// * `status` - New status for the job
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobStatus};
    ///
    /// let mut job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// job.update_status(JobStatus::Done(0));
    /// assert_eq!(job.status, JobStatus::Done(0));
    /// assert_eq!(job.exit_code, Some(0));
    /// ```
    pub fn update_status(&mut self, status: JobStatus) {
        self.status = status.clone();
        if let JobStatus::Done(code) = status {
            self.exit_code = Some(code);
        }
    }

    /// Updates the status of a specific PID in the job
    ///
    /// For pipeline jobs, this tracks the status of each individual process.
    /// The overall job status is computed by aggregating all PID states:
    /// - If any PID is Running, the job is Running
    /// - If all PIDs are Done, the job is Done (with the exit code of the last PID)
    /// - If all PIDs are either Stopped or Done, and at least one is Stopped, the job is Stopped
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID to update
    /// * `status` - The new status for this PID
    ///
    /// # Returns
    ///
    /// `true` if the PID was found and updated, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobStatus};
    ///
    /// let mut job = Job::new(1, Some(1234), "sleep 5 | sleep 10 &".to_string(), vec![1234, 1235], false);
    ///
    /// // Update first PID to Done
    /// assert!(job.update_pid_status(1234, JobStatus::Done(0)));
    /// assert_eq!(job.status, JobStatus::Running); // Still running because second PID is running
    ///
    /// // Update second PID to Done
    /// assert!(job.update_pid_status(1235, JobStatus::Done(0)));
    /// assert_eq!(job.status, JobStatus::Done(0)); // Now the entire job is done
    /// ```
    pub fn update_pid_status(&mut self, pid: u32, status: JobStatus) -> bool {
        // Check if this PID belongs to this job
        if !self.pids.contains(&pid) {
            return false;
        }
        
        // Update the individual PID status
        self.pid_status.insert(pid, status.clone());
        
        // Recompute the overall job status based on all PID states
        self.recompute_job_status();
        
        true
    }

    /// Recomputes the overall job status based on individual PID states
    ///
    /// This is called after updating any PID status to ensure the job's
    /// overall status accurately reflects the state of all processes.
    fn recompute_job_status(&mut self) {
        if self.pids.is_empty() {
            // No PIDs means this is likely a builtin - keep current status
            return;
        }
        
        let mut all_done = true;
        let mut any_running = false;
        let mut any_stopped = false;
        let mut last_exit_code = 0;
        
        // Check the status of each PID
        for &pid in &self.pids {
            match self.pid_status.get(&pid) {
                Some(JobStatus::Running) => {
                    any_running = true;
                    all_done = false;
                }
                Some(JobStatus::Stopped) => {
                    any_stopped = true;
                    all_done = false;
                }
                Some(JobStatus::Done(code)) => {
                    // Track the exit code of the last process in the pipeline
                    // (POSIX behavior: pipeline exit code is the last command's exit code)
                    last_exit_code = *code;
                }
                None => {
                    // PID not yet tracked, assume it's still running
                    any_running = true;
                    all_done = false;
                }
            }
        }
        
        // Determine overall job status based on aggregated PID states
        if any_running {
            self.status = JobStatus::Running;
            self.exit_code = None;
        } else if all_done {
            // All PIDs have exited - job is done
            // Use the exit code from the last PID in the pipeline
            if let Some(&last_pid) = self.pids.last()
                && let Some(JobStatus::Done(code)) = self.pid_status.get(&last_pid) {
                    last_exit_code = *code;
                }
            self.status = JobStatus::Done(last_exit_code);
            self.exit_code = Some(last_exit_code);
        } else if any_stopped {
            // At least one process is stopped, none are running
            self.status = JobStatus::Stopped;
            self.exit_code = None;
        }
    }

    /// Checks if the job is still active (running or stopped)
    ///
    /// # Returns
    ///
    /// `true` if the job is running or stopped, `false` if done
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobStatus};
    ///
    /// let mut job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// assert!(job.is_active());
    ///
    /// job.update_status(JobStatus::Stopped);
    /// assert!(job.is_active());
    ///
    /// job.update_status(JobStatus::Done(0));
    /// assert!(!job.is_active());
    /// ```
    pub fn is_active(&self) -> bool {
        matches!(self.status, JobStatus::Running | JobStatus::Stopped)
    }
}

/// Job table for managing background jobs
#[derive(Debug, Clone)]
pub struct JobTable {
    /// Map of job ID to Job
    jobs: HashMap<usize, Job>,
    /// Next available job ID
    next_job_id: usize,
    /// Current job ID (most recently started or referenced)
    current_job: Option<usize>,
    /// Previous job ID (second most recently started or referenced)
    previous_job: Option<usize>,
}

impl JobTable {
    /// Creates a new empty job table
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::JobTable;
    ///
    /// let job_table = JobTable::new();
    /// assert_eq!(job_table.get_all_jobs().len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
            next_job_id: 1,
            current_job: None,
            previous_job: None,
        }
    }

    /// Allocates a new unique job ID
    ///
    /// # Returns
    ///
    /// A unique job ID that can be used to create a new job
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::JobTable;
    ///
    /// let mut job_table = JobTable::new();
    /// let id1 = job_table.allocate_job_id();
    /// let id2 = job_table.allocate_job_id();
    /// assert_eq!(id1, 1);
    /// assert_eq!(id2, 2);
    /// ```
    pub fn allocate_job_id(&mut self) -> usize {
        let id = self.next_job_id;
        self.next_job_id += 1;
        id
    }

    /// Adds a job to the job table
    ///
    /// # Arguments
    ///
    /// * `job` - The job to add
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobTable};
    ///
    /// let mut job_table = JobTable::new();
    /// let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// job_table.add_job(job);
    /// assert_eq!(job_table.get_all_jobs().len(), 1);
    /// ```
    pub fn add_job(&mut self, job: Job) {
        let job_id = job.job_id;
        
        // Update current/previous job tracking
        if let Some(current) = self.current_job {
            self.previous_job = Some(current);
        }
        self.current_job = Some(job_id);
        
        self.jobs.insert(job_id, job);
    }

    /// Removes a job from the job table
    ///
    /// # Arguments
    ///
    /// * `job_id` - The ID of the job to remove
    ///
    /// # Returns
    ///
    /// The removed job, or `None` if no job with that ID exists
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobTable};
    ///
    /// let mut job_table = JobTable::new();
    /// let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// job_table.add_job(job);
    /// 
    /// let removed = job_table.remove_job(1);
    /// assert!(removed.is_some());
    /// assert_eq!(job_table.get_all_jobs().len(), 0);
    /// ```
    pub fn remove_job(&mut self, job_id: usize) -> Option<Job> {
        let removed = self.jobs.remove(&job_id);
        
        // After removing a job, recompute current and previous from remaining jobs
        let mut remaining_ids: Vec<usize> = self.jobs.keys().copied().collect();
        remaining_ids.sort();
        
        if remaining_ids.is_empty() {
            self.current_job = None;
            self.previous_job = None;
        } else if remaining_ids.len() == 1 {
            self.current_job = Some(remaining_ids[0]);
            self.previous_job = None;
        } else {
            // Two or more jobs remain
            self.current_job = Some(remaining_ids[remaining_ids.len() - 1]);
            self.previous_job = Some(remaining_ids[remaining_ids.len() - 2]);
        }
        
        removed
    }

    /// Gets a reference to a job by its ID
    ///
    /// # Arguments
    ///
    /// * `job_id` - The ID of the job to retrieve
    ///
    /// # Returns
    ///
    /// A reference to the job, or `None` if no job with that ID exists
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobTable};
    ///
    /// let mut job_table = JobTable::new();
    /// let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// job_table.add_job(job);
    /// 
    /// let retrieved = job_table.get_job(1);
    /// assert!(retrieved.is_some());
    /// assert_eq!(retrieved.unwrap().job_id, 1);
    /// ```
    pub fn get_job(&self, job_id: usize) -> Option<&Job> {
        self.jobs.get(&job_id)
    }

    /// Gets a mutable reference to a job by its ID
    ///
    /// # Arguments
    ///
    /// * `job_id` - The ID of the job to retrieve
    ///
    /// # Returns
    ///
    /// A mutable reference to the job, or `None` if no job with that ID exists
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobTable, JobStatus};
    ///
    /// let mut job_table = JobTable::new();
    /// let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// job_table.add_job(job);
    /// 
    /// if let Some(job) = job_table.get_job_mut(1) {
    ///     job.update_status(JobStatus::Done(0));
    /// }
    /// 
    /// assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(0));
    /// ```
    pub fn get_job_mut(&mut self, job_id: usize) -> Option<&mut Job> {
        self.jobs.get_mut(&job_id)
    }

    /// Gets references to all jobs in the table
    ///
    /// # Returns
    ///
    /// A vector of references to all jobs, sorted by job ID
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobTable};
    ///
    /// let mut job_table = JobTable::new();
    /// let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
    /// job_table.add_job(job1);
    /// job_table.add_job(job2);
    /// 
    /// let jobs = job_table.get_all_jobs();
    /// assert_eq!(jobs.len(), 2);
    /// ```
    pub fn get_all_jobs(&self) -> Vec<&Job> {
        let mut jobs: Vec<&Job> = self.jobs.values().collect();
        jobs.sort_by_key(|job| job.job_id);
        jobs
    }

    /// Finds a job by process ID
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID to search for
    ///
    /// # Returns
    ///
    /// A reference to the job containing the given PID, or `None` if not found
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobTable};
    ///
    /// let mut job_table = JobTable::new();
    /// let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// job_table.add_job(job);
    /// 
    /// let found = job_table.find_job_by_pid(1234);
    /// assert!(found.is_some());
    /// assert_eq!(found.unwrap().job_id, 1);
    /// ```
    pub fn find_job_by_pid(&self, pid: u32) -> Option<&Job> {
        self.jobs.values().find(|job| job.pids.contains(&pid))
    }

    /// Finds a mutable job by process ID
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID to search for
    ///
    /// # Returns
    ///
    /// A mutable reference to the job containing the given PID, or `None` if not found
    fn find_job_by_pid_mut(&mut self, pid: u32) -> Option<&mut Job> {
        self.jobs.values_mut().find(|job| job.pids.contains(&pid))
    }

    /// Updates the status of a job by process ID
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID of the job to update
    /// * `status` - The new status for the job
    ///
    /// # Returns
    ///
    /// `true` if a job was found and updated, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobTable, JobStatus};
    ///
    /// let mut job_table = JobTable::new();
    /// let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// job_table.add_job(job);
    /// 
    /// let updated = job_table.update_job_status(1234, JobStatus::Done(0));
    /// assert!(updated);
    /// assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(0));
    /// ```
    pub fn update_job_status(&mut self, pid: u32, status: JobStatus) -> bool {
        if let Some(job) = self.find_job_by_pid_mut(pid) {
            job.update_pid_status(pid, status)
        } else {
            false
        }
    }

    /// Gets the current job ID
    ///
    /// # Returns
    ///
    /// The ID of the current job, or `None` if there are no jobs
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobTable};
    ///
    /// let mut job_table = JobTable::new();
    /// assert_eq!(job_table.get_current_job(), None);
    /// 
    /// let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// job_table.add_job(job);
    /// assert_eq!(job_table.get_current_job(), Some(1));
    /// ```
    pub fn get_current_job(&self) -> Option<usize> {
        self.current_job
    }

    /// Gets the previous job ID
    ///
    /// # Returns
    ///
    /// The ID of the previous job, or `None` if there is no previous job
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobTable};
    ///
    /// let mut job_table = JobTable::new();
    /// let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
    /// job_table.add_job(job1);
    /// job_table.add_job(job2);
    /// 
    /// assert_eq!(job_table.get_current_job(), Some(2));
    /// assert_eq!(job_table.get_previous_job(), Some(1));
    /// ```
    pub fn get_previous_job(&self) -> Option<usize> {
        self.previous_job
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
    /// - n: Direct job number (without % prefix)
    ///
    /// # Arguments
    ///
    /// * `jobspec` - The jobspec string to parse
    /// * `builtin_name` - Name of the builtin command (for error messages)
    ///
    /// # Returns
    ///
    /// The job ID on success, or an error message on failure
    ///
    /// # Examples
    ///
    /// ```
    /// use rush_sh::state::{Job, JobTable};
    ///
    /// let mut job_table = JobTable::new();
    /// let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    /// job_table.add_job(job);
    ///
    /// // Parse by number
    /// assert_eq!(job_table.parse_jobspec("%1", "fg").unwrap(), 1);
    /// assert_eq!(job_table.parse_jobspec("1", "fg").unwrap(), 1);
    ///
    /// // Parse current job
    /// assert_eq!(job_table.parse_jobspec("%", "fg").unwrap(), 1);
    /// assert_eq!(job_table.parse_jobspec("%+", "fg").unwrap(), 1);
    /// ```
    pub fn parse_jobspec(&self, jobspec: &str, builtin_name: &str) -> Result<usize, String> {
        if let Some(spec) = jobspec.strip_prefix('%') {
            // %+ or % - current job
            if spec.is_empty() || spec == "+" {
                return self
                    .get_current_job()
                    .ok_or_else(|| format!("{}: no current job", builtin_name));
            }
            
            // %- - previous job
            if spec == "-" {
                return self
                    .get_previous_job()
                    .ok_or_else(|| format!("{}: no previous job", builtin_name));
            }
            
            // %?string - job whose command contains string
            if let Some(search_str) = spec.strip_prefix('?') {
                for job in self.get_all_jobs() {
                    // Skip completed jobs when matching by command
                    if job.is_active() && job.command.contains(search_str) {
                        return Ok(job.job_id);
                    }
                }
                return Err(format!("{}: {}: no such job", builtin_name, jobspec));
            }
            
            // %string - job whose command begins with string
            // Try to parse as number first
            if let Ok(job_id) = spec.parse::<usize>() {
                return Ok(job_id);
            }
            
            // Otherwise, search for command prefix
            for job in self.get_all_jobs() {
                // Skip completed jobs when matching by command prefix
                if job.is_active() && job.command.starts_with(spec) {
                    return Ok(job.job_id);
                }
            }
            
            Err(format!("{}: {}: no such job", builtin_name, jobspec))
        } else {
            // Direct job number
            jobspec
                .parse::<usize>()
                .map_err(|_| format!("{}: {}: no such job", builtin_name, jobspec))
        }
    }
}

impl Default for JobTable {
    fn default() -> Self {
        Self::new()
    }
}
