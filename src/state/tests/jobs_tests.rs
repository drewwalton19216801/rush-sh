/// Tests for job management functionality

use crate::state::{Job, JobStatus, JobTable};

#[test]
fn test_job_status_to_string() {
    assert_eq!(JobStatus::Running.to_string(), "Running");
    assert_eq!(JobStatus::Stopped.to_string(), "Stopped");
    assert_eq!(JobStatus::Done(0).to_string(), "Done");
    assert_eq!(JobStatus::Done(1).to_string(), "Done(1)");
    assert_eq!(JobStatus::Done(127).to_string(), "Done(127)");
}

#[test]
fn test_job_creation() {
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    
    assert_eq!(job.job_id, 1);
    assert_eq!(job.pgid, Some(1234));
    assert_eq!(job.command, "sleep 10 &");
    assert_eq!(job.pids, vec![1234]);
    assert_eq!(job.status, JobStatus::Running);
    assert_eq!(job.exit_code, None);
    assert!(!job.is_builtin);
}

#[test]
fn test_job_creation_builtin() {
    let job = Job::new(1, None, "sleep 10 &".to_string(), vec![], true);
    
    assert_eq!(job.job_id, 1);
    assert_eq!(job.pgid, None);
    assert!(job.is_builtin);
}

#[test]
fn test_job_update_status() {
    let mut job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    
    assert_eq!(job.status, JobStatus::Running);
    assert_eq!(job.exit_code, None);
    
    job.update_status(JobStatus::Stopped);
    assert_eq!(job.status, JobStatus::Stopped);
    assert_eq!(job.exit_code, None);
    
    job.update_status(JobStatus::Done(0));
    assert_eq!(job.status, JobStatus::Done(0));
    assert_eq!(job.exit_code, Some(0));
    
    job.update_status(JobStatus::Done(1));
    assert_eq!(job.status, JobStatus::Done(1));
    assert_eq!(job.exit_code, Some(1));
}

#[test]
fn test_job_is_active() {
    let mut job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    
    assert!(job.is_active());
    
    job.update_status(JobStatus::Stopped);
    assert!(job.is_active());
    
    job.update_status(JobStatus::Done(0));
    assert!(!job.is_active());
}

#[test]
fn test_job_table_creation() {
    let job_table = JobTable::new();
    
    assert_eq!(job_table.get_all_jobs().len(), 0);
    assert_eq!(job_table.get_current_job(), None);
    assert_eq!(job_table.get_previous_job(), None);
}

#[test]
fn test_job_table_allocate_job_id() {
    let mut job_table = JobTable::new();
    
    let id1 = job_table.allocate_job_id();
    let id2 = job_table.allocate_job_id();
    let id3 = job_table.allocate_job_id();
    
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

#[test]
fn test_job_table_add_job() {
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    job_table.add_job(job);
    
    assert_eq!(job_table.get_all_jobs().len(), 1);
    assert_eq!(job_table.get_current_job(), Some(1));
    assert_eq!(job_table.get_previous_job(), None);
}

#[test]
fn test_job_table_add_multiple_jobs() {
    let mut job_table = JobTable::new();
    
    let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
    let job3 = Job::new(3, Some(1236), "sleep 30 &".to_string(), vec![1236], false);
    
    job_table.add_job(job1);
    assert_eq!(job_table.get_current_job(), Some(1));
    assert_eq!(job_table.get_previous_job(), None);
    
    job_table.add_job(job2);
    assert_eq!(job_table.get_current_job(), Some(2));
    assert_eq!(job_table.get_previous_job(), Some(1));
    
    job_table.add_job(job3);
    assert_eq!(job_table.get_current_job(), Some(3));
    assert_eq!(job_table.get_previous_job(), Some(2));
    
    assert_eq!(job_table.get_all_jobs().len(), 3);
}

#[test]
fn test_job_table_get_job() {
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    job_table.add_job(job);
    
    let retrieved = job_table.get_job(1);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().job_id, 1);
    assert_eq!(retrieved.unwrap().command, "sleep 10 &");
    
    let not_found = job_table.get_job(999);
    assert!(not_found.is_none());
}

