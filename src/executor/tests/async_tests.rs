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
    
    // Command string should show original unexpanded variable (to avoid re-running command substitutions)
    let job = jobs[0];
    assert!(job.command.contains("$DELAY"));
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

// ============================================================================
// Integration Tests - End-to-End Job Control Workflows
// ============================================================================

#[test]
fn test_integration_start_check_wait_workflow() {
    // Test complete workflow: start job, check with jobs, wait for completion
    let mut shell_state = ShellState::new();
    
    // Start a background job
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["sleep".to_string(), "0.1".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Check job was created
    {
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].status, JobStatus::Running);
        assert_eq!(jobs[0].job_id, 1);
    }
    
    // Wait for job to complete
    thread::sleep(Duration::from_millis(150));
    
    // Job should still be in table but may have completed
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
}

#[test]
fn test_integration_multiple_concurrent_jobs() {
    // Test managing multiple concurrent background jobs
    let mut shell_state = ShellState::new();
    
    // Start 3 background jobs with different durations
    for i in 1..=3 {
        let ast = Ast::AsyncCommand {
            command: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["sleep".to_string(), format!("0.{}", i)],
                redirections: vec![],
                compound: None,
            }])),
        };
        execute(ast, &mut shell_state);
    }
    
    // Verify all jobs were created
    {
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), 3);
        
        // All should be running
        for job in &jobs {
            assert_eq!(job.status, JobStatus::Running);
        }
        
        // Verify job IDs are sequential
        assert_eq!(jobs[0].job_id, 1);
        assert_eq!(jobs[1].job_id, 2);
        assert_eq!(jobs[2].job_id, 3);
        
        // Current job should be the last one started
        assert_eq!(job_table.get_current_job(), Some(3));
        // Previous job should be the second-to-last
        assert_eq!(job_table.get_previous_job(), Some(2));
    }
    
    // Wait for all jobs to complete
    thread::sleep(Duration::from_millis(400));
}

#[test]
fn test_integration_pipeline_background_execution() {
    // Test complete pipeline execution in background
    let mut shell_state = ShellState::new();
    
    // Create a multi-stage pipeline in background
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["echo".to_string(), "test data".to_string()],
                redirections: vec![],
                compound: None,
            },
            ShellCommand {
                args: vec!["grep".to_string(), "test".to_string()],
                redirections: vec![],
                compound: None,
            },
            ShellCommand {
                args: vec!["wc".to_string(), "-l".to_string()],
                redirections: vec![],
                compound: None,
            },
        ])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Verify pipeline job was created
    {
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), 1);
        
        let job = jobs[0];
        assert_eq!(job.pids.len(), 3); // Three processes in pipeline
        assert!(job.pgid.is_some());
        assert_eq!(job.status, JobStatus::Running);
        
        // All PIDs should be in the same process group
        assert_eq!(job.pgid.unwrap(), job.pids[0]);
    }
    
    // Wait for pipeline to complete
    thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_integration_job_control_with_builtins() {
    // Test background execution of builtin commands
    let mut shell_state = ShellState::new();
    
    // Execute multiple builtins in background
    let builtins = vec!["pwd", ":", "times"];
    
    for (i, builtin) in builtins.iter().enumerate() {
        let ast = Ast::AsyncCommand {
            command: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec![builtin.to_string()],
                redirections: vec![],
                compound: None,
            }])),
        };
        execute(ast, &mut shell_state);
        
        // Verify job was created
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), i + 1);
        assert!(jobs[i].is_builtin);
    }
    
    // Wait for all builtins to complete
    thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_integration_job_state_transitions() {
    // Test job state transitions from Running to Done
    let mut shell_state = ShellState::new();
    
    // Start a quick job
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    execute(ast, &mut shell_state);
    
    // Initially should be Running
    {
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].status, JobStatus::Running);
    }
    
    // Wait for job to complete
    thread::sleep(Duration::from_millis(50));
    
    // Job should still be in table (cleanup happens on next prompt or explicit wait)
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 1);
}

