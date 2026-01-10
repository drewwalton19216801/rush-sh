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
    For {
        variable: String,
        items: Vec<String>,
        body: Box<Ast>,
    },
    While {
        condition: Box<Ast>,
        body: Box<Ast>,
    },
    FunctionDefinition {
        name: String,
        body: Box<Ast>,
    },
    FunctionCall {
        name: String,
        args: Vec<String>,
    },
    Return {
        value: Option<String>,
    },
    And {
        left: Box<Ast>,
        right: Box<Ast>,
    },
    Or {
        left: Box<Ast>,
        right: Box<Ast>,
    },
    /// Subshell execution: (commands)
    /// Commands execute in an isolated copy of the shell state
    Subshell {
        body: Box<Ast>,
    },
}

/// Represents a single redirection operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Redirection {
    /// Input from file: < file or N< file
    Input(String),
    /// Output to file: > file or N> file
    Output(String),
    /// Append to file: >> file or N>> file
    Append(String),
    /// Input from file with explicit fd: N< file
    FdInput(i32, String),
    /// Output to file with explicit fd: N> file
    FdOutput(i32, String),
    /// Append to file with explicit fd: N>> file
    FdAppend(i32, String),
    /// Duplicate file descriptor: N>&M or N<&M
    FdDuplicate(i32, i32),
    /// Close file descriptor: N>&- or N<&-
    FdClose(i32),
    /// Open file for read/write: N<> file
    FdInputOutput(i32, String),
    /// Here-document: << EOF ... EOF
    HereDoc(String, String),
    /// Here-string: <<< "string"
    HereString(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellCommand {
    pub args: Vec<String>,
    /// All redirections in order of appearance (for POSIX left-to-right processing)
    pub redirections: Vec<Redirection>,
    /// Optional compound command (subshell, command group, etc.)
    /// If present, this takes precedence over args
    pub compound: Option<Box<Ast>>,
}

/// Helper function to validate if a string is a valid variable name.
/// Returns true if the name starts with a letter or underscore.
fn is_valid_variable_name(name: &str) -> bool {
    if let Some(first_char) = name.chars().next() {
        first_char.is_alphabetic() || first_char == '_'
    } else {
        false
    }
}

/// Helper function to create an empty body AST (a no-op that returns success).
/// Used for empty then/else branches, empty loop bodies, and empty function bodies.
fn create_empty_body_ast() -> Ast {
    Ast::Pipeline(vec![ShellCommand {
        args: vec!["true".to_string()],
        redirections: Vec::new(),
        compound: None,
    }])
}

/// Helper function to skip consecutive newline tokens.
/// Updates the index to point to the first non-newline token.
fn skip_newlines(tokens: &[Token], i: &mut usize) {
    while *i < tokens.len() && tokens[*i] == Token::Newline {
        *i += 1;
    }
}

/// Helper function to skip to the matching 'fi' token for an 'if' statement.
/// Handles nested if statements correctly.
fn skip_to_matching_fi(tokens: &[Token], i: &mut usize) {
    let mut if_depth = 1;
    *i += 1; // Move past the 'if' token
    while *i < tokens.len() && if_depth > 0 {
        match tokens[*i] {
            Token::If => if_depth += 1,
            Token::Fi => if_depth -= 1,
            _ => {}
        }
        *i += 1;
    }
}

/// Helper function to skip to the matching 'done' token for a 'for' or 'while' loop.
/// Handles nested loops correctly.
fn skip_to_matching_done(tokens: &[Token], i: &mut usize) {
    let mut loop_depth = 1;
    *i += 1; // Move past the 'for' or 'while' token
    while *i < tokens.len() && loop_depth > 0 {
        match tokens[*i] {
            Token::For | Token::While => loop_depth += 1,
            Token::Done => loop_depth -= 1,
            _ => {}
        }
        *i += 1;
    }
}

/// Helper function to skip to the matching 'esac' token for a 'case' statement.
fn skip_to_matching_esac(tokens: &[Token], i: &mut usize) {
    *i += 1; // Move past the 'case' token
    while *i < tokens.len() {
        if tokens[*i] == Token::Esac {
            *i += 1;
            break;
        }
        *i += 1;
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<Ast, String> {
    // First, try to detect and parse function definitions that span multiple lines
    if tokens.len() >= 4
        && let (Token::Word(_), Token::LeftParen, Token::RightParen, Token::LeftBrace) =
            (&tokens[0], &tokens[1], &tokens[2], &tokens[3])
    {
        // Look for the matching RightBrace
        // Start from the opening brace (token 3) and find its match
        let mut brace_depth = 1; // We've already seen the opening brace at position 3
        let mut function_end = tokens.len();
        let mut j = 4; // Start after the opening brace

        while j < tokens.len() {
            match &tokens[j] {
                Token::LeftBrace => {
                    brace_depth += 1;
                    j += 1;
                }
                Token::RightBrace => {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        function_end = j + 1; // Include the closing brace
                        break;
                    }
                    j += 1;
                }
                Token::If => {
                    // Skip to matching fi to avoid confusion
                    let mut if_depth = 1;
                    j += 1;
                    while j < tokens.len() && if_depth > 0 {
                        match tokens[j] {
                            Token::If => if_depth += 1,
                            Token::Fi => if_depth -= 1,
                            _ => {}
                        }
                        j += 1;
                    }
                }
                Token::For | Token::While => {
                    // Skip to matching done
                    let mut for_depth = 1;
                    j += 1;
                    while j < tokens.len() && for_depth > 0 {
                        match tokens[j] {
                            Token::For | Token::While => for_depth += 1,
                            Token::Done => for_depth -= 1,
                            _ => {}
                        }
                        j += 1;
                    }
                }
                Token::Case => {
                    // Skip to matching esac
                    j += 1;
                    while j < tokens.len() {
                        if tokens[j] == Token::Esac {
                            j += 1;
                            break;
                        }
                        j += 1;
                    }
                }
                _ => {
                    j += 1;
                }
            }
        }

        if brace_depth == 0 && function_end <= tokens.len() {
            // We found the complete function definition
            let function_tokens = &tokens[0..function_end];
            let remaining_tokens = &tokens[function_end..];

            let function_ast = parse_function_definition(function_tokens)?;

            return if remaining_tokens.is_empty() {
                Ok(function_ast)
            } else {
                // There are more commands after the function
                let remaining_ast = parse_commands_sequentially(remaining_tokens)?;
                Ok(Ast::Sequence(vec![function_ast, remaining_ast]))
            };
        }
    }

    // Also check for legacy function definition format (word with parentheses followed by brace)
    if tokens.len() >= 2
        && let Token::Word(ref word) = tokens[0]
        && let Some(paren_pos) = word.find('(')
        && word.ends_with(')')
        && paren_pos > 0
        && tokens[1] == Token::LeftBrace
    {
        return parse_function_definition(&tokens);
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
        if let (Token::Word(var_eq), Token::Word(value)) = (&tokens[0], &tokens[1])
            && let Some(eq_pos) = var_eq.find('=')
            && eq_pos > 0
            && eq_pos < var_eq.len()
        {
            let var = var_eq[..eq_pos].to_string();
            let full_value = format!("{}{}", &var_eq[eq_pos + 1..], value);
            // Basic validation: variable name should start with letter or underscore
            if is_valid_variable_name(&var) {
                return Ok(Ast::Assignment {
                    var,
                    value: full_value,
                });
            }
        }
    }

    // Check if it's an assignment (VAR= VALUE)
    if tokens.len() == 2
        && let (Token::Word(var_eq), Token::Word(value)) = (&tokens[0], &tokens[1])
        && let Some(eq_pos) = var_eq.find('=')
        && eq_pos > 0
        && eq_pos == var_eq.len() - 1
    {
        let var = var_eq[..eq_pos].to_string();
        // Basic validation: variable name should start with letter or underscore
        if is_valid_variable_name(&var) {
            return Ok(Ast::Assignment {
                var,
                value: value.clone(),
            });
        }
    }

    // Check if it's a local assignment (local VAR VALUE or local VAR= VALUE)
    if tokens.len() == 3
        && let (Token::Local, Token::Word(var), Token::Word(value)) =
            (&tokens[0], &tokens[1], &tokens[2])
    {
        // Strip trailing = if present (handles "local var= value" format)
        let clean_var = if var.ends_with('=') {
            &var[..var.len() - 1]
        } else {
            var
        };
        // Basic validation: variable name should start with letter or underscore
        if is_valid_variable_name(clean_var) {
            return Ok(Ast::LocalAssignment {
                var: clean_var.to_string(),
                value: value.clone(),
            });
        }
    }

    // Check if it's a return statement
    if !tokens.is_empty()
        && tokens.len() <= 2
        && let Token::Return = &tokens[0]
    {
        if tokens.len() == 1 {
            // return (with no value, defaults to 0)
            return Ok(Ast::Return { value: None });
        } else if let Token::Word(word) = &tokens[1] {
            // return value
            return Ok(Ast::Return {
                value: Some(word.clone()),
            });
        }
    }

    // Check if it's a local assignment (local VAR=VALUE)
    if tokens.len() == 2
        && let (Token::Local, Token::Word(var_eq)) = (&tokens[0], &tokens[1])
        && let Some(eq_pos) = var_eq.find('=')
        && eq_pos > 0
        && eq_pos < var_eq.len()
    {
        let var = var_eq[..eq_pos].to_string();
        let value = var_eq[eq_pos + 1..].to_string();
        // Basic validation: variable name should start with letter or underscore
        if is_valid_variable_name(&var) {
            return Ok(Ast::LocalAssignment { var, value });
        }
    }

    // Check if it's an assignment (single token with =)
    if tokens.len() == 1
        && let Token::Word(ref word) = tokens[0]
        && let Some(eq_pos) = word.find('=')
        && eq_pos > 0
        && eq_pos < word.len()
    {
        let var = word[..eq_pos].to_string();
        let value = word[eq_pos + 1..].to_string();
        // Basic validation: variable name should start with letter or underscore
        if is_valid_variable_name(&var) {
            return Ok(Ast::Assignment { var, value });
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

    // Check if it's a for loop
    if let Token::For = tokens[0] {
        return parse_for(tokens);
    }

    // Check if it's a while loop
    if let Token::While = tokens[0] {
        return parse_while(tokens);
    }

    // Check if it's a function definition
    // Pattern: Word LeftParen RightParen LeftBrace
    if tokens.len() >= 4
        && let (Token::Word(word), Token::LeftParen, Token::RightParen, Token::LeftBrace) =
            (&tokens[0], &tokens[1], &tokens[2], &tokens[3])
        && is_valid_variable_name(word)
    {
        return parse_function_definition(tokens);
    }

    // Also check for function definition with parentheses in the word (legacy support)
    if tokens.len() >= 2
        && let Token::Word(ref word) = tokens[0]
        && let Some(paren_pos) = word.find('(')
        && word.ends_with(')')
        && paren_pos > 0
    {
        let func_name = &word[..paren_pos];
        if is_valid_variable_name(func_name) && tokens[1] == Token::LeftBrace {
            return parse_function_definition(tokens);
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

        // Check for subshell: LeftParen at start of command
        // Must check BEFORE function definition to avoid ambiguity
        if tokens[i] == Token::LeftParen {
            // This is a subshell - find the matching RightParen
            let mut paren_depth = 1;
            let mut j = i + 1;

            while j < tokens.len() && paren_depth > 0 {
                match tokens[j] {
                    Token::LeftParen => paren_depth += 1,
                    Token::RightParen => paren_depth -= 1,
                    _ => {}
                }
                j += 1;
            }

            if paren_depth != 0 {
                return Err("Unmatched parenthesis in subshell".to_string());
            }

            // Extract subshell body (tokens between parens)
            let subshell_tokens = &tokens[i + 1..j - 1];

            // Parse the subshell body recursively
            // Empty subshells are valid and equivalent to 'true'
            let body_ast = if subshell_tokens.is_empty() {
                create_empty_body_ast()
            } else {
                parse_commands_sequentially(subshell_tokens)?
            };

            let mut subshell_ast = Ast::Subshell {
                body: Box::new(body_ast),
            };

            i = j; // Move past the closing paren

            // Check for redirections after subshell
            let mut redirections = Vec::new();
            while i < tokens.len() {
                match &tokens[i] {
                    Token::RedirOut => {
                        i += 1;
                        if i < tokens.len() {
                            if let Token::Word(file) = &tokens[i] {
                                redirections.push(Redirection::Output(file.clone()));
                                i += 1;
                            }
                        }
                    }
                    Token::RedirIn => {
                        i += 1;
                        if i < tokens.len() {
                            if let Token::Word(file) = &tokens[i] {
                                redirections.push(Redirection::Input(file.clone()));
                                i += 1;
                            }
                        }
                    }
                    Token::RedirAppend => {
                        i += 1;
                        if i < tokens.len() {
                            if let Token::Word(file) = &tokens[i] {
                                redirections.push(Redirection::Append(file.clone()));
                                i += 1;
                            }
                        }
                    }
                    Token::RedirectFdOut(fd, file) => {
                        redirections.push(Redirection::FdOutput(*fd, file.clone()));
                        i += 1;
                    }
                    Token::RedirectFdIn(fd, file) => {
                        redirections.push(Redirection::FdInput(*fd, file.clone()));
                        i += 1;
                    }
                    Token::RedirectFdAppend(fd, file) => {
                        redirections.push(Redirection::FdAppend(*fd, file.clone()));
                        i += 1;
                    }
                    Token::RedirectFdDup(from_fd, to_fd) => {
                        redirections.push(Redirection::FdDuplicate(*from_fd, *to_fd));
                        i += 1;
                    }
                    Token::RedirectFdClose(fd) => {
                        redirections.push(Redirection::FdClose(*fd));
                        i += 1;
                    }
                    Token::RedirectFdInOut(fd, file) => {
                        redirections.push(Redirection::FdInputOutput(*fd, file.clone()));
                        i += 1;
                    }
                    Token::RedirHereDoc(delimiter, quoted) => {
                        redirections
                            .push(Redirection::HereDoc(delimiter.clone(), quoted.to_string()));
                        i += 1;
                    }
                    Token::RedirHereString(content) => {
                        redirections.push(Redirection::HereString(content.clone()));
                        i += 1;
                    }
                    _ => break,
                }
            }

            // If redirections found, wrap subshell in a pipeline with redirections
            if !redirections.is_empty() {
                subshell_ast = Ast::Pipeline(vec![ShellCommand {
                    args: Vec::new(),
                    redirections,
                    compound: Some(Box::new(subshell_ast)),
                }]);
            }

            // Check if this is part of a pipeline
            if i < tokens.len() && tokens[i] == Token::Pipe {
                // This subshell is part of a pipeline - parse the entire line as a pipeline
                let pipeline_ast = parse_pipeline(&tokens[start..])?;
                commands.push(pipeline_ast);
                break; // We've consumed the rest of the tokens
            }

            // Handle operators after subshell (&&, ||, ;, newline)
            if i < tokens.len() && (tokens[i] == Token::And || tokens[i] == Token::Or) {
                let operator = tokens[i].clone();
                i += 1; // Skip the operator

                // Skip any newlines after the operator
                while i < tokens.len() && tokens[i] == Token::Newline {
                    i += 1;
                }

                // Parse the right side recursively
                let remaining_tokens = &tokens[i..];
                let right_ast = parse_commands_sequentially(remaining_tokens)?;

                // Create And or Or node
                let combined_ast = match operator {
                    Token::And => Ast::And {
                        left: Box::new(subshell_ast),
                        right: Box::new(right_ast),
                    },
                    Token::Or => Ast::Or {
                        left: Box::new(subshell_ast),
                        right: Box::new(right_ast),
                    },
                    _ => unreachable!(),
                };

                commands.push(combined_ast);
                break; // We've consumed the rest of the tokens
            } else {
                commands.push(subshell_ast);
            }

            // Skip semicolon or newline after subshell
            if i < tokens.len() && (tokens[i] == Token::Newline || tokens[i] == Token::Semicolon) {
                i += 1;
            }
            continue;
        }

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
        } else if tokens[i] == Token::For {
            // For for loops, find the matching done
            let mut depth = 1; // Start at 1 because we're already inside the for
            i += 1; // Move past the 'for' token
            while i < tokens.len() {
                match tokens[i] {
                    Token::For | Token::While => depth += 1,
                    Token::Done => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1; // Include the done
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        } else if tokens[i] == Token::While {
            // For while loops, find the matching done
            let mut depth = 1; // Start at 1 because we're already inside the while
            i += 1; // Move past the 'while' token
            while i < tokens.len() {
                match tokens[i] {
                    Token::While | Token::For => depth += 1,
                    Token::Done => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1; // Include the done
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
        } else if i + 3 < tokens.len()
            && matches!(tokens[i], Token::Word(_))
            && tokens[i + 1] == Token::LeftParen
            && tokens[i + 2] == Token::RightParen
            && tokens[i + 3] == Token::LeftBrace
        {
            // This is a function definition - find the matching closing brace
            let mut brace_depth = 1;
            i += 4; // Skip to after opening brace
            while i < tokens.len() && brace_depth > 0 {
                match tokens[i] {
                    Token::LeftBrace => brace_depth += 1,
                    Token::RightBrace => brace_depth -= 1,
                    _ => {}
                }
                i += 1;
            }
        } else {
            // For simple commands, stop at newline, semicolon, &&, or ||
            // But check if the next token after newline is a control flow keyword
            while i < tokens.len() {
                if tokens[i] == Token::Newline
                    || tokens[i] == Token::Semicolon
                    || tokens[i] == Token::And
                    || tokens[i] == Token::Or
                {
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

            // Check if the next token is && or ||
            if i < tokens.len() && (tokens[i] == Token::And || tokens[i] == Token::Or) {
                let operator = tokens[i].clone();
                i += 1; // Skip the operator

                // Skip any newlines after the operator
                while i < tokens.len() && tokens[i] == Token::Newline {
                    i += 1;
                }

                // Parse the right side recursively
                let remaining_tokens = &tokens[i..];
                let right_ast = parse_commands_sequentially(remaining_tokens)?;

                // Create And or Or node
                let combined_ast = match operator {
                    Token::And => Ast::And {
                        left: Box::new(ast),
                        right: Box::new(right_ast),
                    },
                    Token::Or => Ast::Or {
                        left: Box::new(ast),
                        right: Box::new(right_ast),
                    },
                    _ => unreachable!(),
                };

                commands.push(combined_ast);
                break; // We've consumed the rest of the tokens
            } else {
                commands.push(ast);
            }
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
    let mut current_cmd = ShellCommand::default();

    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        match token {
            Token::LeftParen => {
                // Start of subshell in pipeline
                // Find matching RightParen
                let mut paren_depth = 1;
                let mut j = i + 1;

                while j < tokens.len() && paren_depth > 0 {
                    match tokens[j] {
                        Token::LeftParen => paren_depth += 1,
                        Token::RightParen => paren_depth -= 1,
                        _ => {}
                    }
                    j += 1;
                }

                if paren_depth != 0 {
                    return Err("Unmatched parenthesis in pipeline".to_string());
                }

                // Parse subshell body
                let subshell_tokens = &tokens[i + 1..j - 1];

                // Empty subshells are valid and equivalent to 'true'
                let body_ast = if subshell_tokens.is_empty() {
                    create_empty_body_ast()
                } else {
                    parse_commands_sequentially(subshell_tokens)?
                };

                // Create ShellCommand with compound subshell
                current_cmd.compound = Some(Box::new(Ast::Subshell {
                    body: Box::new(body_ast),
                }));

                i = j; // Move past closing paren

                // Check for redirections after subshell
                while i < tokens.len() {
                    match &tokens[i] {
                        Token::RedirOut => {
                            i += 1;
                            if i < tokens.len() {
                                if let Token::Word(file) = &tokens[i] {
                                    current_cmd
                                        .redirections
                                        .push(Redirection::Output(file.clone()));
                                    i += 1;
                                }
                            }
                        }
                        Token::RedirIn => {
                            i += 1;
                            if i < tokens.len() {
                                if let Token::Word(file) = &tokens[i] {
                                    current_cmd
                                        .redirections
                                        .push(Redirection::Input(file.clone()));
                                    i += 1;
                                }
                            }
                        }
                        Token::RedirAppend => {
                            i += 1;
                            if i < tokens.len() {
                                if let Token::Word(file) = &tokens[i] {
                                    current_cmd
                                        .redirections
                                        .push(Redirection::Append(file.clone()));
                                    i += 1;
                                }
                            }
                        }
                        Token::RedirectFdOut(fd, file) => {
                            current_cmd
                                .redirections
                                .push(Redirection::FdOutput(*fd, file.clone()));
                            i += 1;
                        }
                        Token::RedirectFdIn(fd, file) => {
                            current_cmd
                                .redirections
                                .push(Redirection::FdInput(*fd, file.clone()));
                            i += 1;
                        }
                        Token::RedirectFdAppend(fd, file) => {
                            current_cmd
                                .redirections
                                .push(Redirection::FdAppend(*fd, file.clone()));
                            i += 1;
                        }
                        Token::RedirectFdDup(from_fd, to_fd) => {
                            current_cmd
                                .redirections
                                .push(Redirection::FdDuplicate(*from_fd, *to_fd));
                            i += 1;
                        }
                        Token::RedirectFdClose(fd) => {
                            current_cmd.redirections.push(Redirection::FdClose(*fd));
                            i += 1;
                        }
                        Token::RedirectFdInOut(fd, file) => {
                            current_cmd
                                .redirections
                                .push(Redirection::FdInputOutput(*fd, file.clone()));
                            i += 1;
                        }
                        Token::RedirHereDoc(delimiter, quoted) => {
                            current_cmd
                                .redirections
                                .push(Redirection::HereDoc(delimiter.clone(), quoted.to_string()));
                            i += 1;
                        }
                        Token::RedirHereString(content) => {
                            current_cmd
                                .redirections
                                .push(Redirection::HereString(content.clone()));
                            i += 1;
                        }
                        Token::Pipe => {
                            // End of this pipeline stage
                            break;
                        }
                        _ => break,
                    }
                }

                // Push the command with subshell
                commands.push(current_cmd.clone());
                current_cmd = ShellCommand::default();

                continue;
            }
            Token::Word(word) => {
                current_cmd.args.push(word.clone());
            }
            Token::Pipe => {
                if !current_cmd.args.is_empty() || current_cmd.compound.is_some() {
                    commands.push(current_cmd.clone());
                    current_cmd = ShellCommand::default();
                }
            }
            // Basic redirections (backward compatible)
            Token::RedirIn => {
                i += 1;
                if i < tokens.len()
                    && let Token::Word(ref file) = tokens[i]
                {
                    current_cmd
                        .redirections
                        .push(Redirection::Input(file.clone()));
                }
            }
            Token::RedirOut => {
                i += 1;
                if i < tokens.len()
                    && let Token::Word(ref file) = tokens[i]
                {
                    current_cmd
                        .redirections
                        .push(Redirection::Output(file.clone()));
                }
            }
            Token::RedirAppend => {
                i += 1;
                if i < tokens.len()
                    && let Token::Word(ref file) = tokens[i]
                {
                    current_cmd
                        .redirections
                        .push(Redirection::Append(file.clone()));
                }
            }
            Token::RedirHereDoc(delimiter, quoted) => {
                // Store delimiter and quoted flag - content will be read by executor
                current_cmd
                    .redirections
                    .push(Redirection::HereDoc(delimiter.clone(), quoted.to_string()));
            }
            Token::RedirHereString(content) => {
                current_cmd
                    .redirections
                    .push(Redirection::HereString(content.clone()));
            }
            // File descriptor redirections
            Token::RedirectFdIn(fd, file) => {
                current_cmd
                    .redirections
                    .push(Redirection::FdInput(*fd, file.clone()));
            }
            Token::RedirectFdOut(fd, file) => {
                current_cmd
                    .redirections
                    .push(Redirection::FdOutput(*fd, file.clone()));
            }
            Token::RedirectFdAppend(fd, file) => {
                current_cmd
                    .redirections
                    .push(Redirection::FdAppend(*fd, file.clone()));
            }
            Token::RedirectFdDup(from_fd, to_fd) => {
                current_cmd
                    .redirections
                    .push(Redirection::FdDuplicate(*from_fd, *to_fd));
            }
            Token::RedirectFdClose(fd) => {
                current_cmd.redirections.push(Redirection::FdClose(*fd));
            }
            Token::RedirectFdInOut(fd, file) => {
                current_cmd
                    .redirections
                    .push(Redirection::FdInputOutput(*fd, file.clone()));
            }
            Token::RightParen => {
                // Check if this looks like a function call pattern: Word LeftParen ... RightParen
                // If so, treat it as a function call even if the function doesn't exist
                if !current_cmd.args.is_empty()
                    && i > 0
                    && let Token::LeftParen = tokens[i - 1]
                {
                    // This looks like a function call pattern, treat as function call
                    // For now, we'll handle this in the executor by checking if it's a function
                    // If not a function, the executor will handle the error gracefully
                    break;
                }
                return Err("Unexpected ) in pipeline".to_string());
            }
            Token::Newline => {
                // Newlines are handled at the sequence level, skip them in pipelines
                i += 1;
                continue;
            }
            Token::Do
            | Token::Done
            | Token::Then
            | Token::Else
            | Token::Elif
            | Token::Fi
            | Token::Esac => {
                // These are control flow keywords that should be handled at a higher level
                // If we encounter them here, it means we've reached the end of the current command
                break;
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
        skip_newlines(tokens, &mut i);

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
        skip_newlines(tokens, &mut i);

        let then_ast = if then_tokens.is_empty() {
            // Empty then branch - create a no-op
            create_empty_body_ast()
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
            create_empty_body_ast()
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

fn parse_for(tokens: &[Token]) -> Result<Ast, String> {
    let mut i = 1; // Skip 'for'

    // Parse variable name
    if i >= tokens.len() || !matches!(tokens[i], Token::Word(_)) {
        return Err("Expected variable name after for".to_string());
    }
    let variable = if let Token::Word(ref v) = tokens[i] {
        v.clone()
    } else {
        unreachable!()
    };
    i += 1;

    // Expect 'in'
    if i >= tokens.len() || tokens[i] != Token::In {
        return Err("Expected 'in' after for variable".to_string());
    }
    i += 1;

    // Parse items until we hit 'do' or semicolon/newline
    let mut items = Vec::new();
    while i < tokens.len() {
        match &tokens[i] {
            Token::Do => break,
            Token::Semicolon | Token::Newline => {
                i += 1;
                // Check if next token is 'do'
                if i < tokens.len() && tokens[i] == Token::Do {
                    break;
                }
            }
            Token::Word(word) => {
                items.push(word.clone());
                i += 1;
            }
            _ => {
                return Err(format!("Unexpected token in for items: {:?}", tokens[i]));
            }
        }
    }

    // Skip any newlines before 'do'
    while i < tokens.len() && tokens[i] == Token::Newline {
        i += 1;
    }

    // Expect 'do'
    if i >= tokens.len() || tokens[i] != Token::Do {
        return Err("Expected 'do' in for loop".to_string());
    }
    i += 1;

    // Skip any newlines after 'do'
    while i < tokens.len() && tokens[i] == Token::Newline {
        i += 1;
    }

    // Parse body until 'done'
    let mut body_tokens = Vec::new();
    let mut depth = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::For => {
                depth += 1;
                body_tokens.push(tokens[i].clone());
            }
            Token::Done => {
                if depth > 0 {
                    depth -= 1;
                    body_tokens.push(tokens[i].clone());
                } else {
                    break; // This done closes our for loop
                }
            }
            Token::Newline => {
                // Skip newlines but check what comes after
                let mut j = i + 1;
                while j < tokens.len() && tokens[j] == Token::Newline {
                    j += 1;
                }
                if j < tokens.len() && depth == 0 && tokens[j] == Token::Done {
                    i = j; // Skip to done
                    break;
                }
                // Otherwise it's just a newline in the middle of commands
                body_tokens.push(tokens[i].clone());
            }
            _ => {
                body_tokens.push(tokens[i].clone());
            }
        }
        i += 1;
    }

    if i >= tokens.len() || tokens[i] != Token::Done {
        return Err("Expected 'done' to close for loop".to_string());
    }

    // Parse the body
    let body_ast = if body_tokens.is_empty() {
        // Empty body - create a no-op
        create_empty_body_ast()
    } else {
        parse_commands_sequentially(&body_tokens)?
    };

    Ok(Ast::For {
        variable,
        items,
        body: Box::new(body_ast),
    })
}

fn parse_while(tokens: &[Token]) -> Result<Ast, String> {
    let mut i = 1; // Skip 'while'

    // Parse condition until we hit 'do' or semicolon/newline
    let mut cond_tokens = Vec::new();
    while i < tokens.len() {
        match &tokens[i] {
            Token::Do => break,
            Token::Semicolon | Token::Newline => {
                i += 1;
                // Check if next token is 'do'
                if i < tokens.len() && tokens[i] == Token::Do {
                    break;
                }
            }
            _ => {
                cond_tokens.push(tokens[i].clone());
                i += 1;
            }
        }
    }

    if cond_tokens.is_empty() {
        return Err("Expected condition after while".to_string());
    }

    // Skip any newlines before 'do'
    while i < tokens.len() && tokens[i] == Token::Newline {
        i += 1;
    }

    // Expect 'do'
    if i >= tokens.len() || tokens[i] != Token::Do {
        return Err("Expected 'do' in while loop".to_string());
    }
    i += 1;

    // Skip any newlines after 'do'
    while i < tokens.len() && tokens[i] == Token::Newline {
        i += 1;
    }

    // Parse body until 'done'
    let mut body_tokens = Vec::new();
    let mut depth = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::While | Token::For => {
                depth += 1;
                body_tokens.push(tokens[i].clone());
            }
            Token::Done => {
                if depth > 0 {
                    depth -= 1;
                    body_tokens.push(tokens[i].clone());
                } else {
                    break; // This done closes our while loop
                }
            }
            Token::Newline => {
                // Skip newlines but check what comes after
                let mut j = i + 1;
                while j < tokens.len() && tokens[j] == Token::Newline {
                    j += 1;
                }
                if j < tokens.len() && depth == 0 && tokens[j] == Token::Done {
                    i = j; // Skip to done
                    break;
                }
                // Otherwise it's just a newline in the middle of commands
                body_tokens.push(tokens[i].clone());
            }
            _ => {
                body_tokens.push(tokens[i].clone());
            }
        }
        i += 1;
    }

    if i >= tokens.len() || tokens[i] != Token::Done {
        return Err("Expected 'done' to close while loop".to_string());
    }

    // Parse the condition
    let condition_ast = parse_slice(&cond_tokens)?;

    // Parse the body
    let body_ast = if body_tokens.is_empty() {
        // Empty body - create a no-op
        create_empty_body_ast()
    } else {
        parse_commands_sequentially(&body_tokens)?
    };

    Ok(Ast::While {
        condition: Box::new(condition_ast),
        body: Box::new(body_ast),
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
    let brace_pos =
        if tokens.len() >= 4 && tokens[1] == Token::LeftParen && tokens[2] == Token::RightParen {
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

    // Find the matching closing brace, accounting for nested function definitions and control structures
    let mut brace_depth = 0;
    let mut body_end = 0;
    let mut found_closing = false;
    let mut i = brace_pos + 1;

    while i < tokens.len() {
        // Check if this is the start of a nested function definition
        // Pattern: Word LeftParen RightParen LeftBrace
        if i + 3 < tokens.len()
            && matches!(&tokens[i], Token::Word(_))
            && tokens[i + 1] == Token::LeftParen
            && tokens[i + 2] == Token::RightParen
            && tokens[i + 3] == Token::LeftBrace
        {
            // This is a nested function - skip over it entirely
            // Skip to after the opening brace of nested function
            i += 4;
            let mut nested_depth = 1;
            while i < tokens.len() && nested_depth > 0 {
                match tokens[i] {
                    Token::LeftBrace => nested_depth += 1,
                    Token::RightBrace => nested_depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            // Don't increment i again - continue from current position
            continue;
        }

        match &tokens[i] {
            Token::LeftBrace => {
                brace_depth += 1;
                i += 1;
            }
            Token::RightBrace => {
                if brace_depth == 0 {
                    // This is our matching closing brace
                    body_end = i;
                    found_closing = true;
                    break;
                } else {
                    brace_depth -= 1;
                    i += 1;
                }
            }
            Token::If => {
                // Skip to matching fi
                skip_to_matching_fi(tokens, &mut i);
            }
            Token::For | Token::While => {
                // Skip to matching done
                skip_to_matching_done(tokens, &mut i);
            }
            Token::Case => {
                // Skip to matching esac
                skip_to_matching_esac(tokens, &mut i);
            }
            _ => {
                i += 1;
            }
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
        create_empty_body_ast()
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
}