#[test]
fn test_job_table_get_job_mut() {
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    job_table.add_job(job);
    
    if let Some(job) = job_table.get_job_mut(1) {
        job.update_status(JobStatus::Done(0));
    }
    
    let retrieved = job_table.get_job(1);
    assert_eq!(retrieved.unwrap().status, JobStatus::Done(0));
}

#[test]
fn test_job_table_remove_job() {
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    job_table.add_job(job);
    
    assert_eq!(job_table.get_all_jobs().len(), 1);
    
    let removed = job_table.remove_job(1);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().job_id, 1);
    
    assert_eq!(job_table.get_all_jobs().len(), 0);
    assert_eq!(job_table.get_current_job(), None);
}

#[test]
fn test_job_table_remove_job_updates_current() {
    let mut job_table = JobTable::new();
    
    let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
    let job3 = Job::new(3, Some(1236), "sleep 30 &".to_string(), vec![1236], false);
    
    job_table.add_job(job1);
    job_table.add_job(job2);
    job_table.add_job(job3);
    
    // Current is 3, previous is 2
    assert_eq!(job_table.get_current_job(), Some(3));
    assert_eq!(job_table.get_previous_job(), Some(2));
    
    // Remove current job (3)
    job_table.remove_job(3);
    
    // After removing job 3, jobs 1 and 2 remain
    // Current should be 2 (highest ID), previous should be 1 (second highest)
    assert_eq!(job_table.get_current_job(), Some(2));
    assert_eq!(job_table.get_previous_job(), Some(1));
}

#[test]
fn test_job_table_remove_previous_job() {
    let mut job_table = JobTable::new();
    
    let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
    
    job_table.add_job(job1);
    job_table.add_job(job2);
    
    assert_eq!(job_table.get_current_job(), Some(2));
    assert_eq!(job_table.get_previous_job(), Some(1));
    
    // Remove previous job (1)
    job_table.remove_job(1);
    
    assert_eq!(job_table.get_current_job(), Some(2));
    assert_eq!(job_table.get_previous_job(), None);
}

#[test]
fn test_job_table_find_job_by_pid() {
    let mut job_table = JobTable::new();
    
    let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
    
    job_table.add_job(job1);
    job_table.add_job(job2);
    
    let found = job_table.find_job_by_pid(1234);
    assert!(found.is_some());
    assert_eq!(found.unwrap().job_id, 1);
    
    let found = job_table.find_job_by_pid(1235);
    assert!(found.is_some());
    assert_eq!(found.unwrap().job_id, 2);
    
    let not_found = job_table.find_job_by_pid(9999);
    assert!(not_found.is_none());
}

#[test]
fn test_job_table_find_job_by_pid_pipeline() {
    let mut job_table = JobTable::new();
    
    // Pipeline with multiple PIDs
    let job = Job::new(1, Some(1234), "cat file | grep pattern &".to_string(), vec![1234, 1235], false);
    job_table.add_job(job);
    
    let found1 = job_table.find_job_by_pid(1234);
    assert!(found1.is_some());
    assert_eq!(found1.unwrap().job_id, 1);
    
    let found2 = job_table.find_job_by_pid(1235);
    assert!(found2.is_some());
    assert_eq!(found2.unwrap().job_id, 1);
}

#[test]
fn test_job_table_update_job_status() {
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    job_table.add_job(job);
    
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    let updated = job_table.update_job_status(1234, JobStatus::Done(0));
    assert!(updated);
    
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().exit_code, Some(0));
}

#[test]
fn test_job_table_update_job_status_not_found() {
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    job_table.add_job(job);
    
    let updated = job_table.update_job_status(9999, JobStatus::Done(0));
    assert!(!updated);
}

#[test]
fn test_job_table_get_all_jobs_sorted() {
    let mut job_table = JobTable::new();
    
    // Add jobs in non-sequential order
    let job3 = Job::new(3, Some(1236), "sleep 30 &".to_string(), vec![1236], false);
    let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
    
    job_table.add_job(job3);
    job_table.add_job(job1);
    job_table.add_job(job2);
    
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 3);
    
    // Should be sorted by job ID
    assert_eq!(jobs[0].job_id, 1);
    assert_eq!(jobs[1].job_id, 2);
    assert_eq!(jobs[2].job_id, 3);
}

