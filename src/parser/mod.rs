//! Parser module for the Rush shell.
//!
//! This module provides functionality to parse tokenized shell input into an Abstract
//! Syntax Tree (AST) that can be executed by the executor module.

pub mod ast;
mod control_flow;

pub use ast::{Ast, Redirection, ShellCommand};
use control_flow::*;

use super::lexer::Token;

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
pub(crate) fn create_empty_body_ast() -> Ast {
    Ast::Pipeline(vec![ShellCommand {
        args: vec!["true".to_string()],
        redirections: Vec::new(),
        compound: None,
    }])
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
                Token::For | Token::While | Token::Until => {
                    // Skip to matching done
                    let mut for_depth = 1;
                    j += 1;
                    while j < tokens.len() && for_depth > 0 {
                        match tokens[j] {
                            Token::For | Token::While | Token::Until => for_depth += 1,
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

/// Parses a single top-level command slice into an AST.
///
/// Recognizes assignments, local assignments, `return`, negation (`!`), control constructs
/// (`if`, `case`, `for`, `while`, `until`), function definitions, and otherwise falls back to
/// pipeline parsing to produce an `Ast` for the provided token slice.
///
/// # Returns
///
/// `Ok(Ast)` on success, `Err(String)` with a descriptive error message on failure (for example,
/// when the slice is empty or a `!` is not followed by a command).
///
/// # Examples
///
/// ```
/// // Note: parse_slice is a private function
/// // This example is for documentation only
/// ```
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
        } else {
            return Err(format!("Invalid variable name: {}", clean_var));
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
        } else {
            return Err(format!("Invalid variable name: {}", var));
        }
    }

    // Check if it's a local assignment (local VAR) with no initial value
    if tokens.len() == 2
        && let (Token::Local, Token::Word(var)) = (&tokens[0], &tokens[1])
        && !var.contains('=')
    {
        // Basic validation: variable name should start with letter or underscore
        if is_valid_variable_name(var) {
            return Ok(Ast::LocalAssignment {
                var: var.clone(),
                value: String::new(),
            });
        } else {
            return Err(format!("Invalid variable name: {}", var));
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

    // Check if it's an until loop
    if let Token::Until = tokens[0] {
        return parse_until(tokens);
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

/// Helper function to parse a single command without operators.
/// Returns the parsed AST and the number of tokens consumed.
fn parse_single_command(tokens: &[Token]) -> Result<(Ast, usize), String> {
    if tokens.is_empty() {
        return Err("Expected command".to_string());
    }

    let mut i = 0;

    // Skip leading newlines
    while i < tokens.len() && tokens[i] == Token::Newline {
        i += 1;
    }

    if i >= tokens.len() {
        return Err("Expected command".to_string());
    }

    // Handle negation first - recursively parse what comes after !
    if tokens[i] == Token::Bang {
        i += 1; // Skip the bang

        // Skip any newlines after the bang
        while i < tokens.len() && tokens[i] == Token::Newline {
            i += 1;
        }

        if i >= tokens.len() {
            return Err("Expected command after !".to_string());
        }

        // Recursively parse the negated command
        // IMPORTANT: This should only consume a single atomic command,
        // not a chain with logical operators
        let (negated_ast, consumed) = parse_single_command(&tokens[i..])?;
        i += consumed;

        // Negation only applies to the immediately following command
        // Return immediately without consuming any following operators
        return Ok((
            Ast::Negation {
                command: Box::new(negated_ast),
            },
            i,
        ));
    }

    let start = i;

    // Handle special constructs that have their own boundaries
    match &tokens[i] {
        Token::LeftParen => {
            // Subshell - find matching paren
            let mut paren_depth = 1;
            i += 1;
            while i < tokens.len() && paren_depth > 0 {
                match tokens[i] {
                    Token::LeftParen => paren_depth += 1,
                    Token::RightParen => paren_depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            if paren_depth != 0 {
                return Err("Unmatched parenthesis".to_string());
            }
        }
        Token::LeftBrace => {
            // Command group - find matching brace
            let mut brace_depth = 1;
            i += 1;
            while i < tokens.len() && brace_depth > 0 {
                match tokens[i] {
                    Token::LeftBrace => brace_depth += 1,
                    Token::RightBrace => brace_depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            if brace_depth != 0 {
                return Err("Unmatched brace".to_string());
            }
        }
        Token::If => {
            // Find matching fi
            let mut if_depth = 1;
            i += 1;
            while i < tokens.len() && if_depth > 0 {
                match tokens[i] {
                    Token::If => if_depth += 1,
                    Token::Fi => {
                        if_depth -= 1;
                        if if_depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
        Token::For | Token::While | Token::Until => {
            // Find matching done
            let mut loop_depth = 1;
            i += 1;
            while i < tokens.len() && loop_depth > 0 {
                match tokens[i] {
                    Token::For | Token::While | Token::Until => loop_depth += 1,
                    Token::Done => {
                        loop_depth -= 1;
                        if loop_depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
        Token::Case => {
            // Find matching esac
            i += 1;
            while i < tokens.len() {
                if tokens[i] == Token::Esac {
                    i += 1;
                    break;
                }
                i += 1;
            }
        }
        _ => {
            // Regular command/pipeline - stop at sequence separators
            let mut brace_depth = 0;
            let mut paren_depth = 0;
            let mut last_was_pipe = false;

            while i < tokens.len() {
                // Check if we should stop before processing this token
                // For logical operators (&&, ||), always stop at depth 0 after consuming at least one token
                // For other separators, only stop after consuming at least one token
                if i > start {
                    match &tokens[i] {
                        Token::And | Token::Or => {
                            if brace_depth == 0 && paren_depth == 0 {
                                if last_was_pipe {
                                    return Err("Expected command after |".to_string());
                                }
                                break;
                            }
                        }
                        Token::Newline | Token::Semicolon | Token::Ampersand => {
                            if brace_depth == 0 && paren_depth == 0 && !last_was_pipe {
                                break;
                            }
                        }
                        Token::RightBrace if brace_depth == 0 => break,
                        Token::RightParen if paren_depth == 0 => break,
                        _ => {}
                    }
                }

                // Now process the token
                match &tokens[i] {
                    Token::LeftBrace => {
                        brace_depth += 1;
                        last_was_pipe = false;
                    }
                    Token::RightBrace => {
                        if brace_depth > 0 {
                            brace_depth -= 1;
                            last_was_pipe = false;
                        }
                    }
                    Token::LeftParen => {
                        paren_depth += 1;
                        last_was_pipe = false;
                    }
                    Token::RightParen => {
                        if paren_depth > 0 {
                            paren_depth -= 1;
                            last_was_pipe = false;
                        }
                    }
                    Token::Pipe => last_was_pipe = true,
                    Token::Word(_) => last_was_pipe = false,
                    _ => last_was_pipe = false,
                }
                i += 1;
            }
        }
    }

    let command_tokens = &tokens[start..i];

    // Safety check: ensure we consumed at least one token to prevent infinite loops
    if i == start {
        return Err("Internal parser error: parse_single_command consumed no tokens".to_string());
    }

    let mut ast = parse_slice(command_tokens)?;
    
    // Check if this command should be executed asynchronously (ends with &)
    if i < tokens.len() && tokens[i] == Token::Ampersand {
        i += 1; // Consume the &
        ast = Ast::AsyncCommand {
            command: Box::new(ast),
        };
    }
    
    Ok((ast, i))
}

/// Helper function to parse commands with && and || operators.
/// This builds a left-associative chain of operators.
/// Returns the parsed AST and the number of tokens consumed.
fn parse_next_command(tokens: &[Token]) -> Result<(Ast, usize), String> {
    // Parse the first command
    let (mut ast, mut i) = parse_single_command(tokens)?;

    // Build left-associative chain of && and || operators iteratively
    loop {
        // Check if there's an && or || operator after this command
        if i >= tokens.len() || (tokens[i] != Token::And && tokens[i] != Token::Or) {
            break;
        }

        let operator = tokens[i].clone();
        i += 1; // Skip the operator

        // Skip any newlines after the operator
        while i < tokens.len() && tokens[i] == Token::Newline {
            i += 1;
        }

        if i >= tokens.len() {
            return Err("Expected command after operator".to_string());
        }

        // Parse the next single command (without chaining)
        let (right_ast, consumed) = parse_single_command(&tokens[i..])?;
        i += consumed;

        // Build left-associative structure
        ast = match operator {
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
    }

    Ok((ast, i))
}

/// Parses a slice of tokens into a top-level AST representing one or more sequential shell commands.
///
/// This function consumes the provided token sequence and produces an `Ast` that represents either a
/// single command/pipeline/compound construct or a `Sequence` of commands joined by semicolons/newlines
/// and conditional operators. It recognizes subshells, command groups, pipelines, redirections,
/// negation (`!`), function definitions, and control-flow blocks, and composes appropriate AST nodes.
///
/// # Errors
///
/// Returns an `Err(String)` when the tokens contain a syntactic problem that prevents building a valid AST,
/// for example unmatched braces/parentheses, an empty subshell or command group, or when no commands are present.
///
/// # Examples
///
/// ```
/// // Note: parse_commands_sequentially is a private function
/// // This example is for documentation only
/// ```
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
            // Empty subshells are not allowed
            let body_ast = if subshell_tokens.is_empty() {
                return Err("Empty subshell".to_string());
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
                        if i < tokens.len()
                            && let Token::Word(file) = &tokens[i]
                        {
                            redirections.push(Redirection::Output(file.clone()));
                            i += 1;
                        }
                    }
                    Token::RedirOutClobber => {
                        i += 1;
                        if i >= tokens.len() {
                            return Err("expected filename after >|".to_string());
                        }
                        if let Token::Word(file) = &tokens[i] {
                            redirections.push(Redirection::OutputClobber(file.clone()));
                            i += 1;
                        } else {
                            return Err("expected filename after >|".to_string());
                        }
                    }
                    Token::RedirIn => {
                        i += 1;
                        if i < tokens.len()
                            && let Token::Word(file) = &tokens[i]
                        {
                            redirections.push(Redirection::Input(file.clone()));
                            i += 1;
                        }
                    }
                    Token::RedirAppend => {
                        i += 1;
                        if i < tokens.len()
                            && let Token::Word(file) = &tokens[i]
                        {
                            redirections.push(Redirection::Append(file.clone()));
                            i += 1;
                        }
                    }
                    Token::RedirectFdOut(fd, file) => {
                        redirections.push(Redirection::FdOutput(*fd, file.clone()));
                        i += 1;
                    }
                    Token::RedirectFdOutClobber(fd, file) => {
                        redirections.push(Redirection::FdOutputClobber(*fd, file.clone()));
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

            // Check if this subshell is part of a pipeline
            if i < tokens.len() && tokens[i] == Token::Pipe {
                // Find end of pipeline
                let mut end = i;
                let mut brace_depth = 0;
                let mut paren_depth = 0;
                let mut last_was_pipe = true; // Started with a pipe
                while end < tokens.len() {
                    match &tokens[end] {
                        Token::Pipe => last_was_pipe = true,
                        Token::LeftBrace => {
                            brace_depth += 1;
                            last_was_pipe = false;
                        }
                        Token::RightBrace => {
                            if brace_depth > 0 {
                                brace_depth -= 1;
                            } else {
                                break;
                            }
                            last_was_pipe = false;
                        }
                        Token::LeftParen => {
                            paren_depth += 1;
                            last_was_pipe = false;
                        }
                        Token::RightParen => {
                            if paren_depth > 0 {
                                paren_depth -= 1;
                            } else {
                                break;
                            }
                            last_was_pipe = false;
                        }
                        Token::Newline | Token::Semicolon => {
                            if brace_depth == 0 && paren_depth == 0 && !last_was_pipe {
                                break;
                            }
                        }
                        Token::Word(_) => last_was_pipe = false,
                        _ => {}
                    }
                    end += 1;
                }

                let pipeline_ast = parse_pipeline(&tokens[start..end])?;
                commands.push(pipeline_ast);
                i = end;
                continue;
            }

            // If not part of a pipeline, apply redirections to the subshell itself
            if !redirections.is_empty() {
                subshell_ast = Ast::Pipeline(vec![ShellCommand {
                    args: Vec::new(),
                    redirections,
                    compound: Some(Box::new(subshell_ast)),
                }]);
            }

            // Handle operators after subshell (&&, ||, ;, newline)
            if i < tokens.len() && (tokens[i] == Token::And || tokens[i] == Token::Or) {
                let operator = tokens[i].clone();
                i += 1; // Skip the operator

                // Skip any newlines after the operator
                while i < tokens.len() && tokens[i] == Token::Newline {
                    i += 1;
                }

                // Parse only the next command (not the entire remaining sequence)
                let (right_ast, consumed) = parse_next_command(&tokens[i..])?;
                i += consumed;

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

                // Skip semicolon or newline after the combined command
                if i < tokens.len()
                    && (tokens[i] == Token::Newline || tokens[i] == Token::Semicolon)
                {
                    i += 1;
                }
                continue;
            } else {
                commands.push(subshell_ast);
            }

            // Skip semicolon or newline after subshell
            if i < tokens.len() && (tokens[i] == Token::Newline || tokens[i] == Token::Semicolon) {
                i += 1;
            }
            continue;
        }

        // Check for command group: LeftBrace at start of command
        if tokens[i] == Token::LeftBrace {
            // This is a command group - find the matching RightBrace
            let mut brace_depth = 1;
            let mut j = i + 1;

            while j < tokens.len() && brace_depth > 0 {
                match tokens[j] {
                    Token::LeftBrace => brace_depth += 1,
                    Token::RightBrace => brace_depth -= 1,
                    _ => {}
                }
                j += 1;
            }

            if brace_depth != 0 {
                return Err("Unmatched brace in command group".to_string());
            }

            // Extract group body (tokens between braces)
            let group_tokens = &tokens[i + 1..j - 1];

            // Parse the group body recursively
            // Empty groups are not allowed
            let body_ast = if group_tokens.is_empty() {
                return Err("Empty command group".to_string());
            } else {
                parse_commands_sequentially(group_tokens)?
            };

            let mut group_ast = Ast::CommandGroup {
                body: Box::new(body_ast),
            };

            i = j; // Move past the closing brace

            // Check for redirections after command group
            let mut redirections = Vec::new();
            while i < tokens.len() {
                match &tokens[i] {
                    Token::RedirOut => {
                        i += 1;
                        if i < tokens.len()
                            && let Token::Word(file) = &tokens[i]
                        {
                            redirections.push(Redirection::Output(file.clone()));
                            i += 1;
                        }
                    }
                    Token::RedirOutClobber => {
                        i += 1;
                        if i >= tokens.len() {
                            return Err("expected filename after >|".to_string());
                        }
                        if let Token::Word(file) = &tokens[i] {
                            redirections.push(Redirection::OutputClobber(file.clone()));
                            i += 1;
                        } else {
                            return Err("expected filename after >|".to_string());
                        }
                    }
                    Token::RedirIn => {
                        i += 1;
                        if i < tokens.len()
                            && let Token::Word(file) = &tokens[i]
                        {
                            redirections.push(Redirection::Input(file.clone()));
                            i += 1;
                        }
                    }
                    Token::RedirAppend => {
                        i += 1;
                        if i < tokens.len()
                            && let Token::Word(file) = &tokens[i]
                        {
                            redirections.push(Redirection::Append(file.clone()));
                            i += 1;
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

            // Check if this group is part of a pipeline
            if i < tokens.len() && tokens[i] == Token::Pipe {
                // Find end of pipeline
                let mut end = i;
                let mut brace_depth = 0;
                let mut paren_depth = 0;
                let mut last_was_pipe = true; // Started with a pipe
                while end < tokens.len() {
                    match &tokens[end] {
                        Token::Pipe => last_was_pipe = true,
                        Token::LeftBrace => {
                            brace_depth += 1;
                            last_was_pipe = false;
                        }
                        Token::RightBrace => {
                            if brace_depth > 0 {
                                brace_depth -= 1;
                            } else {
                                break;
                            }
                            last_was_pipe = false;
                        }
                        Token::LeftParen => {
                            paren_depth += 1;
                            last_was_pipe = false;
                        }
                        Token::RightParen => {
                            if paren_depth > 0 {
                                paren_depth -= 1;
                            } else {
                                break;
                            }
                            last_was_pipe = false;
                        }
                        Token::Newline | Token::Semicolon => {
                            if brace_depth == 0 && paren_depth == 0 && !last_was_pipe {
                                break;
                            }
                        }
                        Token::Word(_) => last_was_pipe = false,
                        _ => {}
                    }
                    end += 1;
                }

                let pipeline_ast = parse_pipeline(&tokens[start..end])?;
                commands.push(pipeline_ast);
                i = end;
                continue;
            }

            // If not part of a pipeline, apply redirections to the group itself
            if !redirections.is_empty() {
                group_ast = Ast::Pipeline(vec![ShellCommand {
                    args: Vec::new(),
                    redirections,
                    compound: Some(Box::new(group_ast)),
                }]);
            }

            // Handle operators after group (&&, ||, ;, newline)
            if i < tokens.len() && (tokens[i] == Token::And || tokens[i] == Token::Or) {
                let operator = tokens[i].clone();
                i += 1; // Skip the operator

                // Skip any newlines after the operator
                while i < tokens.len() && tokens[i] == Token::Newline {
                    i += 1;
                }

                // Parse only the next command (not the entire remaining sequence)
                let (right_ast, consumed) = parse_next_command(&tokens[i..])?;
                i += consumed;

                // Create And or Or node
                let combined_ast = match operator {
                    Token::And => Ast::And {
                        left: Box::new(group_ast),
                        right: Box::new(right_ast),
                    },
                    Token::Or => Ast::Or {
                        left: Box::new(group_ast),
                        right: Box::new(right_ast),
                    },
                    _ => unreachable!(),
                };

                commands.push(combined_ast);

                // Skip semicolon or newline after the combined command
                if i < tokens.len()
                    && (tokens[i] == Token::Newline || tokens[i] == Token::Semicolon)
                {
                    i += 1;
                }
                continue;
            } else {
                commands.push(group_ast);
            }

            // Skip semicolon or newline after group
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
                    Token::For | Token::While | Token::Until => depth += 1,
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
                    Token::While | Token::For | Token::Until => depth += 1,
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
        } else if tokens[i] == Token::Until {
            // For until loops, find the matching done
            let mut depth = 1; // Start at 1 because we're already inside the until
            i += 1; // Move past the 'until' token
            while i < tokens.len() {
                match tokens[i] {
                    Token::Until | Token::For | Token::While => depth += 1,
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
            // Sanity check: we shouldn't be starting a command with an operator
            if matches!(tokens[i], Token::And | Token::Or | Token::Semicolon) {
                return Err(format!(
                    "Unexpected operator at command start: {:?}",
                    tokens[i]
                ));
            }

            // For simple commands, stop at newline, semicolon, &&, or ||
            // But check if the next token after newline is a control flow keyword
            let mut brace_depth = 0;
            let mut paren_depth = 0;
            let mut last_was_pipe = false;
            while i < tokens.len() {
                match &tokens[i] {
                    Token::LeftBrace => {
                        brace_depth += 1;
                        last_was_pipe = false;
                    }
                    Token::RightBrace => {
                        if brace_depth > 0 {
                            brace_depth -= 1;
                        } else {
                            break;
                        }
                        last_was_pipe = false;
                    }
                    Token::LeftParen => {
                        paren_depth += 1;
                        last_was_pipe = false;
                    }
                    Token::RightParen => {
                        if paren_depth > 0 {
                            paren_depth -= 1;
                        } else {
                            break;
                        }
                        last_was_pipe = false;
                    }
                    Token::Pipe => last_was_pipe = true,
                    Token::Newline | Token::Semicolon | Token::And | Token::Or | Token::Ampersand => {
                        if brace_depth == 0 && paren_depth == 0 && !last_was_pipe {
                            break;
                        }
                    }
                    Token::Word(_) => last_was_pipe = false,
                    _ => {}
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

            // Use parse_next_command to handle operators
            let (mut ast, consumed) = parse_next_command(&tokens[start..])?;
            i = start + consumed;

            // Check if this command should be executed asynchronously (ends with &)
            if i < tokens.len() && tokens[i] == Token::Ampersand {
                i += 1; // Consume the &
                ast = Ast::AsyncCommand {
                    command: Box::new(ast),
                };
            }

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

/// Parses a sequence of tokens into an `Ast::Pipeline` representing one or more pipeline stages.
///
/// The resulting pipeline contains one `ShellCommand` per stage with collected `args`,
/// ordered `redirections`, and an optional `compound` (subshell or command group). Returns an
/// error if the tokens contain unmatched braces/parentheses, an unexpected token, or no commands.
///
/// # Examples
///
/// ```
/// // Note: parse_pipeline is a private function
/// // This example is for documentation only
/// ```
fn parse_pipeline(tokens: &[Token]) -> Result<Ast, String> {
    let mut commands = Vec::new();
    let mut current_cmd = ShellCommand::default();

    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        match token {
            Token::LeftBrace => {
                // Start of command group in pipeline
                // Find matching RightBrace
                let mut brace_depth = 1;
                let mut j = i + 1;

                while j < tokens.len() && brace_depth > 0 {
                    match tokens[j] {
                        Token::LeftBrace => brace_depth += 1,
                        Token::RightBrace => brace_depth -= 1,
                        _ => {}
                    }
                    j += 1;
                }

                if brace_depth != 0 {
                    return Err("Unmatched brace in pipeline".to_string());
                }

                // Parse group body
                let group_tokens = &tokens[i + 1..j - 1];

                // Empty groups are valid and equivalent to 'true'
                let body_ast = if group_tokens.is_empty() {
                    create_empty_body_ast()
                } else {
                    parse_commands_sequentially(group_tokens)?
                };

                // Create ShellCommand with compound command group
                current_cmd.compound = Some(Box::new(Ast::CommandGroup {
                    body: Box::new(body_ast),
                }));

                i = j; // Move past closing brace

                // Check for redirections after command group
                while i < tokens.len() {
                    match &tokens[i] {
                        Token::RedirOut => {
                            i += 1;
                            if i < tokens.len()
                                && let Token::Word(file) = &tokens[i]
                            {
                                current_cmd
                                    .redirections
                                    .push(Redirection::Output(file.clone()));
                                i += 1;
                            }
                        }
                        Token::RedirOutClobber => {
                            i += 1;
                            if i >= tokens.len() {
                                return Err("expected filename after >|".to_string());
                            }
                            if let Token::Word(file) = &tokens[i] {
                                current_cmd
                                    .redirections
                                    .push(Redirection::OutputClobber(file.clone()));
                                i += 1;
                            } else {
                                return Err("expected filename after >|".to_string());
                            }
                        }
                        Token::RedirIn => {
                            i += 1;
                            if i < tokens.len()
                                && let Token::Word(file) = &tokens[i]
                            {
                                current_cmd
                                    .redirections
                                    .push(Redirection::Input(file.clone()));
                                i += 1;
                            }
                        }
                        Token::RedirAppend => {
                            i += 1;
                            if i < tokens.len()
                                && let Token::Word(file) = &tokens[i]
                            {
                                current_cmd
                                    .redirections
                                    .push(Redirection::Append(file.clone()));
                                i += 1;
                            }
                        }
                        Token::RedirectFdOut(fd, file) => {
                            current_cmd
                                .redirections
                                .push(Redirection::FdOutput(*fd, file.clone()));
                            i += 1;
                        }
                        Token::RedirectFdOutClobber(fd, file) => {
                            current_cmd
                                .redirections
                                .push(Redirection::FdOutputClobber(*fd, file.clone()));
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

                // Stage will be pushed at next | or end of loop
                continue;
            }
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
                            if i < tokens.len()
                                && let Token::Word(file) = &tokens[i]
                            {
                                current_cmd
                                    .redirections
                                    .push(Redirection::Output(file.clone()));
                                i += 1;
                            }
                        }
                        Token::RedirOutClobber => {
                            i += 1;
                            if i >= tokens.len() {
                                return Err("expected filename after >|".to_string());
                            }
                            if let Token::Word(file) = &tokens[i] {
                                current_cmd
                                    .redirections
                                    .push(Redirection::OutputClobber(file.clone()));
                                i += 1;
                            } else {
                                return Err("expected filename after >|".to_string());
                            }
                        }
                        Token::RedirIn => {
                            i += 1;
                            if i < tokens.len()
                                && let Token::Word(file) = &tokens[i]
                            {
                                current_cmd
                                    .redirections
                                    .push(Redirection::Input(file.clone()));
                                i += 1;
                            }
                        }
                        Token::RedirAppend => {
                            i += 1;
                            if i < tokens.len()
                                && let Token::Word(file) = &tokens[i]
                            {
                                current_cmd
                                    .redirections
                                    .push(Redirection::Append(file.clone()));
                                i += 1;
                            }
                        }
                        Token::RedirectFdOut(fd, file) => {
                            current_cmd
                                .redirections
                                .push(Redirection::FdOutput(*fd, file.clone()));
                            i += 1;
                        }
                        Token::RedirectFdOutClobber(fd, file) => {
                            current_cmd
                                .redirections
                                .push(Redirection::FdOutputClobber(*fd, file.clone()));
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

                // Stage will be pushed at next | or end of loop
                continue;
            }
            Token::Word(word) => {
                current_cmd.args.push(word.clone());
            }
            Token::Local => {
                current_cmd.args.push("local".to_string());
            }
            Token::Return => {
                current_cmd.args.push("return".to_string());
            }
            Token::Break => {
                current_cmd.args.push("break".to_string());
            }
            Token::Continue => {
                current_cmd.args.push("continue".to_string());
            }
            // Handle keywords as command arguments
            // When keywords appear in pipeline context (not at start of command),
            // they should be treated as regular word arguments
            Token::If => {
                current_cmd.args.push("if".to_string());
            }
            Token::Then => {
                current_cmd.args.push("then".to_string());
            }
            Token::Else => {
                current_cmd.args.push("else".to_string());
            }
            Token::Elif => {
                current_cmd.args.push("elif".to_string());
            }
            Token::Fi => {
                current_cmd.args.push("fi".to_string());
            }
            Token::Case => {
                current_cmd.args.push("case".to_string());
            }
            Token::In => {
                current_cmd.args.push("in".to_string());
            }
            Token::Esac => {
                current_cmd.args.push("esac".to_string());
            }
            Token::For => {
                current_cmd.args.push("for".to_string());
            }
            Token::While => {
                current_cmd.args.push("while".to_string());
            }
            Token::Until => {
                current_cmd.args.push("until".to_string());
            }
            Token::Do => {
                current_cmd.args.push("do".to_string());
            }
            Token::Done => {
                current_cmd.args.push("done".to_string());
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
            Token::RedirOutClobber => {
                i += 1;
                if i >= tokens.len() {
                    return Err("expected filename after >|".to_string());
                }
                if let Token::Word(ref file) = tokens[i] {
                    current_cmd
                        .redirections
                        .push(Redirection::OutputClobber(file.clone()));
                } else {
                    return Err("expected filename after >|".to_string());
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
            Token::RedirectFdOutClobber(fd, file) => {
                current_cmd
                    .redirections
                    .push(Redirection::FdOutputClobber(*fd, file.clone()));
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
                // Ignore newlines in pipelines if they follow a pipe or if we are at the start of a stage
                if current_cmd.args.is_empty() && current_cmd.compound.is_none() {
                    // This newline is between commands or at the start, skip it
                } else {
                    break;
                }
            }
            Token::And | Token::Or | Token::Semicolon => {
                // These tokens end the current pipeline
                // They will be handled by parse_commands_sequentially
                break;
            }
            _ => {
                return Err(format!("Unexpected token in pipeline: {:?}", token));
            }
        }
        i += 1;
    }

    if !current_cmd.args.is_empty() || current_cmd.compound.is_some() {
        commands.push(current_cmd);
    }

    if commands.is_empty() {
        return Err("No commands found".to_string());
    }

    Ok(Ast::Pipeline(commands))
}

#[cfg(test)]
mod tests;
