//! Tests for control flow parsing (if/elif/else, case, for, while, until, functions)

use crate::lexer::Token;
use crate::parser::{parse, Ast};

#[test]
fn test_parse_if() {
    let tokens = vec![
        Token::If,
        Token::Word("true".to_string()),
        Token::Semicolon,
        Token::Then,
        Token::Word("printf".to_string()),
        Token::Word("yes".to_string()),
        Token::Semicolon,
        Token::Fi,
    ];
    let result = parse(tokens).unwrap();
    if let Ast::If {
        branches,
        else_branch,
    } = result
    {
        assert_eq!(branches.len(), 1);
        let (condition, then_branch) = &branches[0];
        if let Ast::Pipeline(cmds) = &**condition {
            assert_eq!(cmds[0].args, vec!["true"]);
        } else {
            panic!("condition not pipeline");
        }
        if let Ast::Pipeline(cmds) = &**then_branch {
            assert_eq!(cmds[0].args, vec!["printf", "yes"]);
        } else {
            panic!("then_branch not pipeline");
        }
        assert!(else_branch.is_none());
    } else {
        panic!("not if");
    }
}

#[test]
fn test_parse_if_elif() {
    let tokens = vec![
        Token::If,
        Token::Word("false".to_string()),
        Token::Semicolon,
        Token::Then,
        Token::Word("printf".to_string()),
        Token::Word("no".to_string()),
        Token::Semicolon,
        Token::Elif,
        Token::Word("true".to_string()),
        Token::Semicolon,
        Token::Then,
        Token::Word("printf".to_string()),
        Token::Word("yes".to_string()),
        Token::Semicolon,
        Token::Fi,
    ];
    let result = parse(tokens).unwrap();
    if let Ast::If {
        branches,
        else_branch,
    } = result
    {
        assert_eq!(branches.len(), 2);
        // First branch: false -> printf no
        let (condition1, then1) = &branches[0];
        if let Ast::Pipeline(cmds) = &**condition1 {
            assert_eq!(cmds[0].args, vec!["false"]);
        }
        if let Ast::Pipeline(cmds) = &**then1 {
            assert_eq!(cmds[0].args, vec!["printf", "no"]);
        }
        // Second branch: true -> printf yes
        let (condition2, then2) = &branches[1];
        if let Ast::Pipeline(cmds) = &**condition2 {
            assert_eq!(cmds[0].args, vec!["true"]);
        }
        if let Ast::Pipeline(cmds) = &**then2 {
            assert_eq!(cmds[0].args, vec!["printf", "yes"]);
        }
        assert!(else_branch.is_none());
    } else {
        panic!("not if");
    }
}