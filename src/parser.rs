use super::lexer::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ast {
    Pipeline(Vec<ShellCommand>),
    If { condition: Box<Ast>, then_branch: Box<Ast>, else_branch: Option<Box<Ast>> },
    Case { word: String, cases: Vec<(String, Ast)>, default: Option<Box<Ast>> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand {
    pub args: Vec<String>,
    pub input: Option<String>,
    pub output: Option<String>,
    pub append: Option<String>,
}

pub fn parse(tokens: Vec<Token>) -> Result<Ast, String> {
    parse_slice(&tokens)
}

fn parse_slice(tokens: &[Token]) -> Result<Ast, String> {
    if tokens.is_empty() {
        return Err("No commands found".to_string());
    }

    // Check if it's an if statement
    if let Token::If = tokens[0] {
        return parse_if(tokens);
    }

    // Check if it's a case statement
    if let Token::Case = tokens[0] {
        return parse_case(tokens);
    }

    // Otherwise, parse as pipeline
    parse_pipeline(tokens)
}

fn parse_pipeline(tokens: &[Token]) -> Result<Ast, String> {
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
            Token::RightParen => {
                return Err("Unexpected ) in pipeline".to_string());
            }
            _ => {
                return Err(format!("Unexpected token in pipeline: {:?}", token));
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

fn parse_if(tokens: &[Token]) -> Result<Ast, String> {
    // Simple if: if condition ; then commands ; else commands ; fi
    let mut i = 1; // Skip 'if'

    // Parse condition until ;
    let mut cond_tokens = Vec::new();
    while i < tokens.len() && tokens[i] != Token::Semicolon {
        cond_tokens.push(tokens[i].clone());
        i += 1;
    }
    if i >= tokens.len() || tokens[i] != Token::Semicolon {
        return Err("Expected ; after if condition".to_string());
    }
    i += 1; // Skip ;

    if i >= tokens.len() || tokens[i] != Token::Then {
        return Err("Expected then after if condition".to_string());
    }
    i += 1; // Skip then

    // Parse then branch until ; or else or fi
    let mut then_tokens = Vec::new();
    while i < tokens.len() && tokens[i] != Token::Semicolon && tokens[i] != Token::Else && tokens[i] != Token::Fi {
        then_tokens.push(tokens[i].clone());
        i += 1;
    }

    let then_ast = parse_slice(&then_tokens)?;

    // Skip the ; after then branch if present
    if i < tokens.len() && tokens[i] == Token::Semicolon {
        i += 1;
    }

    let else_ast = if i < tokens.len() && tokens[i] == Token::Else {
        i += 1; // Skip else
        let mut else_tokens = Vec::new();
        while i < tokens.len() && tokens[i] != Token::Semicolon && tokens[i] != Token::Fi {
            else_tokens.push(tokens[i].clone());
            i += 1;
        }
        // Skip ; after else branch
        if i < tokens.len() && tokens[i] == Token::Semicolon {
            i += 1;
        }
        Some(Box::new(parse_slice(&else_tokens)?))
    } else {
        None
    };

    if i >= tokens.len() || tokens[i] != Token::Fi {
        return Err("Expected fi".to_string());
    }

    let condition = parse_slice(&cond_tokens)?;
    Ok(Ast::If {
        condition: Box::new(condition),
        then_branch: Box::new(then_ast),
        else_branch: else_ast,
    })
}

fn parse_case(tokens: &[Token]) -> Result<Ast, String> {
    // Simple case: case word in pattern) commands ;; esac
    let mut i = 1; // Skip 'case'

    // Parse word
    if i >= tokens.len() || !matches!(tokens[i], Token::Word(_)) {
        return Err("Expected word after case".to_string());
    }
    let word = if let Token::Word(ref w) = tokens[i] {
        w.clone()
    } else {
        unreachable!()
    };
    i += 1;

    if i >= tokens.len() || tokens[i] != Token::In {
        return Err("Expected in after case word".to_string());
    }
    i += 1;

    // Parse pattern
    if i >= tokens.len() || !matches!(tokens[i], Token::Word(_)) {
        return Err("Expected pattern after in".to_string());
    }
    let pattern = if let Token::Word(ref p) = tokens[i] {
        p.clone()
    } else {
        unreachable!()
    };
    i += 1;

    if i >= tokens.len() || tokens[i] != Token::RightParen {
        return Err("Expected ) after pattern".to_string());
    }
    i += 1;

    // Parse commands
    let mut commands_tokens = Vec::new();
    while i < tokens.len() && tokens[i] != Token::DoubleSemicolon && tokens[i] != Token::Esac {
        commands_tokens.push(tokens[i].clone());
        i += 1;
    }

    let commands_ast = parse_slice(&commands_tokens)?;

    if i >= tokens.len() || tokens[i] != Token::DoubleSemicolon {
        return Err("Expected ;; after commands".to_string());
    }
    i += 1;

    if i >= tokens.len() || tokens[i] != Token::Esac {
        return Err("Expected esac".to_string());
    }

    Ok(Ast::Case {
        word,
        cases: vec![(pattern, commands_ast)],
        default: None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::lexer::Token;
    use super::*;

    #[test]
    fn test_single_command() {
        let tokens = vec![Token::Word("ls".to_string())];
        let result = parse(tokens).unwrap();
        assert_eq!(
            result,
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["ls".to_string()],
                input: None,
                output: None,
                append: None,
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
                input: None,
                output: None,
                append: None,
            }])
        );
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
        assert_eq!(
            result,
            Ast::Pipeline(vec![
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
            ])
        );
    }

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
                input: Some("input.txt".to_string()),
                output: None,
                append: None,
            }])
        );
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
        assert_eq!(
            result,
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                input: None,
                output: Some("output.txt".to_string()),
                append: None,
            }])
        );
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
        assert_eq!(
            result,
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["echo".to_string(), "hello".to_string()],
                input: None,
                output: None,
                append: Some("output.txt".to_string()),
            }])
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
            ])
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
    fn test_redirection_without_file() {
        // Parser doesn't check for missing file, just skips if no token after
        let tokens = vec![Token::Word("cat".to_string()), Token::RedirIn];
        let result = parse(tokens).unwrap();
        assert_eq!(
            result,
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["cat".to_string()],
                input: None,
                output: None,
                append: None,
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
                input: Some("file1.txt".to_string()),
                output: Some("file2.txt".to_string()),
                append: None,
            }])
        );
    }

    #[test]
    fn test_parse_if() {
        let tokens = vec![
            Token::If,
            Token::Word("true".to_string()),
            Token::Semicolon,
            Token::Then,
            Token::Word("echo".to_string()),
            Token::Word("yes".to_string()),
            Token::Semicolon,
            Token::Fi,
        ];
        let result = parse(tokens).unwrap();
        if let Ast::If { condition, then_branch, else_branch } = result {
            if let Ast::Pipeline(cmds) = *condition {
                assert_eq!(cmds[0].args, vec!["true"]);
            } else {
                panic!("condition not pipeline");
            }
            if let Ast::Pipeline(cmds) = *then_branch {
                assert_eq!(cmds[0].args, vec!["echo", "yes"]);
            } else {
                panic!("then_branch not pipeline");
            }
            assert!(else_branch.is_none());
        } else {
            panic!("not if");
        }
    }
}
