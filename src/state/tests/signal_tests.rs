//! Tests for signal handling functionality
//!
//! This module tests SIGCHLD handling, job status updates, and job notifications.

use crate::state::{ShellState, Job, JobStatus, enqueue_signal, check_background_jobs};

#[test]
fn test_sigchld_handler_registration() {
    // Test that SIGCHLD can be enqueued
    enqueue_signal("CHLD", 17);
    
    // The signal should be in the queue (we can't easily test this without
    // exposing the queue, but we can verify it doesn't panic)
}

#[test]
fn test_job_status_update() {
    let shell_state = ShellState::new();
    
    // Create a job
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    shell_state.job_table.borrow_mut().add_job(job);
    
    // Update job status
    let updated = shell_state.job_table.borrow_mut().update_job_status(1234, JobStatus::Done(0));
    assert!(updated);
    
    // Verify status was updated
    let job = shell_state.job_table.borrow().get_job(1).unwrap().clone();
    assert_eq!(job.status, JobStatus::Done(0));
    assert_eq!(job.exit_code, Some(0));
}

#[test]
fn test_job_status_update_stopped() {
    let shell_state = ShellState::new();
    
    // Create a job
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    shell_state.job_table.borrow_mut().add_job(job);
    
    // Update job status to stopped
    let updated = shell_state.job_table.borrow_mut().update_job_status(1234, JobStatus::Stopped);
    assert!(updated);
    
    // Verify status was updated
    let job = shell_state.job_table.borrow().get_job(1).unwrap().clone();
    assert_eq!(job.status, JobStatus::Stopped);
    assert!(job.is_active());
}

#[test]
fn test_job_status_update_signaled() {
    let shell_state = ShellState::new();
    
    // Create a job
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    shell_state.job_table.borrow_mut().add_job(job);
    
    // Update job status to signaled (e.g., killed by SIGTERM = 15, so exit code = 128 + 15 = 143)
    let updated = shell_state.job_table.borrow_mut().update_job_status(1234, JobStatus::Done(143));
    assert!(updated);
    
    // Verify status was updated
    let job = shell_state.job_table.borrow().get_job(1).unwrap().clone();
    assert_eq!(job.status, JobStatus::Done(143));
    assert_eq!(job.exit_code, Some(143));
    assert!(!job.is_active());
}

#[test]
fn test_check_background_jobs_completed() {
    let mut shell_state = ShellState::new();
    
    // Create a completed job
    let mut job = Job::new(1, Some(1234), "echo test &".to_string(), vec![1234], false);
    job.update_status(JobStatus::Done(0));
    shell_state.job_table.borrow_mut().add_job(job);
    
    // Check background jobs (this should print notification and remove the job)
    check_background_jobs(&mut shell_state);
    
    // Job should be removed from table
    assert!(shell_state.job_table.borrow().get_job(1).is_none());
}

#[test]
fn test_check_background_jobs_stopped() {
    let mut shell_state = ShellState::new();
    
    // Create a stopped job
    let mut job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    job.update_status(JobStatus::Stopped);
    shell_state.job_table.borrow_mut().add_job(job);
    
    // Check background jobs (this should print notification but NOT remove the job)
    check_background_jobs(&mut shell_state);
    
    // Job should still be in table
    assert!(shell_state.job_table.borrow().get_job(1).is_some());
    let job = shell_state.job_table.borrow().get_job(1).unwrap().clone();
    assert_eq!(job.status, JobStatus::Stopped);
}

#[test]
fn test_check_background_jobs_multiple() {
    let mut shell_state = ShellState::new();
    
    // Create multiple jobs with different statuses
    let mut job1 = Job::new(1, Some(1234), "echo test1 &".to_string(), vec![1234], false);
    job1.update_status(JobStatus::Done(0));
    shell_state.job_table.borrow_mut().add_job(job1);
    
    let mut job2 = Job::new(2, Some(1235), "sleep 10 &".to_string(), vec![1235], false);
    job2.update_status(JobStatus::Stopped);
    shell_state.job_table.borrow_mut().add_job(job2);
    
    let mut job3 = Job::new(3, Some(1236), "false &".to_string(), vec![1236], false);
    job3.update_status(JobStatus::Done(1));
    shell_state.job_table.borrow_mut().add_job(job3);
    
    // Check background jobs
    check_background_jobs(&mut shell_state);
    
    // Completed jobs should be removed
    assert!(shell_state.job_table.borrow().get_job(1).is_none());
    assert!(shell_state.job_table.borrow().get_job(3).is_none());
    
    // Stopped job should remain
    assert!(shell_state.job_table.borrow().get_job(2).is_some());
}

