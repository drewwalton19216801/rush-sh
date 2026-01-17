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
    
    // Current should become previous (2), and previous should be cleared
    // But since there's still job 2, it should find the highest ID
    assert_eq!(job_table.get_current_job(), Some(2));
    assert_eq!(job_table.get_previous_job(), None);
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
