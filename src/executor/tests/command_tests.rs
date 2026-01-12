//! Tests for command execution (single commands, pipelines, etc.)

use crate::executor::{execute, execute_pipeline, execute_single_command};
use crate::parser::{Ast, ShellCommand};
use crate::state::ShellState;

#[test]
fn test_execute_single_command_builtin() {
    let cmd = ShellCommand {
        args: vec!["true".to_string()],
        redirections: Vec::new(),
        compound: None,
    };
    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);
}

// For external commands, test with a command that exists
#[test]
fn test_execute_single_command_external() {
    let cmd = ShellCommand {
        args: vec!["true".to_string()], // Assume true exists
        redirections: Vec::new(),
        compound: None,
    };
    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_execute_single_command_external_nonexistent() {
    let cmd = ShellCommand {
        args: vec!["nonexistent_command".to_string()],
        redirections: Vec::new(),
        compound: None,
    };
    let mut shell_state = ShellState::new();
    let exit_code = execute_single_command(&cmd, &mut shell_state);
    assert_eq!(exit_code, 1); // Command not found
}

#[test]
fn test_execute_pipeline() {
    let commands = vec![
        ShellCommand {
            args: vec!["printf".to_string(), "hello".to_string()],
            redirections: Vec::new(),
            compound: None,
        },
        ShellCommand {
            args: vec!["cat".to_string()], // cat reads from stdin
            redirections: Vec::new(),
            compound: None,
        },
    ];
    let mut shell_state = ShellState::new();
    let exit_code = execute_pipeline(&commands, &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_execute_empty_pipeline() {
    let commands = vec![];
    let mut shell_state = ShellState::new();
    let exit_code = execute(Ast::Pipeline(commands), &mut shell_state);
    assert_eq!(exit_code, 0);
}

#[test]
fn test_execute_single_command() {
    let ast = Ast::Pipeline(vec![ShellCommand {
        args: vec!["true".to_string()],
        redirections: Vec::new(),
        compound: None,
    }]);
    let mut shell_state = ShellState::new();
    let exit_code = execute(ast, &mut shell_state);
    assert_eq!(exit_code, 0);
}