#[test]
fn test_check_background_jobs_running() {
    let mut shell_state = ShellState::new();
    
    // Create a running job
    let job = Job::new(1, Some(1234), "sleep 10 &".to_string(), vec![1234], false);
    shell_state.job_table.borrow_mut().add_job(job);
    
    // Check background jobs (should not print anything or remove the job)
    check_background_jobs(&mut shell_state);
    
    // Job should still be in table
    assert!(shell_state.job_table.borrow().get_job(1).is_some());
    let job = shell_state.job_table.borrow().get_job(1).unwrap().clone();
    assert_eq!(job.status, JobStatus::Running);
}

#[test]
fn test_check_background_jobs_empty_table() {
    let mut shell_state = ShellState::new();
    
    // Check background jobs with empty table (should not panic)
    check_background_jobs(&mut shell_state);
    
    // Table should still be empty
    assert_eq!(shell_state.job_table.borrow().get_all_jobs().len(), 0);
}

#[test]
fn test_job_notification_format_success() {
    let mut shell_state = ShellState::new();
    
    // Create a job that completed successfully
    let mut job = Job::new(1, Some(1234), "echo hello &".to_string(), vec![1234], false);
    job.update_status(JobStatus::Done(0));
    shell_state.job_table.borrow_mut().add_job(job);
    
    // Check background jobs (should print "[1]+ Done    echo hello &")
    check_background_jobs(&mut shell_state);
    
    // Job should be removed
    assert!(shell_state.job_table.borrow().get_job(1).is_none());
}

#[test]
fn test_job_notification_format_failure() {
    let mut shell_state = ShellState::new();
    
    // Create a job that failed
    let mut job = Job::new(1, Some(1234), "false &".to_string(), vec![1234], false);
    job.update_status(JobStatus::Done(1));
    shell_state.job_table.borrow_mut().add_job(job);
    
    // Check background jobs (should print "[1]+ Done(1)    false &")
    check_background_jobs(&mut shell_state);
    
    // Job should be removed
    assert!(shell_state.job_table.borrow().get_job(1).is_none());
}

#[test]
fn test_job_notification_current_marker() {
    let mut shell_state = ShellState::new();
    
    // Create two jobs, second one is current
    let mut job1 = Job::new(1, Some(1234), "echo test1 &".to_string(), vec![1234], false);
    job1.update_status(JobStatus::Done(0));
    shell_state.job_table.borrow_mut().add_job(job1);
    
    let mut job2 = Job::new(2, Some(1235), "echo test2 &".to_string(), vec![1235], false);
    job2.update_status(JobStatus::Done(0));
    shell_state.job_table.borrow_mut().add_job(job2);
    
    // Job 2 should be current (most recently added)
    assert_eq!(shell_state.job_table.borrow().get_current_job(), Some(2));
    
    // Check background jobs
    check_background_jobs(&mut shell_state);
    
    // Both jobs should be removed
    assert!(shell_state.job_table.borrow().get_job(1).is_none());
    assert!(shell_state.job_table.borrow().get_job(2).is_none());
}

#[test]
fn test_update_job_status_nonexistent_pid() {
    let shell_state = ShellState::new();
    
    // Try to update status for a PID that doesn't exist
    let updated = shell_state.job_table.borrow_mut().update_job_status(9999, JobStatus::Done(0));
    assert!(!updated);
}

#[test]
fn test_sigchld_enqueue() {
    // Test that we can enqueue SIGCHLD multiple times
    enqueue_signal("CHLD", 17);
    enqueue_signal("CHLD", 17);
    enqueue_signal("CHLD", 17);
    
    // Should not panic or cause issues
}
