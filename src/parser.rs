use super::lexer::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ast {
    Pipeline(Vec<ShellCommand>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand {
    pub args: Vec<String>,
    pub input: Option<String>,
    pub output: Option<String>,
    pub append: Option<String>,
}

pub fn parse(tokens: Vec<Token>) -> Result<Ast, String> {
    let mut commands = Vec::new();
    let mut current_cmd = ShellCommand {
        args: Vec::new(),
        input: None,
        output: None,
        append: None,
    };

    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        match token {
            Token::Word(word) => {
                current_cmd.args.push(word.clone());
            }
            Token::Pipe => {
                if !current_cmd.args.is_empty() {
                    commands.push(current_cmd.clone());
                    current_cmd = ShellCommand {
                        args: Vec::new(),
                        input: None,
                        output: None,
                        append: None,
                    };
                }
            }
            Token::RedirIn => {
                i += 1;
                if i < tokens.len() {
                    if let Token::Word(ref file) = tokens[i] {
                        current_cmd.input = Some(file.clone());
                    }
                }
            }
            Token::RedirOut => {
                i += 1;
                if i < tokens.len() {
                    if let Token::Word(ref file) = tokens[i] {
                        current_cmd.output = Some(file.clone());
                    }
                }
            }
            Token::RedirAppend => {
                i += 1;
                if i < tokens.len() {
                    if let Token::Word(ref file) = tokens[i] {
                        current_cmd.append = Some(file.clone());
                    }
                }
            }
        }
        i += 1;
    }

    if !current_cmd.args.is_empty() {
        commands.push(current_cmd);
    }

    if commands.is_empty() {
        return Err("No commands found".to_string());
    }

    Ok(Ast::Pipeline(commands))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::lexer::Token;

    #[test]
    fn test_single_command() {
        let tokens = vec![Token::Word("ls".to_string())];
        let result = parse(tokens).unwrap();
        assert_eq!(result, Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["ls".to_string()],
                input: None,
                output: None,
                append: None,
            }
        ]));
    }

    #[test]
    fn test_command_with_args() {
        let tokens = vec![
            Token::Word("ls".to_string()),
            Token::Word("-la".to_string()),
        ];
        let result = parse(tokens).unwrap();
        assert_eq!(result, Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["ls".to_string(), "-la".to_string()],
                input: None,
                output: None,
                append: None,
            }
        ]));
    }

    #[test]
    fn test_pipeline() {
        let tokens = vec![
            Token::Word("ls".to_string()),
            Token::Pipe,
            Token::Word("grep".to_string()),
            Token::Word("txt".to_string()),
        ];
        let result = parse(tokens).unwrap();
        assert_eq!(result, Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["ls".to_string()],
                input: None,
                output: None,
                append: None,
            },
            ShellCommand {
                args: vec!["grep".to_string(), "txt".to_string()],
                input: None,
                output: None,
                append: None,
            }
        ]));
    }

    #[test]
    fn test_input_redirection() {
        let tokens = vec![
            Token::Word("cat".to_string()),
            Token::RedirIn,
            Token::Word("input.txt".to_string()),
        ];
        let result = parse(tokens).unwrap();
        assert_eq!(result, Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["cat".to_string()],
                input: Some("input.txt".to_string()),
                output: None,
                append: None,
            }
        ]));
    }

    #[test]
    fn test_output_redirection() {
        let tokens = vec![
            Token::Word("echo".to_string()),
            Token::Word("hello".to_string()),
            Token::RedirOut,
            Token::Word("output.txt".to_string()),
        ];
        let result = parse(tokens).unwrap();
        assert_eq!(result, Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                input: None,
                output: Some("output.txt".to_string()),
                append: None,
            }
        ]));
    }

    #[test]
    fn test_append_redirection() {
        let tokens = vec![
            Token::Word("echo".to_string()),
            Token::Word("hello".to_string()),
            Token::RedirAppend,
            Token::Word("output.txt".to_string()),
        ];
        let result = parse(tokens).unwrap();
        assert_eq!(result, Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                input: None,
                output: None,
                append: Some("output.txt".to_string()),
            }
        ]));
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
        assert_eq!(result, Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["cat".to_string()],
                input: Some("input.txt".to_string()),
                output: None,
                append: None,
            },
            ShellCommand {
                args: vec!["grep".to_string(), "pattern".to_string()],
                input: None,
                output: None,
                append: None,
            },
            ShellCommand {
                args: vec!["sort".to_string()],
                input: None,
                output: Some("output.txt".to_string()),
                append: None,
            }
        ]));
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
    fn test_redirection_without_file() {
        // Parser doesn't check for missing file, just skips if no token after
        let tokens = vec![
            Token::Word("cat".to_string()),
            Token::RedirIn,
        ];
        let result = parse(tokens).unwrap();
        assert_eq!(result, Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["cat".to_string()],
                input: None,
                output: None,
                append: None,
            }
        ]));
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
        assert_eq!(result, Ast::Pipeline(vec![
            ShellCommand {
                args: vec!["cat".to_string()],
                input: Some("file1.txt".to_string()),
                output: Some("file2.txt".to_string()),
                append: None,
            }
        ]));
    }
}
