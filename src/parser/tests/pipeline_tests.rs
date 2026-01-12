//! Tests for pipeline parsing

use crate::lexer::Token;
use crate::parser::{parse, Ast, Redirection, ShellCommand};

#[test]
fn test_pipeline() {
    let tokens = vec![
        Token::Word("ls".to_string()),
        Token::Pipe,
        Token::Word("grep".to_string()),
        Token::Word("txt".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["ls".to_string()],
                redirections: Vec::new(),
                compound: None,
            },
            ShellCommand {
                args: vec!["grep".to_string(), "txt".to_string()],
                redirections: Vec::new(),
                compound: None,
            }
        ])
    );
}

#[test]
fn test_complex_pipeline_with_redirections() {
    let tokens = vec![
        Token::Word("cat".to_string()),
        Token::RedirIn,
        Token::Word("input.txt".to_string()),
        Token::Pipe,
        Token::Word("grep".to_string()),
        Token::Word("pattern".to_string()),
        Token::Pipe,
        Token::Word("sort".to_string()),
        Token::RedirOut,
        Token::Word("output.txt".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["cat".to_string()],
                compound: None,
                redirections: vec![Redirection::Input("input.txt".to_string())],
            },
            ShellCommand {
                args: vec!["grep".to_string(), "pattern".to_string()],
                compound: None,
                redirections: Vec::new(),
            },
            ShellCommand {
                args: vec!["sort".to_string()],
                redirections: vec![Redirection::Output("output.txt".to_string())],
                compound: None,
            }
        ])
    );
}