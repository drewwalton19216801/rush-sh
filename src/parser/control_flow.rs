//! Control flow parsing module for the Rush shell.
//!
//! This module handles parsing of POSIX shell control flow constructs including:
//! - **if/elif/else/fi** - Conditional execution with multiple branches
//! - **case/esac** - Pattern matching with multiple cases
//! - **for/in/do/done** - Iteration over lists
//! - **while/do/done** - Conditional loops (execute while condition is true)
//! - **until/do/done** - Conditional loops (execute until condition is true)
//! - **function definitions** - Named function declarations with body
//!
//! ## Parsing Strategy
//!
//! Each control structure parser follows a similar pattern:
//! 1. Validate the opening keyword (if, case, for, while, until, function name)
//! 2. Parse the control expression (condition, pattern, variable, etc.)
//! 3. Parse the body tokens, handling nested structures correctly
//! 4. Validate the closing keyword (fi, esac, done, closing brace)
//! 5. Construct the appropriate AST node
//!
//! ## Nested Structure Handling
//!
//! The parsers correctly handle nested control structures by tracking depth:
//! - `parse_if()` tracks nested if/fi pairs
//! - `parse_for()`, `parse_while()`, `parse_until()` track nested loop/done pairs
//! - `parse_case()` finds the matching esac
//! - `parse_function_definition()` tracks nested braces and control structures
//!
//! ## POSIX Compliance
//!
//! These parsers implement POSIX-compliant control flow syntax:
//! - Proper handling of newlines and semicolons as command separators
//! - Support for empty bodies (treated as no-op/true)
//! - Correct precedence of operators within conditions
//! - Support for both standard and legacy function definition formats
//!
//! ## Integration with Main Parser
//!
//! These functions are called from the main parser when control flow keywords
//! are encountered. They return `Result<Ast, String>` to allow error propagation.
//! The parsed AST nodes are then integrated into the larger parse tree.

use super::*;

/// Helper function to skip consecutive newline tokens.
/// Updates the index to point to the first non-newline token.
pub(super) fn skip_newlines(tokens: &[Token], i: &mut usize) {
    while *i < tokens.len() && tokens[*i] == Token::Newline {
        *i += 1;
    }
}

