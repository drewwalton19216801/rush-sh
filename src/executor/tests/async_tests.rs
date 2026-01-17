//! Tests for asynchronous command execution (background jobs).

use crate::parser::{Ast, ShellCommand};
use crate::state::{JobStatus, ShellState};
use crate::executor::execute;
use std::thread;
use std::time::Duration;

#[test]
fn test_simple_background_command() {
    let mut shell_state = ShellState::new();
    
    // Create a simple background command: sleep 0.1 &
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["sleep".to_string(), "0.1".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    // Execute the background command
    let exit_code = execute(ast, &mut shell_state);
    
    // Should return 0 immediately
    assert_eq!(exit_code, 0);
    
    // Should have created a job
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
    
    let job = jobs[0];
    assert_eq!(job.job_id, 1);
    assert_eq!(job.status, JobStatus::Running);
    assert!(!job.is_builtin);
    assert_eq!(job.pids.len(), 1);
    assert!(job.pgid.is_some());
}

#[test]
fn test_builtin_background_execution() {
    let mut shell_state = ShellState::new();
    
    // Create a background builtin command: pwd &
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["pwd".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    // Execute the background command
    let exit_code = execute(ast, &mut shell_state);
    
    // Should return 0 immediately
    assert_eq!(exit_code, 0);
    
    // Should have created a job
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
    
    let job = jobs[0];
    assert_eq!(job.job_id, 1);
    assert_eq!(job.status, JobStatus::Running);
    assert!(job.is_builtin);
    assert_eq!(job.pids.len(), 1);
    assert!(job.pgid.is_some());
}

#[test]
fn test_pipeline_background_execution() {
    let mut shell_state = ShellState::new();
    
    // Create a background pipeline: echo hello | cat &
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                redirections: vec![],
                compound: None,
            },
            ShellCommand {
                args: vec!["cat".to_string()],
                redirections: vec![],
                compound: None,
            },
        ])),
    };
    
    // Execute the background command
    let exit_code = execute(ast, &mut shell_state);
    
    // Should return 0 immediately
    assert_eq!(exit_code, 0);
    
    // Should have created a job
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
    
    let job = jobs[0];
    assert_eq!(job.job_id, 1);
    assert_eq!(job.status, JobStatus::Running);
    assert!(!job.is_builtin);
    // Pipeline should have 2 PIDs
    assert_eq!(job.pids.len(), 2);
    assert!(job.pgid.is_some());
}

#[test]
fn test_multiple_background_jobs() {
    let mut shell_state = ShellState::new();
    
    // Start first background job
    let ast1 = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["sleep".to_string(), "0.1".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    execute(ast1, &mut shell_state);
    
    // Start second background job
    let ast2 = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["sleep".to_string(), "0.1".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    execute(ast2, &mut shell_state);
    
    // Should have two jobs
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 2);
    
    assert_eq!(jobs[0].job_id, 1);
    assert_eq!(jobs[1].job_id, 2);
    
    // Both should be running
    assert_eq!(jobs[0].status, JobStatus::Running);
    assert_eq!(jobs[1].status, JobStatus::Running);
}

#[test]
fn test_job_table_updates() {
    let mut shell_state = ShellState::new();
    
    // Create a background command
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    execute(ast, &mut shell_state);
    
    // Verify job was added to table
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
    
    let job = jobs[0];
    assert_eq!(job.job_id, 1);
    assert_eq!(job.status, JobStatus::Running);
    
    // Verify current job tracking
    assert_eq!(job_table.get_current_job(), Some(1));
}

#[test]
fn test_process_group_creation() {
    let mut shell_state = ShellState::new();
    
    // Create a background command
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["sleep".to_string(), "0.1".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    execute(ast, &mut shell_state);
    
    // Verify process group was set
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
    
    let job = jobs[0];
    assert!(job.pgid.is_some());
    
    // For single-process jobs, pgid should equal the pid
    assert_eq!(job.pgid.unwrap(), job.pids[0]);
}

#[test]
fn test_pipeline_process_group() {
    let mut shell_state = ShellState::new();
    
    // Create a background pipeline
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["echo".to_string(), "test".to_string()],
                redirections: vec![],
                compound: None,
            },
            ShellCommand {
                args: vec!["cat".to_string()],
                redirections: vec![],
                compound: None,
            },
        ])),
    };
    
    execute(ast, &mut shell_state);
    
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
    
    let job = jobs[0];
    assert!(job.pgid.is_some());
    
    // All processes in pipeline should be in same process group
    // pgid should be the first process's PID
    assert_eq!(job.pgid.unwrap(), job.pids[0]);
}

