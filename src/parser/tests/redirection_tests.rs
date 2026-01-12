//! Tests for I/O redirection parsing

use crate::lexer::Token;
use crate::parser::{parse, Ast, Redirection, ShellCommand};

#[test]
fn test_input_redirection() {
    let tokens = vec![
        Token::Word("cat".to_string()),
        Token::RedirIn,
        Token::Word("input.txt".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["cat".to_string()],
            redirections: vec![Redirection::Input("input.txt".to_string())],
            compound: None,
        }])
    );
}

#[test]
fn test_output_redirection() {
    let tokens = vec![
        Token::Word("printf".to_string()),
        Token::Word("hello".to_string()),
        Token::RedirOut,
        Token::Word("output.txt".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["printf".to_string(), "hello".to_string()],
            compound: None,
            redirections: vec![Redirection::Output("output.txt".to_string())],
        }])
    );
}

#[test]
fn test_append_redirection() {
    let tokens = vec![
        Token::Word("printf".to_string()),
        Token::Word("hello".to_string()),
        Token::RedirAppend,
        Token::Word("output.txt".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["printf".to_string(), "hello".to_string()],
            compound: None,
            redirections: vec![Redirection::Append("output.txt".to_string())],
        }])
    );
}

#[test]
fn test_redirection_without_file() {
    // Parser doesn't check for missing file, just skips if no token after
    let tokens = vec![Token::Word("cat".to_string()), Token::RedirIn];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["cat".to_string()],
            compound: None,
            redirections: Vec::new(),
        }])
    );
}

#[test]
fn test_multiple_redirections() {
    let tokens = vec![
        Token::Word("cat".to_string()),
        Token::RedirIn,
        Token::Word("file1.txt".to_string()),
        Token::RedirOut,
        Token::Word("file2.txt".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["cat".to_string()],
            redirections: vec![
                Redirection::Input("file1.txt".to_string()),
                Redirection::Output("file2.txt".to_string()),
            ],
            compound: None,
        }])
    );
}

#[test]
fn test_parse_here_document_redirection() {
    let tokens = vec![
        Token::Word("cat".to_string()),
        Token::RedirHereDoc("EOF".to_string(), false),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["cat".to_string()],
            redirections: vec![Redirection::HereDoc("EOF".to_string(), "false".to_string())],
            compound: None,
        }])
    );
}

#[test]
fn test_parse_here_string_redirection() {
    let tokens = vec![
        Token::Word("grep".to_string()),
        Token::RedirHereString("pattern".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["grep".to_string()],
            compound: None,
            redirections: vec![Redirection::HereString("pattern".to_string())],
        }])
    );
}

#[test]
fn test_parse_mixed_redirections() {
    let tokens = vec![
        Token::Word("cat".to_string()),
        Token::RedirIn,
        Token::Word("file.txt".to_string()),
        Token::RedirHereString("fallback".to_string()),
        Token::RedirOut,
        Token::Word("output.txt".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["cat".to_string()],
            compound: None,
            redirections: vec![
                Redirection::Input("file.txt".to_string()),
                Redirection::HereString("fallback".to_string()),
                Redirection::Output("output.txt".to_string()),
            ],
        }])
    );
}

// ===== File Descriptor Redirection Tests =====

#[test]
fn test_parse_fd_input_redirection() {
    let tokens = vec![
        Token::Word("command".to_string()),
        Token::RedirectFdIn(3, "input.txt".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["command".to_string()],
            redirections: vec![Redirection::FdInput(3, "input.txt".to_string())],
            compound: None,
        }])
    );
}

#[test]
fn test_parse_fd_output_redirection() {
    let tokens = vec![
        Token::Word("command".to_string()),
        Token::RedirectFdOut(2, "errors.log".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["command".to_string()],
            compound: None,
            redirections: vec![Redirection::FdOutput(2, "errors.log".to_string())],
        }])
    );
}

#[test]
fn test_parse_fd_append_redirection() {
    let tokens = vec![
        Token::Word("command".to_string()),
        Token::RedirectFdAppend(2, "errors.log".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["command".to_string()],
            compound: None,
            redirections: vec![Redirection::FdAppend(2, "errors.log".to_string())],
        }])
    );
}

#[test]
fn test_parse_fd_duplicate() {
    let tokens = vec![
        Token::Word("command".to_string()),
        Token::RedirectFdDup(2, 1),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["command".to_string()],
            compound: None,
            redirections: vec![Redirection::FdDuplicate(2, 1)],
        }])
    );
}

#[test]
fn test_parse_fd_close() {
    let tokens = vec![
        Token::Word("command".to_string()),
        Token::RedirectFdClose(2),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["command".to_string()],
            compound: None,
            redirections: vec![Redirection::FdClose(2)],
        }])
    );
}

#[test]
fn test_parse_fd_input_output() {
    let tokens = vec![
        Token::Word("command".to_string()),
        Token::RedirectFdInOut(3, "file.txt".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["command".to_string()],
            compound: None,
            redirections: vec![Redirection::FdInputOutput(3, "file.txt".to_string())],
        }])
    );
}