/// Helper function to skip to the matching 'fi' token for an 'if' statement.
/// Handles nested if statements correctly by tracking depth.
///
/// # Arguments
/// * `tokens` - The token slice to search through
/// * `i` - Mutable reference to current index, updated to point after the matching 'fi'
pub(super) fn skip_to_matching_fi(tokens: &[Token], i: &mut usize) {
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

/// Helper function to skip to the matching 'done' token for a 'for', 'while', or 'until' loop.
/// Handles nested loops correctly by tracking depth.
///
/// # Arguments
/// * `tokens` - The token slice to search through
/// * `i` - Mutable reference to current index, updated to point after the matching 'done'
pub(super) fn skip_to_matching_done(tokens: &[Token], i: &mut usize) {
    let mut loop_depth = 1;
    *i += 1; // Move past the 'for' or 'while' or 'until' token
    while *i < tokens.len() && loop_depth > 0 {
        match tokens[*i] {
            Token::For | Token::While | Token::Until => loop_depth += 1,
            Token::Done => loop_depth -= 1,
            _ => {}
        }
        *i += 1;
    }
}

/// Helper function to skip to the matching 'esac' token for a 'case' statement.
///
/// # Arguments
/// * `tokens` - The token slice to search through
/// * `i` - Mutable reference to current index, updated to point after the matching 'esac'
pub(super) fn skip_to_matching_esac(tokens: &[Token], i: &mut usize) {
    *i += 1; // Move past the 'case' token
    while *i < tokens.len() {
        if tokens[*i] == Token::Esac {
            *i += 1;
            break;
        }
        *i += 1;
    }
}

/// Parses an if/elif/else/fi conditional construct.
///
/// Syntax: `if condition; then commands; [elif condition; then commands;]... [else commands;] fi`
///
/// This function handles:
/// - Multiple elif branches
/// - Optional else branch
/// - Nested if statements
/// - Empty then/else bodies (treated as no-op)
/// - Newlines and semicolons as command separators
///
/// # Arguments
/// * `tokens` - Token slice starting with Token::If
///
/// # Returns
/// * `Ok(Ast::If)` with branches and optional else_branch
/// * `Err(String)` if syntax is invalid (missing then, fi, etc.)
///
/// # Examples
/// ```text
/// if true; then echo yes; fi
/// if false; then echo no; elif true; then echo maybe; else echo default; fi
/// ```
pub(super) fn parse_if(tokens: &[Token]) -> Result<Ast, String> {
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

        // Parse condition using parse_next_command to handle ! and operators correctly
        let (condition, _) = parse_next_command(&cond_tokens)?;
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

/// Parses a case/in/esac pattern matching construct.
///
/// Syntax: `case word in pattern) commands;; ... esac`
///
/// This function handles:
/// - Multiple pattern branches separated by |
/// - Optional default case (pattern *)
/// - Commands terminated by ;; or esac
/// - Empty command bodies
///
/// # Arguments
/// * `tokens` - Token slice starting with Token::Case
///
/// # Returns
/// * `Ok(Ast::Case)` with word, cases, and optional default
/// * `Err(String)` if syntax is invalid
///
/// # Examples
/// ```text
/// case $var in
///   pattern1) echo one;;
///   pattern2|pattern3) echo two or three;;
///   *) echo default;;
/// esac
/// ```
pub(super) fn parse_case(tokens: &[Token]) -> Result<Ast, String> {
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
        // Parse case body using parse_next_command to handle ! and operators correctly
        let (commands_ast, _) = parse_next_command(&commands_tokens)?;

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

/// Parses a for/in/do/done iteration loop.
///
/// Syntax: `for variable in items; do commands; done`
///
/// This function handles:
/// - Variable name validation
/// - List of items to iterate over
/// - Loop body with nested structures
/// - Empty loop bodies (treated as no-op)
///
/// # Arguments
/// * `tokens` - Token slice starting with Token::For
///
/// # Returns
/// * `Ok(Ast::For)` with variable, items, and body
/// * `Err(String)` if syntax is invalid
///
/// # Examples
/// ```text
/// for i in 1 2 3; do echo $i; done
/// for file in *.txt; do cat "$file"; done
/// ```
pub(super) fn parse_for(tokens: &[Token]) -> Result<Ast, String> {
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

/// Parses a while/do/done conditional loop.
///
/// Syntax: `while condition; do commands; done`
///
/// This function handles:
/// - Condition parsing with operators and negation
/// - Loop body with nested structures
/// - Empty loop bodies (treated as no-op)
///
/// # Arguments
/// * `tokens` - Token slice starting with Token::While
///
/// # Returns
/// * `Ok(Ast::While)` with condition and body
/// * `Err(String)` if syntax is invalid
///
/// # Examples
/// ```text
/// while true; do echo loop; done
/// while [ $count -lt 10 ]; do count=$((count + 1)); done
/// ```
pub(super) fn parse_while(tokens: &[Token]) -> Result<Ast, String> {
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
            Token::While | Token::For | Token::Until => {
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

    // Parse the condition using parse_next_command to handle ! and operators correctly
    let (condition_ast, _) = parse_next_command(&cond_tokens)?;

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

/// Parses an until/do/done conditional loop.
///
/// Syntax: `until condition; do commands; done`
///
/// This function handles:
/// - Condition parsing with operators and negation
/// - Loop body with nested structures
/// - Empty loop bodies (treated as no-op)
///
/// # Arguments
/// * `tokens` - Token slice starting with Token::Until
///
/// # Returns
/// * `Ok(Ast::Until)` with condition and body
/// * `Err(String)` if syntax is invalid
///
/// # Examples
/// ```text
/// until false; do echo loop; done
/// until [ $count -ge 10 ]; do count=$((count + 1)); done
/// ```
pub(super) fn parse_until(tokens: &[Token]) -> Result<Ast, String> {
    let mut i = 1; // Skip 'until'

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
        return Err("Expected condition after until".to_string());
    }

    // Skip any newlines before 'do'
    while i < tokens.len() && tokens[i] == Token::Newline {
        i += 1;
    }

    // Expect 'do'
    if i >= tokens.len() || tokens[i] != Token::Do {
        return Err("Expected 'do' in until loop".to_string());
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
            Token::While | Token::For | Token::Until => {
                depth += 1;
                body_tokens.push(tokens[i].clone());
            }
            Token::Done => {
                if depth > 0 {
                    depth -= 1;
                    body_tokens.push(tokens[i].clone());
                } else {
                    break; // This done closes our until loop
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
        return Err("Expected 'done' to close until loop".to_string());
    }

    // Parse the condition using parse_next_command to handle ! and operators correctly
    let (condition_ast, _) = parse_next_command(&cond_tokens)?;

    // Parse the body
    let body_ast = if body_tokens.is_empty() {
        // Empty body - create a no-op
        create_empty_body_ast()
    } else {
        parse_commands_sequentially(&body_tokens)?
    };

    Ok(Ast::Until {
        condition: Box::new(condition_ast),
        body: Box::new(body_ast),
    })
}

/// Parses a function definition.
///
/// Syntax: `name() { commands; }` or legacy `name() { commands; }`
///
/// This function handles:
/// - Standard format: `name ( ) { body }`
/// - Legacy format: `name() { body }`
/// - Nested function definitions
/// - Nested control structures (if, for, while, case)
/// - Empty function bodies (treated as no-op)
///
/// # Arguments
/// * `tokens` - Token slice starting with function name
///
/// # Returns
/// * `Ok(Ast::FunctionDefinition)` with name and body
/// * `Err(String)` if syntax is invalid
///
/// # Examples
/// ```text
/// myfunc() { echo hello; }
/// greet() { echo "Hello, $1"; }
/// ```
pub(super) fn parse_function_definition(tokens: &[Token]) -> Result<Ast, String> {
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
            Token::For | Token::While | Token::Until => {
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