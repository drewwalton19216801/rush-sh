//! Basic parser tests for simple commands, assignments, and basic parsing

use crate::lexer::Token;
use crate::parser::{parse, Ast, ShellCommand};

#[test]
fn test_single_command() {
    let tokens = vec![Token::Word("ls".to_string())];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["ls".to_string()],
            redirections: Vec::new(),
            compound: None,
        }])
    );
}

#[test]
fn test_command_with_args() {
    let tokens = vec![
        Token::Word("ls".to_string()),
        Token::Word("-la".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["ls".to_string(), "-la".to_string()],
            redirections: Vec::new(),
            compound: None,
        }])
    );
}

#[test]
fn test_empty_tokens() {
    let tokens = vec![];
    let result = parse(tokens);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "No commands found");
}

#[test]
fn test_only_pipe() {
    let tokens = vec![Token::Pipe];
    let result = parse(tokens);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "No commands found");
}

#[test]
fn test_parse_assignment() {
    let tokens = vec![Token::Word("MY_VAR=test_value".to_string())];
    let result = parse(tokens).unwrap();
    if let Ast::Assignment { var, value } = result {
        assert_eq!(var, "MY_VAR");
        assert_eq!(value, "test_value");
    } else {
        panic!("not assignment");
    }
}

#[test]
fn test_parse_assignment_quoted() {
    let tokens = vec![Token::Word("MY_VAR=hello world".to_string())];
    let result = parse(tokens).unwrap();
    if let Ast::Assignment { var, value } = result {
        assert_eq!(var, "MY_VAR");
        assert_eq!(value, "hello world");
    } else {
        panic!("not assignment");
    }
}

#[test]
fn test_parse_assignment_invalid() {
    // Variable name starting with number should not be parsed as assignment
    let tokens = vec![Token::Word("123VAR=value".to_string())];
    let result = parse(tokens).unwrap();
    if let Ast::Pipeline(cmds) = result {
        assert_eq!(cmds[0].args, vec!["123VAR=value"]);
    } else {
        panic!("should be parsed as pipeline");
    }
}

#[test]
fn test_parse_local_assignment() {
    let tokens = vec![Token::Local, Token::Word("MY_VAR=test_value".to_string())];
    let result = parse(tokens).unwrap();
    if let Ast::LocalAssignment { var, value } = result {
        assert_eq!(var, "MY_VAR");
        assert_eq!(value, "test_value");
    } else {
        panic!("should be parsed as local assignment");
    }
}

#[test]
fn test_parse_local_assignment_separate_tokens() {
    let tokens = vec![
        Token::Local,
        Token::Word("MY_VAR".to_string()),
        Token::Word("test_value".to_string()),
    ];
    let result = parse(tokens).unwrap();
    if let Ast::LocalAssignment { var, value } = result {
        assert_eq!(var, "MY_VAR");
        assert_eq!(value, "test_value");
    } else {
        panic!("should be parsed as local assignment");
    }
}

#[test]
fn test_parse_local_assignment_invalid_var_name() {
    // Variable name starting with number should not be parsed as local assignment
    let tokens = vec![Token::Local, Token::Word("123VAR=value".to_string())];
    let result = parse(tokens);
    // Should return an error since 123VAR is not a valid variable name
    assert!(result.is_err());
}

#[test]
fn test_parse_function_definition() {
    let tokens = vec![
        Token::Word("myfunc".to_string()),
        Token::LeftParen,
        Token::RightParen,
        Token::LeftBrace,
        Token::Word("echo".to_string()),
        Token::Word("hello".to_string()),
        Token::RightBrace,
    ];
    let result = parse(tokens).unwrap();
    if let Ast::FunctionDefinition { name, body } = result {
        assert_eq!(name, "myfunc");
        // Body should be a pipeline with echo hello
        if let Ast::Pipeline(cmds) = *body {
            assert_eq!(cmds[0].args, vec!["echo", "hello"]);
        } else {
            panic!("function body should be a pipeline");
        }
    } else {
        panic!("should be parsed as function definition");
    }
}

#[test]
fn test_parse_function_definition_empty() {
    let tokens = vec![
        Token::Word("emptyfunc".to_string()),
        Token::LeftParen,
        Token::RightParen,
        Token::LeftBrace,
        Token::RightBrace,
    ];
    let result = parse(tokens).unwrap();
    if let Ast::FunctionDefinition { name, body } = result {
        assert_eq!(name, "emptyfunc");
        // Empty body should default to true command
        if let Ast::Pipeline(cmds) = *body {
            assert_eq!(cmds[0].args, vec!["true"]);
        } else {
            panic!("function body should be a pipeline");
        }
    } else {
        panic!("should be parsed as function definition");
    }
}

#[test]
fn test_parse_function_definition_legacy_format() {
    // Test backward compatibility with parentheses in the function name
    let tokens = vec![
        Token::Word("legacyfunc()".to_string()),
        Token::LeftBrace,
        Token::Word("echo".to_string()),
        Token::Word("hello".to_string()),
        Token::RightBrace,
    ];
    let result = parse(tokens).unwrap();
    if let Ast::FunctionDefinition { name, body } = result {
        assert_eq!(name, "legacyfunc");
        // Body should be a pipeline with echo hello
        if let Ast::Pipeline(cmds) = *body {
            assert_eq!(cmds[0].args, vec!["echo", "hello"]);
        } else {
            panic!("function body should be a pipeline");
        }
    } else {
        panic!("should be parsed as function definition");
    }
}