#[test]
fn test_parse_multiple_fd_redirections() {
    let tokens = vec![
        Token::Word("command".to_string()),
        Token::RedirectFdOut(2, "err.log".to_string()),
        Token::RedirectFdIn(3, "input.txt".to_string()),
        Token::RedirectFdAppend(4, "append.log".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["command".to_string()],
            compound: None,
            redirections: vec![
                Redirection::FdOutput(2, "err.log".to_string()),
                Redirection::FdInput(3, "input.txt".to_string()),
                Redirection::FdAppend(4, "append.log".to_string()),
            ],
        }])
    );
}

#[test]
fn test_parse_fd_swap_pattern() {
    let tokens = vec![
        Token::Word("command".to_string()),
        Token::RedirectFdDup(3, 1),
        Token::RedirectFdDup(1, 2),
        Token::RedirectFdDup(2, 3),
        Token::RedirectFdClose(3),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["command".to_string()],
            redirections: vec![
                Redirection::FdDuplicate(3, 1),
                Redirection::FdDuplicate(1, 2),
                Redirection::FdDuplicate(2, 3),
                Redirection::FdClose(3),
            ],
            compound: None,
        }])
    );
}

#[test]
fn test_parse_mixed_basic_and_fd_redirections() {
    let tokens = vec![
        Token::Word("command".to_string()),
        Token::RedirOut,
        Token::Word("output.txt".to_string()),
        Token::RedirectFdDup(2, 1),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["command".to_string()],
            redirections: vec![
                Redirection::Output("output.txt".to_string()),
                Redirection::FdDuplicate(2, 1),
            ],
            compound: None,
        }])
    );
}

#[test]
fn test_parse_fd_redirection_ordering() {
    // Test that redirections are preserved in left-to-right order
    let tokens = vec![
        Token::Word("command".to_string()),
        Token::RedirectFdOut(2, "first.log".to_string()),
        Token::RedirOut,
        Token::Word("second.txt".to_string()),
        Token::RedirectFdDup(2, 1),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["command".to_string()],
            redirections: vec![
                Redirection::FdOutput(2, "first.log".to_string()),
                Redirection::Output("second.txt".to_string()),
                Redirection::FdDuplicate(2, 1),
            ],
            compound: None,
        }])
    );
}

#[test]
fn test_parse_fd_redirection_with_pipe() {
    let tokens = vec![
        Token::Word("command".to_string()),
        Token::RedirectFdDup(2, 1),
        Token::Pipe,
        Token::Word("grep".to_string()),
        Token::Word("error".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["command".to_string()],
                redirections: vec![Redirection::FdDuplicate(2, 1)],
                compound: None,
            },
            ShellCommand {
                args: vec!["grep".to_string(), "error".to_string()],
                compound: None,
                redirections: Vec::new(),
            }
        ])
    );
}

#[test]
fn test_parse_all_fd_numbers() {
    // Test fd 0
    let tokens = vec![
        Token::Word("cmd".to_string()),
        Token::RedirectFdIn(0, "file".to_string()),
    ];
    let result = parse(tokens).unwrap();
    if let Ast::Pipeline(cmds) = result {
        assert_eq!(
            cmds[0].redirections[0],
            Redirection::FdInput(0, "file".to_string())
        );
    } else {
        panic!("Expected Pipeline");
    }

    // Test fd 9
    let tokens = vec![
        Token::Word("cmd".to_string()),
        Token::RedirectFdOut(9, "file".to_string()),
    ];
    let result = parse(tokens).unwrap();
    if let Ast::Pipeline(cmds) = result {
        assert_eq!(
            cmds[0].redirections[0],
            Redirection::FdOutput(9, "file".to_string())
        );
    } else {
        panic!("Expected Pipeline");
    }
}

#[test]
fn test_redirclobber_without_filename() {
    // Test that >| without a filename returns an error
    let tokens = vec![
        Token::Word("echo".to_string()),
        Token::Word("hello".to_string()),
        Token::RedirOutClobber,
    ];
    let result = parse(tokens);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "expected filename after >|");
}

#[test]
fn test_redirclobber_with_non_word_token() {
    // Test that >| followed by a non-Word token returns an error
    let tokens = vec![
        Token::Word("echo".to_string()),
        Token::Word("hello".to_string()),
        Token::RedirOutClobber,
        Token::Pipe,
    ];
    let result = parse(tokens);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "expected filename after >|");
}

#[test]
fn test_redirclobber_with_valid_filename() {
    // Test that >| with a valid filename works correctly
    let tokens = vec![
        Token::Word("echo".to_string()),
        Token::Word("hello".to_string()),
        Token::RedirOutClobber,
        Token::Word("output.txt".to_string()),
    ];
    let result = parse(tokens).unwrap();
    assert_eq!(
        result,
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["echo".to_string(), "hello".to_string()],
            redirections: vec![Redirection::OutputClobber("output.txt".to_string())],
            compound: None,
        }])
    );
}