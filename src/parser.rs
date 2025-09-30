use super::lexer::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ast {
    Pipeline(Vec<ShellCommand>),
    Sequence(Vec<Ast>),
    Assignment {
        var: String,
        value: String,
    },
    LocalAssignment {
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
    FunctionDefinition {
        name: String,
        body: Box<Ast>,
    },
    FunctionCall {
        name: String,
        args: Vec<String>,
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
    // First, try to detect and parse function definitions that span multiple lines
    if tokens.len() >= 4 {
        if let (Token::Word(_), Token::LeftParen, Token::RightParen, Token::LeftBrace) =
            (&tokens[0], &tokens[1], &tokens[2], &tokens[3])
        {
            // Look for the matching RightBrace
            let mut brace_depth = 0;
            let mut function_end = tokens.len();

            for j in 0..tokens.len() {
                match &tokens[j] {
                    Token::LeftBrace => {
                        brace_depth += 1;
                    },
                    Token::RightBrace => {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            function_end = j + 1; // Include the closing brace
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if brace_depth == 0 && function_end <= tokens.len() {
                // We found the complete function definition
                let function_tokens = &tokens[0..function_end];
                let remaining_tokens = &tokens[function_end..];

                let function_ast = parse_function_definition(function_tokens)?;

                if remaining_tokens.is_empty() {
                    return Ok(function_ast);
                } else {
                    // There are more commands after the function
                    let remaining_ast = parse_commands_sequentially(remaining_tokens)?;
                    return Ok(Ast::Sequence(vec![function_ast, remaining_ast]));
                }
            }
        }
    }

    // Also check for legacy function definition format (word with parentheses followed by brace)
    if tokens.len() >= 2 {
        if let Token::Word(ref word) = tokens[0] {
            if let Some(paren_pos) = word.find('(') {
                if word.ends_with(')') && paren_pos > 0 {
                    if tokens[1] == Token::LeftBrace {
                        return parse_function_definition(&tokens);
                    }
                }
            }
        }
    }

    // Fall back to normal parsing
    parse_commands_sequentially(&tokens)
}

fn parse_slice(tokens: &[Token]) -> Result<Ast, String> {
    if tokens.is_empty() {
        return Err("No commands found".to_string());
    }

    // Check if it's an assignment
    if tokens.len() == 2 {
        // Check for pattern: VAR= VALUE
        if let (Token::Word(var_eq), Token::Word(value)) = (&tokens[0], &tokens[1]) {
            if let Some(eq_pos) = var_eq.find('=') {
                if eq_pos > 0 && eq_pos < var_eq.len() - 1 {
                    let var = var_eq[..eq_pos].to_string();
                    let full_value = format!("{}{}", &var_eq[eq_pos + 1..], value);
                    // Basic validation: variable name should start with letter or underscore
                    if var.chars().next().unwrap().is_alphabetic() || var.starts_with('_') {
                        return Ok(Ast::Assignment {
                            var,
                            value: full_value,
                        });
                    }
                }
            }
        }
    }

    // Check if it's an assignment (VAR= VALUE)
    if tokens.len() == 2 {
        if let (Token::Word(var_eq), Token::Word(value)) = (&tokens[0], &tokens[1]) {
            if let Some(eq_pos) = var_eq.find('=') {
                if eq_pos > 0 && eq_pos == var_eq.len() - 1 {
                    let var = var_eq[..eq_pos].to_string();
                    // Basic validation: variable name should start with letter or underscore
                    if var.chars().next().unwrap().is_alphabetic() || var.starts_with('_') {
                        return Ok(Ast::Assignment {
                            var,
                            value: value.clone(),
                        });
                    }
                }
            }
        }
    }

    // Check if it's a local assignment (local VAR VALUE)
    if tokens.len() == 3 {
        if let (Token::Local, Token::Word(var), Token::Word(value)) = (&tokens[0], &tokens[1], &tokens[2]) {
            // Basic validation: variable name should start with letter or underscore
            if var.chars().next().unwrap().is_alphabetic() || var.starts_with('_') {
                return Ok(Ast::LocalAssignment {
                    var: var.clone(),
                    value: value.clone(),
                });
            }
        }
    }

    // Check if it's a local assignment (local VAR=VALUE)
    if tokens.len() == 2 {
        if let (Token::Local, Token::Word(var_eq)) = (&tokens[0], &tokens[1]) {
            if let Some(eq_pos) = var_eq.find('=') {
                if eq_pos > 0 && eq_pos < var_eq.len() - 1 {
                    let var = var_eq[..eq_pos].to_string();
                    let value = var_eq[eq_pos + 1..].to_string();
                    // Basic validation: variable name should start with letter or underscore
                    if var.chars().next().unwrap().is_alphabetic() || var.starts_with('_') {
                        return Ok(Ast::LocalAssignment { var, value });
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

    // Check if it's a function definition
    // Pattern: Word LeftParen RightParen LeftBrace
    if tokens.len() >= 4 {
        if let (Token::Word(word), Token::LeftParen, Token::RightParen, Token::LeftBrace) =
            (&tokens[0], &tokens[1], &tokens[2], &tokens[3])
        {
            if word.chars().next().unwrap().is_alphabetic() || word.starts_with('_') {
                return parse_function_definition(tokens);
            }
        }
    }

    // Also check for function definition with parentheses in the word (legacy support)
    if tokens.len() >= 2 {
        if let Token::Word(ref word) = tokens[0] {
            if let Some(paren_pos) = word.find('(') {
                if word.ends_with(')') && paren_pos > 0 {
                    let func_name = &word[..paren_pos];
                    if func_name.chars().next().unwrap().is_alphabetic() || func_name.starts_with('_') {
                        if tokens[1] == Token::LeftBrace {
                            return parse_function_definition(tokens);
                        }
                    }
                }
            }
        }
    }

    // Check if it's a function call (word followed by arguments)
    // For Phase 1, we'll parse as regular pipeline and handle function calls in executor

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

            // If we didn't find a matching fi, include all remaining tokens
            // This handles the case where the if statement is incomplete
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
            // But check if the next token after newline is a control flow keyword
            while i < tokens.len() {
                if tokens[i] == Token::Newline || tokens[i] == Token::Semicolon {
                    // Look ahead to see if the next non-newline token is else/elif/fi
                    let mut j = i + 1;
                    while j < tokens.len() && tokens[j] == Token::Newline {
                        j += 1;
                    }
                    // If we find else/elif/fi, this is likely part of an if statement that wasn't properly detected
                    if j < tokens.len()
                        && (tokens[j] == Token::Else
                            || tokens[j] == Token::Elif
                            || tokens[j] == Token::Fi)
                    {
                        // Skip this token and continue - it will be handled as a parse error
                        i = j + 1;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
        }

        let command_tokens = &tokens[start..i];
        if !command_tokens.is_empty() {
            // Don't try to parse orphaned else/elif/fi tokens
            if command_tokens.len() == 1 {
                match command_tokens[0] {
                    Token::Else | Token::Elif | Token::Fi => {
                        // Skip orphaned control flow tokens
                        if i < tokens.len()
                            && (tokens[i] == Token::Newline || tokens[i] == Token::Semicolon)
                        {
                            i += 1;
                        }
                        continue;
                    }
                    _ => {}
                }
            }

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
        // Parse condition until ; or newline or then
        let mut cond_tokens = Vec::new();
        while i < tokens.len()
            && tokens[i] != Token::Semicolon
            && tokens[i] != Token::Newline
            && tokens[i] != Token::Then
        {
            cond_tokens.push(tokens[i].clone());
            i += 1;
        }

        // Skip ; or newline if present
        if i < tokens.len() && (tokens[i] == Token::Semicolon || tokens[i] == Token::Newline) {
            i += 1;
        }

        // Skip any additional newlines
        while i < tokens.len() && tokens[i] == Token::Newline {
            i += 1;
        }

        if i >= tokens.len() || tokens[i] != Token::Then {
            return Err("Expected then after if/elif condition".to_string());
        }
        i += 1; // Skip then

        // Skip any newlines after then
        while i < tokens.len() && tokens[i] == Token::Newline {
            i += 1;
        }

        // Parse then branch - collect all tokens until we hit else/elif/fi
        // We need to handle nested structures properly
        let mut then_tokens = Vec::new();
        let mut depth = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::If => {
                    depth += 1;
                    then_tokens.push(tokens[i].clone());
                }
                Token::Fi => {
                    if depth > 0 {
                        depth -= 1;
                        then_tokens.push(tokens[i].clone());
                    } else {
                        break; // This fi closes our if
                    }
                }
                Token::Else | Token::Elif if depth == 0 => {
                    break; // These belong to our if, not nested ones
                }
                Token::Newline => {
                    // Skip newlines but check what comes after
                    let mut j = i + 1;
                    while j < tokens.len() && tokens[j] == Token::Newline {
                        j += 1;
                    }
                    if j < tokens.len()
                        && depth == 0
                        && (tokens[j] == Token::Else
                            || tokens[j] == Token::Elif
                            || tokens[j] == Token::Fi)
                    {
                        i = j; // Skip to the keyword
                        break;
                    }
                    // Otherwise it's just a newline in the middle of commands
                    then_tokens.push(tokens[i].clone());
                }
                _ => {
                    then_tokens.push(tokens[i].clone());
                }
            }
            i += 1;
        }

        // Skip any trailing newlines
        while i < tokens.len() && tokens[i] == Token::Newline {
            i += 1;
        }

        let then_ast = if then_tokens.is_empty() {
            // Empty then branch - create a no-op
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["true".to_string()],
                input: None,
                output: None,
                append: None,
            }])
        } else {
            parse_commands_sequentially(&then_tokens)?
        };

        let condition = parse_slice(&cond_tokens)?;
        branches.push((Box::new(condition), Box::new(then_ast)));

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
        let mut depth = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::If => {
                    depth += 1;
                    else_tokens.push(tokens[i].clone());
                }
                Token::Fi => {
                    if depth > 0 {
                        depth -= 1;
                        else_tokens.push(tokens[i].clone());
                    } else {
                        break; // This fi closes our if
                    }
                }
                Token::Newline => {
                    // Skip newlines but check what comes after
                    let mut j = i + 1;
                    while j < tokens.len() && tokens[j] == Token::Newline {
                        j += 1;
                    }
                    if j < tokens.len() && depth == 0 && tokens[j] == Token::Fi {
                        i = j; // Skip to fi
                        break;
                    }
                    // Otherwise it's just a newline in the middle of commands
                    else_tokens.push(tokens[i].clone());
                }
                _ => {
                    else_tokens.push(tokens[i].clone());
                }
            }
            i += 1;
        }

        let else_ast = if else_tokens.is_empty() {
            // Empty else branch - create a no-op
            Ast::Pipeline(vec![ShellCommand {
                args: vec!["true".to_string()],
                input: None,
                output: None,
                append: None,
            }])
        } else {
            parse_commands_sequentially(&else_tokens)?
        };

        Some(Box::new(else_ast))
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

fn parse_function_definition(tokens: &[Token]) -> Result<Ast, String> {
    if tokens.len() < 2 {
        return Err("Function definition too short".to_string());
    }

    // Extract function name from first token
    let func_name = if let Token::Word(word) = &tokens[0] {
        // Handle legacy format with parentheses in the word (e.g., "legacyfunc()")
        if let Some(paren_pos) = word.find('(') {
            if word.ends_with(')') && paren_pos > 0 {
                word[..paren_pos].to_string()
            } else {
                word.clone()
            }
        } else {
            word.clone()
        }
    } else {
        return Err("Function name must be a word".to_string());
    };

    // Find the opening brace and body
    let brace_pos = if tokens.len() >= 4 && tokens[1] == Token::LeftParen && tokens[2] == Token::RightParen {
        // Standard format: name() {
        if tokens[3] != Token::LeftBrace {
            return Err("Expected { after function name".to_string());
        }
        3
    } else if tokens.len() >= 2 && tokens[1] == Token::LeftBrace {
        // Legacy format: name() {
        1
    } else {
        return Err("Expected ( after function name or { for legacy format".to_string());
    };

    // Find the matching closing brace
    let mut brace_depth = 0;
    let mut body_end = 0;
    let mut found_closing = false;

    for (i, token) in tokens[brace_pos + 1..].iter().enumerate() {
        match token {
            Token::LeftBrace => {
                brace_depth += 1;
            }
            Token::RightBrace => {
                if brace_depth == 0 {
                    // This is our matching closing brace
                    body_end = brace_pos + 1 + i;
                    found_closing = true;
                    break;
                } else {
                    brace_depth -= 1;
                }
            }
            _ => {}
        }
    }

    if !found_closing {
        return Err("Missing closing } for function definition".to_string());
    }

    // Extract body tokens (everything between { and })
    let body_tokens = &tokens[brace_pos + 1..body_end];

    // Parse the function body using the existing parser
    let body_ast = if body_tokens.is_empty() {
        // Empty function body
        Ast::Pipeline(vec![ShellCommand {
            args: vec!["true".to_string()],
            input: None,
            output: None,
            append: None,
        }])
    } else {
        parse_commands_sequentially(body_tokens)?
    };

    Ok(Ast::FunctionDefinition {
        name: func_name,
        body: Box::new(body_ast),
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
                input: None,
                output: Some("output.txt".to_string()),
                append: None,
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

    #[test]
    fn test_parse_local_assignment() {
        let tokens = vec![
            Token::Local,
            Token::Word("MY_VAR=test_value".to_string()),
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
        let tokens = vec![
            Token::Local,
            Token::Word("123VAR=value".to_string()),
        ];
        let result = parse(tokens);
        // Should return an error since 123VAR is not a valid variable name
        assert!(result.is_err());
    }
}