#[test]
fn test_background_job_notification() {
    let mut shell_state = ShellState::new();
    
    // Create a background command
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    // Execute - this will print "[1] <pid>" to stdout
    // We can't easily capture stdout in this test, but we can verify
    // the job was created with the correct ID
    execute(ast, &mut shell_state);
    
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].job_id, 1);
}

#[test]
fn test_empty_background_command() {
    let mut shell_state = ShellState::new();
    
    // Create an empty background command
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    
    // Should return 0
    assert_eq!(exit_code, 0);
    
    // Should not create a job
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 0);
}

#[test]
fn test_background_command_with_variable_expansion() {
    let mut shell_state = ShellState::new();
    shell_state.set_var("DELAY", "0.1".to_string());
    
    // Create a background command with variable: sleep $DELAY &
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["sleep".to_string(), "$DELAY".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    
    assert_eq!(exit_code, 0);
    
    // Should have created a job
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
    
    // Command string should show expanded variable
    let job = jobs[0];
    assert!(job.command.contains("0.1"));
}

#[test]
fn test_background_builtin_with_output() {
    let mut shell_state = ShellState::new();
    
    // Create a background builtin that produces output: pwd &
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["pwd".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    
    assert_eq!(exit_code, 0);
    
    // Should have created a job
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
    assert!(jobs[0].is_builtin);
}

#[test]
fn test_job_id_allocation() {
    let mut shell_state = ShellState::new();
    
    // Start multiple background jobs and verify IDs are sequential
    for i in 1..=3 {
        let ast = Ast::AsyncCommand {
            command: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["true".to_string()],
                redirections: vec![],
                compound: None,
            }])),
        };
        execute(ast, &mut shell_state);
        
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), i);
        assert_eq!(jobs[i - 1].job_id, i);
    }
}

#[test]
fn test_background_command_stdin_redirection() {
    let mut shell_state = ShellState::new();
    
    // Background commands should have stdin redirected to /dev/null
    // This test verifies that a command expecting stdin doesn't hang
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["cat".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    
    assert_eq!(exit_code, 0);
    
    // Give the process a moment to start and read from stdin
    thread::sleep(Duration::from_millis(50));
    
    // Job should be created
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
}

#[test]
fn test_background_job_command_string() {
    let mut shell_state = ShellState::new();
    
    // Create a background command with multiple arguments
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["echo".to_string(), "hello".to_string(), "world".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    execute(ast, &mut shell_state);
    
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
    
    // Command string should contain all arguments
    let job = jobs[0];
    assert!(job.command.contains("echo"));
    assert!(job.command.contains("hello"));
    assert!(job.command.contains("world"));
}

#[test]
fn test_background_pipeline_command_string() {
    let mut shell_state = ShellState::new();
    
    // Create a background pipeline
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["echo".to_string(), "test".to_string()],
                redirections: vec![],
                compound: None,
            },
            ShellCommand {
                args: vec!["grep".to_string(), "test".to_string()],
                redirections: vec![],
                compound: None,
            },
        ])),
    };
    
    execute(ast, &mut shell_state);
    
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
    
    // Command string should show pipeline with |
    let job = jobs[0];
    assert!(job.command.contains("echo"));
    assert!(job.command.contains("grep"));
    assert!(job.command.contains("|"));
}