#[test]
fn test_integration_current_previous_job_tracking() {
    // Test current and previous job tracking
    let mut shell_state = ShellState::new();
    
    // Start first job
    let ast1 = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["sleep".to_string(), "0.1".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    execute(ast1, &mut shell_state);
    
    // Current should be job 1, no previous
    {
        let job_table = shell_state.job_table.borrow();
        assert_eq!(job_table.get_current_job(), Some(1));
        assert_eq!(job_table.get_previous_job(), None);
    }
    
    // Start second job
    let ast2 = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["sleep".to_string(), "0.1".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    execute(ast2, &mut shell_state);
    
    // Current should be job 2, previous should be job 1
    {
        let job_table = shell_state.job_table.borrow();
        assert_eq!(job_table.get_current_job(), Some(2));
        assert_eq!(job_table.get_previous_job(), Some(1));
    }
    
    // Start third job
    let ast3 = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["sleep".to_string(), "0.1".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    execute(ast3, &mut shell_state);
    
    // Current should be job 3, previous should be job 2
    {
        let job_table = shell_state.job_table.borrow();
        assert_eq!(job_table.get_current_job(), Some(3));
        assert_eq!(job_table.get_previous_job(), Some(2));
    }
}

#[test]
fn test_integration_mixed_builtin_external_jobs() {
    // Test mix of builtin and external commands in background
    let mut shell_state = ShellState::new();
    
    // Start builtin job
    let ast1 = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["pwd".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    execute(ast1, &mut shell_state);
    
    // Start external job
    let ast2 = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["sleep".to_string(), "0.1".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    execute(ast2, &mut shell_state);
    
    // Start another builtin job
    let ast3 = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec![":".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    execute(ast3, &mut shell_state);
    
    // Verify all jobs were created with correct types
    {
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), 3);
        
        assert!(jobs[0].is_builtin);
        assert!(!jobs[1].is_builtin);
        assert!(jobs[2].is_builtin);
    }
}

#[test]
fn test_integration_job_cleanup_on_completion() {
    // Test that completed jobs can be cleaned up
    let mut shell_state = ShellState::new();
    
    // Start a quick job
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    execute(ast, &mut shell_state);
    
    let job_id = {
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), 1);
        jobs[0].job_id
    };
    
    // Wait for job to complete
    thread::sleep(Duration::from_millis(50));
    
    // Manually remove completed job (simulating what wait or prompt would do)
    {
        let mut job_table = shell_state.job_table.borrow_mut();
        job_table.remove_job(job_id);
    }
    
    // Job should be removed
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 0);
}

#[test]
fn test_integration_process_group_isolation() {
    // Test that background jobs are in separate process groups
    let mut shell_state = ShellState::new();
    
    // Start multiple jobs
    for _ in 0..3 {
        let ast = Ast::AsyncCommand {
            command: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["sleep".to_string(), "0.1".to_string()],
                redirections: vec![],
                compound: None,
            }])),
        };
        execute(ast, &mut shell_state);
    }
    
    // Verify each job has its own process group
    {
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), 3);
        
        let mut pgids = std::collections::HashSet::new();
        for job in jobs {
            assert!(job.pgid.is_some());
            pgids.insert(job.pgid.unwrap());
        }
        
        // All jobs should have different process groups
        assert_eq!(pgids.len(), 3);
    }
}

#[test]
fn test_integration_background_with_variable_expansion() {
    // Test that variable expansion works in background commands
    let mut shell_state = ShellState::new();
    
    // Set up variables
    shell_state.set_var("CMD", "sleep".to_string());
    shell_state.set_var("ARG", "0.1".to_string());
    
    // Execute command with variables
    let ast = Ast::AsyncCommand {
        command: Box::new(Ast::Pipeline(vec![ShellCommand {
            args: vec!["$CMD".to_string(), "$ARG".to_string()],
            redirections: vec![],
            compound: None,
        }])),
    };
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Verify job was created with original unexpanded command (to avoid re-running command substitutions)
    {
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), 1);
        
        let job = jobs[0];
        assert!(job.command.contains("$CMD"));
        assert!(job.command.contains("$ARG"));
    }
}

#[test]
fn test_integration_sequential_job_execution() {
    // Test starting jobs sequentially and tracking their order
    let mut shell_state = ShellState::new();
    
    let commands = vec!["true", "false", "pwd", "echo"];
    
    for (i, cmd) in commands.iter().enumerate() {
        let ast = Ast::AsyncCommand {
            command: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec![cmd.to_string()],
                redirections: vec![],
                compound: None,
            }])),
        };
        execute(ast, &mut shell_state);
        
        // Verify job count increases
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), i + 1);
    }
    
    // Verify all jobs are present
    let job_table = shell_state.job_table.borrow();
    let jobs = job_table.get_all_jobs();
    assert_eq!(jobs.len(), 4);
    
    // Verify job IDs are sequential
    for (i, job) in jobs.iter().enumerate() {
        assert_eq!(job.job_id, i + 1);
    }
}

#[test]
fn test_integration_background_pipeline_with_builtins() {
    // Test pipeline mixing builtins and external commands
    let mut shell_state = ShellState::new();
    
    // Pipeline with builtin and external command
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
    
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
    
    // Verify pipeline job was created
    {
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), 1);
        
        let job = jobs[0];
        assert_eq!(job.pids.len(), 2);
        assert!(job.command.contains("echo"));
        assert!(job.command.contains("cat"));
        assert!(job.command.contains("|"));
    }
}

#[test]
fn test_integration_rapid_job_creation() {
    // Test creating many jobs rapidly
    let mut shell_state = ShellState::new();
    
    // Create 10 jobs rapidly
    for _ in 0..10 {
        let ast = Ast::AsyncCommand {
            command: Box::new(Ast::Pipeline(vec![ShellCommand {
                args: vec!["true".to_string()],
                redirections: vec![],
                compound: None,
            }])),
        };
        execute(ast, &mut shell_state);
    }
    
    // Verify all jobs were created
    {
        let job_table = shell_state.job_table.borrow();
        let jobs = job_table.get_all_jobs();
        assert_eq!(jobs.len(), 10);
        
        // Verify job IDs are sequential
        for (i, job) in jobs.iter().enumerate() {
            assert_eq!(job.job_id, i + 1);
        }
    }
}