#[test]
fn test_job_table_multiple_status_transitions() {
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    job_table.add_job(job);
    
    // Running -> Stopped
    job_table.update_job_status(1234, JobStatus::Stopped);
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Stopped);
    assert!(job_table.get_job(1).unwrap().is_active());
    
    // Stopped -> Running
    job_table.update_job_status(1234, JobStatus::Running);
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    assert!(job_table.get_job(1).unwrap().is_active());
    
    // Running -> Done
    job_table.update_job_status(1234, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(0));
    assert!(!job_table.get_job(1).unwrap().is_active());
}

#[test]
fn test_job_table_complex_scenario() {
    let mut job_table = JobTable::new();
    
    // Add three jobs
    let job1 = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    let job2 = Job::new(2, Some(1235), "sleep 20 &".to_string(), vec![1235], false);
    let job3 = Job::new(3, Some(1236), "sleep 30 &".to_string(), vec![1236], false);
    
    job_table.add_job(job1);
    job_table.add_job(job2);
    job_table.add_job(job3);
    
    assert_eq!(job_table.get_all_jobs().len(), 3);
    assert_eq!(job_table.get_current_job(), Some(3));
    assert_eq!(job_table.get_previous_job(), Some(2));
    
    // Complete job 2
    job_table.update_job_status(1235, JobStatus::Done(0));
    assert_eq!(job_table.get_job(2).unwrap().status, JobStatus::Done(0));
    
    // Stop job 3
    job_table.update_job_status(1236, JobStatus::Stopped);
    assert_eq!(job_table.get_job(3).unwrap().status, JobStatus::Stopped);
    
    // Remove completed job 2
    job_table.remove_job(2);
    assert_eq!(job_table.get_all_jobs().len(), 2);
    
    // Current should still be 3, previous should be 1
    assert_eq!(job_table.get_current_job(), Some(3));
    
    // Add a new job
    let job4 = Job::new(4, Some(1237), "sleep 40 &".to_string(), vec![1237], false);
    job_table.add_job(job4);
    
    assert_eq!(job_table.get_current_job(), Some(4));
    assert_eq!(job_table.get_previous_job(), Some(3));
}

#[test]
fn test_job_table_empty_operations() {
    let mut job_table = JobTable::new();
    
    assert_eq!(job_table.get_all_jobs().len(), 0);
    assert!(job_table.get_job(1).is_none());
    assert!(job_table.get_job_mut(1).is_none());
    assert!(job_table.find_job_by_pid(1234).is_none());
    assert!(!job_table.update_job_status(1234, JobStatus::Done(0)));
    assert!(job_table.remove_job(1).is_none());
    assert_eq!(job_table.get_current_job(), None);
    assert_eq!(job_table.get_previous_job(), None);
}

#[test]
fn test_job_builtin_vs_external() {
    let mut job_table = JobTable::new();
    
    // External command
    let external_job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    assert!(!external_job.is_builtin);
    assert_eq!(external_job.pgid, Some(1234));
    assert!(!external_job.pids.is_empty());
    
    // Builtin command
    let builtin_job = Job::new(2, None, "sleep 10 &".to_string(), vec![], true);
    assert!(builtin_job.is_builtin);
    assert_eq!(builtin_job.pgid, None);
    assert!(builtin_job.pids.is_empty());
    
    job_table.add_job(external_job);
    job_table.add_job(builtin_job);
    
    assert_eq!(job_table.get_all_jobs().len(), 2);
}

// ============================================================================
// Pipeline Job Status Tracking Tests
// ============================================================================

#[test]
fn test_pipeline_job_single_process_backward_compatibility() {
    // Single process jobs should work exactly as before
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    job_table.add_job(job);
    
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    // Update the single PID to Done
    job_table.update_job_status(1234, JobStatus::Done(0));
    
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().exit_code, Some(0));
}

