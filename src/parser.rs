use super::lexer::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ast {
    Pipeline(Vec<ShellCommand>),
    Sequence(Vec<Ast>),
    Assignment {
        var: String,
        value: String,
    },
    If {
        branches: Vec<(Box<Ast>, Box<Ast>)>, // (condition, then_branch)
        else_branch: Option<Box<Ast>>,
    },
    Case {
        word: String,
        cases: Vec<(Vec<String>, Ast)>,
        default: Option<Box<Ast>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand {
    pub args: Vec<String>,
    pub input: Option<String>,
    pub output: Option<String>,
    pub append: Option<String>,
}

pub fn parse(tokens: Vec<Token>) -> Result<Ast, String> {
    parse_commands_sequentially(&tokens)
}

fn parse_slice(tokens: &[Token]) -> Result<Ast, String> {
    if tokens.is_empty() {
        return Err("No commands found".to_string());
    }

    // Check if it's an assignment
    if tokens.len() == 2 {
        // Check for pattern: VAR= VALUE
        if let (Token::Word(ref var_eq), Token::Word(ref value)) = (&tokens[0], &tokens[1]) {
            if let Some(eq_pos) = var_eq.find('=') {
                if eq_pos > 0 && eq_pos < var_eq.len() - 1 {
                    let var = var_eq[..eq_pos].to_string();
                    let full_value = format!("{}{}", &var_eq[eq_pos + 1..], value);
                    // Basic validation: variable name should start with letter or underscore
                    if var.chars().next().unwrap().is_alphabetic() || var.starts_with('_') {
                        return Ok(Ast::Assignment { var, value: full_value });
                    }
                }
            }
        }
    }

    // Check if it's an assignment (VAR= VALUE)
    if tokens.len() == 2 {
        if let (Token::Word(ref var_eq), Token::Word(ref value)) = (&tokens[0], &tokens[1]) {
            if let Some(eq_pos) = var_eq.find('=') {
                if eq_pos > 0 && eq_pos == var_eq.len() - 1 {
                    let var = var_eq[..eq_pos].to_string();
                    // Basic validation: variable name should start with letter or underscore
                    if var.chars().next().unwrap().is_alphabetic() || var.starts_with('_') {
                        return Ok(Ast::Assignment { var, value: value.clone() });
                    }
                }
            }
        }
    }

    // Check if it's an assignment (single token with =)
    if tokens.len() == 1 {
        if let Token::Word(ref word) = tokens[0] {
            if let Some(eq_pos) = word.find('=') {
                if eq_pos > 0 && eq_pos < word.len() - 1 {
                    let var = word[..eq_pos].to_string();
                    let value = word[eq_pos + 1..].to_string();
                    // Basic validation: variable name should start with letter or underscore
                    if var.chars().next().unwrap().is_alphabetic() || var.starts_with('_') {
                        return Ok(Ast::Assignment { var, value });
                    }
                }
            }
        }
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

fn parse_commands_sequentially(tokens: &[Token]) -> Result<Ast, String> {
    let mut i = 0;
    let mut commands = Vec::new();

    while i < tokens.len() {
        // Skip whitespace and comments
        while i < tokens.len() {
            match &tokens[i] {
                Token::Newline => {
                    i += 1;
                }
                Token::Word(word) if word.starts_with('#') => {
                    // Skip comment line
                    while i < tokens.len() && tokens[i] != Token::Newline {
                        i += 1;
                    }
                    if i < tokens.len() {
                        i += 1; // Skip the newline
                    }
                }
                _ => break,
            }
        }

        if i >= tokens.len() {
            break;
        }

        // Find the end of this command
        let start = i;

        // Special handling for compound commands
        if tokens[i] == Token::If {
            // For if statements, find the matching fi
            let mut depth = 0;
            while i < tokens.len() {
                match tokens[i] {
                    Token::If => depth += 1,
                    Token::Fi => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1; // Include the fi
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        } else if tokens[i] == Token::Case {
            // For case statements, find the matching esac
            while i < tokens.len() {
                if tokens[i] == Token::Esac {
                    i += 1; // Include the esac
                    break;
                }
                i += 1;
            }
        } else {
            // For simple commands, stop at newline or semicolon
            while i < tokens.len() && tokens[i] != Token::Newline && tokens[i] != Token::Semicolon {
                i += 1;
            }
        }

        let command_tokens = &tokens[start..i];
        if !command_tokens.is_empty() {
            let ast = parse_slice(command_tokens)?;
            commands.push(ast);
        }

        if i < tokens.len() && (tokens[i] == Token::Newline || tokens[i] == Token::Semicolon) {
            i += 1;
        }
    }

    if commands.is_empty() {
        return Err("No commands found".to_string());
    }

    if commands.len() == 1 {
        Ok(commands.into_iter().next().unwrap())
    } else {
        Ok(Ast::Sequence(commands))
    }
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
            Token::Newline => {
                // Newlines are handled at the sequence level, skip them in pipelines
                i += 1;
                continue;
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
    let mut i = 1; // Skip 'if'
    let mut branches = Vec::new();

    loop {
        // Parse condition until ; or newline
        let mut cond_tokens = Vec::new();
        while i < tokens.len() && tokens[i] != Token::Semicolon && tokens[i] != Token::Newline {
            cond_tokens.push(tokens[i].clone());
            i += 1;
        }
        if i >= tokens.len() || (tokens[i] != Token::Semicolon && tokens[i] != Token::Newline) {
            return Err("Expected ; after if/elif condition".to_string());
        }
        i += 1; // Skip ; or newline

        if i >= tokens.len() || tokens[i] != Token::Then {
            return Err("Expected then after if/elif condition".to_string());
        }
        i += 1; // Skip then

        // Skip any newlines after then
        while i < tokens.len() && tokens[i] == Token::Newline {
            i += 1;
        }
        // Parse then branch until ; or newline or else or elif or fi
        let mut then_tokens = Vec::new();
        while i < tokens.len()
            && tokens[i] != Token::Semicolon
            && tokens[i] != Token::Newline
            && tokens[i] != Token::Else
            && tokens[i] != Token::Elif
            && tokens[i] != Token::Fi
        {
            then_tokens.push(tokens[i].clone());
            i += 1;
        }
        // Skip any newlines after the then branch
        while i < tokens.len() && tokens[i] == Token::Newline {
            i += 1;
        }

        let then_ast = parse_slice(&then_tokens)?;
        let condition = parse_slice(&cond_tokens)?;
        branches.push((Box::new(condition), Box::new(then_ast)));

        // Skip the ; or newline after then branch if present
        if i < tokens.len() && (tokens[i] == Token::Semicolon || tokens[i] == Token::Newline) {
            i += 1;
        }

        // Check next
        if i < tokens.len() && tokens[i] == Token::Elif {
            i += 1; // Skip elif, continue loop
        } else {
            break;
        }
    }

    let else_ast = if i < tokens.len() && tokens[i] == Token::Else {
        i += 1; // Skip else
        // Skip any newlines after else
        while i < tokens.len() && tokens[i] == Token::Newline {
            i += 1;
        }
        let mut else_tokens = Vec::new();
        while i < tokens.len() && tokens[i] != Token::Semicolon && tokens[i] != Token::Newline && tokens[i] != Token::Fi {
            else_tokens.push(tokens[i].clone());
            i += 1;
        }
        // Skip ; or newlines after else branch
        while i < tokens.len() && (tokens[i] == Token::Semicolon || tokens[i] == Token::Newline) {
            i += 1;
        }
        Some(Box::new(parse_slice(&else_tokens)?))
    } else {
        None
    };

    if i >= tokens.len() || tokens[i] != Token::Fi {
        return Err("Expected fi".to_string());
    }

    Ok(Ast::If {
        branches,
        else_branch: else_ast,
    })
}

fn parse_case(tokens: &[Token]) -> Result<Ast, String> {
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

    let mut cases = Vec::new();
    let mut default = None;

    loop {
        // Skip newlines
        while i < tokens.len() && tokens[i] == Token::Newline {
            i += 1;
        }

        if i >= tokens.len() {
            return Err("Unexpected end in case statement".to_string());
        }

        if tokens[i] == Token::Esac {
            break;
        }

        // Parse patterns
        let mut patterns = Vec::new();
        while i < tokens.len() && tokens[i] != Token::RightParen {
            if let Token::Word(ref p) = tokens[i] {
                // Split pattern on |
                for pat in p.split('|') {
                    patterns.push(pat.to_string());
                }
            } else if tokens[i] == Token::Pipe {
                // Skip | separator
            } else if tokens[i] == Token::Newline {
                // Skip newlines in patterns
            } else {
                return Err(format!("Expected pattern, found {:?}", tokens[i]));
            }
            i += 1;
        }

        if i >= tokens.len() || tokens[i] != Token::RightParen {
            return Err("Expected ) after patterns".to_string());
        }
        i += 1;

        // Parse commands
        let mut commands_tokens = Vec::new();
        while i < tokens.len() && tokens[i] != Token::DoubleSemicolon && tokens[i] != Token::Esac {
            commands_tokens.push(tokens[i].clone());
            i += 1;
        }

        let commands_ast = parse_slice(&commands_tokens)?;

        if i >= tokens.len() {
            return Err("Unexpected end in case statement".to_string());
        }

        if tokens[i] == Token::DoubleSemicolon {
            i += 1;
            // Check if this is the default case (*)
            if patterns.len() == 1 && patterns[0] == "*" {
                default = Some(Box::new(commands_ast));
            } else {
                cases.push((patterns, commands_ast));
            }
        } else if tokens[i] == Token::Esac {
            // Last case without ;;
            if patterns.len() == 1 && patterns[0] == "*" {
                default = Some(Box::new(commands_ast));
            } else {
                cases.push((patterns, commands_ast));
            }
            break;
        } else {
            return Err("Expected ;; or esac after commands".to_string());
        }
    }

    Ok(Ast::Case {
        word,
        cases,
        default,
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
                assert_eq!(cmds[0].args, vec!["echo", "yes"]);
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
            Token::Word("echo".to_string()),
            Token::Word("no".to_string()),
            Token::Semicolon,
            Token::Elif,
            Token::Word("true".to_string()),
            Token::Semicolon,
            Token::Then,
            Token::Word("echo".to_string()),
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
            // First branch: false -> echo no
            let (condition1, then1) = &branches[0];
            if let Ast::Pipeline(cmds) = &**condition1 {
                assert_eq!(cmds[0].args, vec!["false"]);
            }
            if let Ast::Pipeline(cmds) = &**then1 {
                assert_eq!(cmds[0].args, vec!["echo", "no"]);
            }
            // Second branch: true -> echo yes
            let (condition2, then2) = &branches[1];
            if let Ast::Pipeline(cmds) = &**condition2 {
                assert_eq!(cmds[0].args, vec!["true"]);
            }
            if let Ast::Pipeline(cmds) = &**then2 {
                assert_eq!(cmds[0].args, vec!["echo", "yes"]);
            }
            assert!(else_branch.is_none());
        } else {
            panic!("not if");
        }
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
}