#[test]
fn test_pipeline_job_two_processes_both_exit() {
    // Test a simple two-process pipeline where both processes exit
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 5 | sleep 10 &".to_string(), vec![1234, 1235], false);
    job_table.add_job(job);
    
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    // First process exits
    job_table.update_job_status(1234, JobStatus::Done(0));
    
    // Job should still be Running because second process hasn't exited
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    assert!(job_table.get_job(1).unwrap().is_active());
    
    // Second process exits
    job_table.update_job_status(1235, JobStatus::Done(0));
    
    // Now the job should be Done
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(0));
    assert!(!job_table.get_job(1).unwrap().is_active());
    assert_eq!(job_table.get_job(1).unwrap().exit_code, Some(0));
}

#[test]
fn test_pipeline_job_first_exits_before_last() {
    // Test pipeline where first process exits before last
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "cat file | grep pattern | wc -l &".to_string(),
                       vec![1234, 1235, 1236], false);
    job_table.add_job(job);
    
    // First process exits
    job_table.update_job_status(1234, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    // Second process exits
    job_table.update_job_status(1235, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    // Third process exits
    job_table.update_job_status(1236, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(0));
}

#[test]
fn test_pipeline_job_last_exits_before_first() {
    // Test pipeline where last process exits before first
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 | sleep 5 &".to_string(), vec![1234, 1235], false);
    job_table.add_job(job);
    
    // Last process exits first
    job_table.update_job_status(1235, JobStatus::Done(0));
    
    // Job should still be Running
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    assert!(job_table.get_job(1).unwrap().is_active());
    
    // First process exits
    job_table.update_job_status(1234, JobStatus::Done(0));
    
    // Now the job should be Done
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(0));
    assert!(!job_table.get_job(1).unwrap().is_active());
}

#[test]
fn test_pipeline_job_exit_code_from_last_process() {
    // POSIX behavior: pipeline exit code is the exit code of the last command
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "false | true &".to_string(), vec![1234, 1235], false);
    job_table.add_job(job);
    
    // First process exits with error
    job_table.update_job_status(1234, JobStatus::Done(1));
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    // Second process exits successfully
    job_table.update_job_status(1235, JobStatus::Done(0));
    
    // Job exit code should be from the last process (0, not 1)
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().exit_code, Some(0));
}

#[test]
fn test_pipeline_job_exit_code_last_process_fails() {
    // Test that last process exit code is used even when it fails
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "true | false &".to_string(), vec![1234, 1235], false);
    job_table.add_job(job);
    
    // First process exits successfully
    job_table.update_job_status(1234, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    // Second process exits with error
    job_table.update_job_status(1235, JobStatus::Done(1));
    
    // Job exit code should be from the last process (1, not 0)
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(1));
    assert_eq!(job_table.get_job(1).unwrap().exit_code, Some(1));
}

#[test]
fn test_pipeline_job_with_stopped_process() {
    // Test pipeline where one process is stopped but another is still running
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 | sleep 20 &".to_string(), vec![1234, 1235], false);
    job_table.add_job(job);
    
    // First process is stopped (SIGTSTP), but second is still running
    job_table.update_job_status(1234, JobStatus::Stopped);
    
    // Job should still be Running because second process is running
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    assert!(job_table.get_job(1).unwrap().is_active());
    
    // Now stop the second process too
    job_table.update_job_status(1235, JobStatus::Stopped);
    
    // Now the job should be Stopped (all processes stopped)
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Stopped);
    assert!(job_table.get_job(1).unwrap().is_active());
}

#[test]
fn test_pipeline_job_stopped_then_resumed() {
    // Test pipeline where a process is stopped and then resumed
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 | sleep 20 &".to_string(), vec![1234, 1235], false);
    job_table.add_job(job);
    
    // First process is stopped, but second is still running
    job_table.update_job_status(1234, JobStatus::Stopped);
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    // First process is resumed
    job_table.update_job_status(1234, JobStatus::Running);
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    // Both processes exit
    job_table.update_job_status(1234, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    job_table.update_job_status(1235, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(0));
}

#[test]
fn test_pipeline_job_mixed_stopped_and_done() {
    // Test pipeline where some processes are stopped and others are done
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 | sleep 20 | sleep 30 &".to_string(),
                       vec![1234, 1235, 1236], false);
    job_table.add_job(job);
    
    // First process exits
    job_table.update_job_status(1234, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    // Second process is stopped, but third is still running
    job_table.update_job_status(1235, JobStatus::Stopped);
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    // Third process exits
    job_table.update_job_status(1236, JobStatus::Done(0));
    
    // Now job should be Stopped (first is done, second is stopped, third is done)
    // Since not all are done and at least one is stopped, job is Stopped
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Stopped);
}

#[test]
fn test_pipeline_job_update_nonexistent_pid() {
    // Test that updating a PID not in the job returns false
    let mut job_table = JobTable::new();
    
    let job = Job::new(1, Some(1234), "sleep 10 | sleep 20 &".to_string(), vec![1234, 1235], false);
    job_table.add_job(job);
    
    // Try to update a PID that doesn't belong to this job
    let updated = job_table.update_job_status(9999, JobStatus::Done(0));
    assert!(!updated);
    
    // Job status should be unchanged
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
}

#[test]
fn test_pipeline_job_large_pipeline() {
    // Test a larger pipeline with 5 processes
    let mut job_table = JobTable::new();
    
    let pids = vec![1234, 1235, 1236, 1237, 1238];
    let job = Job::new(1, Some(1234), "cmd1 | cmd2 | cmd3 | cmd4 | cmd5 &".to_string(),
                       pids.clone(), false);
    job_table.add_job(job);
    
    // Exit processes in random order
    job_table.update_job_status(1236, JobStatus::Done(0)); // 3rd
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    job_table.update_job_status(1234, JobStatus::Done(0)); // 1st
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    job_table.update_job_status(1238, JobStatus::Done(0)); // 5th (last)
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    job_table.update_job_status(1237, JobStatus::Done(0)); // 4th
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    
    job_table.update_job_status(1235, JobStatus::Done(0)); // 2nd (final)
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(0));
}

#[test]
fn test_pipeline_job_multiple_pipelines() {
    // Test multiple pipeline jobs running concurrently
    let mut job_table = JobTable::new();
    
    let job1 = Job::new(1, Some(1234), "sleep 5 | sleep 10 &".to_string(), vec![1234, 1235], false);
    let job2 = Job::new(2, Some(1236), "cat | grep | wc &".to_string(), vec![1236, 1237, 1238], false);
    
    job_table.add_job(job1);
    job_table.add_job(job2);
    
    // Complete first job
    job_table.update_job_status(1234, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Running);
    assert_eq!(job_table.get_job(2).unwrap().status, JobStatus::Running);
    
    job_table.update_job_status(1235, JobStatus::Done(0));
    assert_eq!(job_table.get_job(1).unwrap().status, JobStatus::Done(0));
    assert_eq!(job_table.get_job(2).unwrap().status, JobStatus::Running);
    
    // Complete second job
    job_table.update_job_status(1236, JobStatus::Done(0));
    job_table.update_job_status(1237, JobStatus::Done(0));
    assert_eq!(job_table.get_job(2).unwrap().status, JobStatus::Running);
    
    job_table.update_job_status(1238, JobStatus::Done(0));
    assert_eq!(job_table.get_job(2).unwrap().status, JobStatus::Done(0));
}

#[test]
fn test_pipeline_job_update_pid_status_directly() {
    // Test the update_pid_status method directly on a Job
    let mut job = Job::new(1, Some(1234), "sleep 5 | sleep 10 &".to_string(), vec![1234, 1235], false);
    
    assert_eq!(job.status, JobStatus::Running);
    
    // Update first PID
    assert!(job.update_pid_status(1234, JobStatus::Done(0)));
    assert_eq!(job.status, JobStatus::Running);
    
    // Update second PID
    assert!(job.update_pid_status(1235, JobStatus::Done(0)));
    assert_eq!(job.status, JobStatus::Done(0));
    
    // Try to update non-existent PID
    assert!(!job.update_pid_status(9999, JobStatus::Done(0)));
}

#[test]
fn test_pipeline_job_builtin_no_pids() {
    // Test that builtin jobs (no PIDs) still work correctly
    let mut job = Job::new(1, None, "sleep 10 &".to_string(), vec![], true);
    
    assert_eq!(job.status, JobStatus::Running);
    
    // Update status directly (not via PID)
    job.update_status(JobStatus::Done(0));
    assert_eq!(job.status, JobStatus::Done(0));
    assert_eq!(job.exit_code, Some(0));
}
