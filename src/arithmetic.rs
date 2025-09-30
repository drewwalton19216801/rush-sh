use super::state::ShellState;

/// Token types for arithmetic expressions
#[derive(Debug, Clone, PartialEq)]
pub enum ArithmeticToken {
    Number(i64),
    Variable(String),
    Operator(ArithmeticOperator),
    LeftParen,
    RightParen,
}

/// Arithmetic operators with their precedence and associativity
#[derive(Debug, Clone, PartialEq)]
pub enum ArithmeticOperator {
    // Unary operators (precedence 100)
    LogicalNot,  // !
    BitwiseNot,  // ~

    // Binary operators in order of precedence (highest to lowest)
    Multiply,        // *   (precedence 90)
    Divide,          // /   (precedence 90)
    Modulo,          // %   (precedence 90)
    Add,             // +   (precedence 80)
    Subtract,        // -   (precedence 80)
    ShiftLeft,       // <<  (precedence 70)
    ShiftRight,      // >>  (precedence 70)
    LessThan,        // <   (precedence 60)
    LessEqual,       // <=  (precedence 60)
    GreaterThan,     // >   (precedence 60)
    GreaterEqual,    // >=  (precedence 60)
    Equal,           // ==  (precedence 50)
    NotEqual,        // !=  (precedence 50)
    BitwiseAnd,      // &   (precedence 40)
    BitwiseXor,      // ^   (precedence 30)
    BitwiseOr,       // |   (precedence 20)
    LogicalAnd,      // &&  (precedence 10)
    LogicalOr,       // ||  (precedence 5)
}

impl ArithmeticOperator {
    pub fn precedence(&self) -> i32 {
        match self {
            ArithmeticOperator::LogicalNot | ArithmeticOperator::BitwiseNot => 100,

            ArithmeticOperator::Multiply | ArithmeticOperator::Divide | ArithmeticOperator::Modulo => 90,
            ArithmeticOperator::Add | ArithmeticOperator::Subtract => 80,
            ArithmeticOperator::ShiftLeft | ArithmeticOperator::ShiftRight => 70,
            ArithmeticOperator::LessThan | ArithmeticOperator::LessEqual |
            ArithmeticOperator::GreaterThan | ArithmeticOperator::GreaterEqual => 60,
            ArithmeticOperator::Equal | ArithmeticOperator::NotEqual => 50,
            ArithmeticOperator::BitwiseAnd => 40,
            ArithmeticOperator::BitwiseXor => 30,
            ArithmeticOperator::BitwiseOr => 20,
            ArithmeticOperator::LogicalAnd => 10,
            ArithmeticOperator::LogicalOr => 5,
        }
    }

    pub fn is_unary(&self) -> bool {
        matches!(self, ArithmeticOperator::LogicalNot | ArithmeticOperator::BitwiseNot)
    }

}

/// Errors that can occur during arithmetic evaluation
#[derive(Debug, Clone)]
pub enum ArithmeticError {
    SyntaxError(String),
    DivisionByZero,
    UndefinedVariable(String),
    UnmatchedParentheses,
    EmptyExpression,
}

impl std::fmt::Display for ArithmeticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArithmeticError::SyntaxError(msg) => write!(f, "Syntax error: {}", msg),
            ArithmeticError::DivisionByZero => write!(f, "Division by zero"),
            ArithmeticError::UndefinedVariable(var) => write!(f, "Undefined variable: {}", var),
            ArithmeticError::UnmatchedParentheses => write!(f, "Unmatched parentheses"),
            ArithmeticError::EmptyExpression => write!(f, "Empty expression"),
        }
    }
}

/// Tokenize an arithmetic expression into tokens
pub fn tokenize_expression(expr: &str) -> Result<Vec<ArithmeticToken>, ArithmeticError> {
    let mut tokens = Vec::new();
    let mut chars = expr.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            ' ' | '\t' | '\n' => continue, // Skip whitespace

            '(' => tokens.push(ArithmeticToken::LeftParen),
            ')' => tokens.push(ArithmeticToken::RightParen),

            '+' => {
                if let Some(next_ch) = chars.peek() {
                    if *next_ch == '+' {
                        return Err(ArithmeticError::SyntaxError("Unexpected ++".to_string()));
                    }
                }
                tokens.push(ArithmeticToken::Operator(ArithmeticOperator::Add));
            }

            '-' => {
                if let Some(next_ch) = chars.peek() {
                    if *next_ch == '-' {
                        return Err(ArithmeticError::SyntaxError("Unexpected --".to_string()));
                    }
                }
                tokens.push(ArithmeticToken::Operator(ArithmeticOperator::Subtract));
            }

            '*' => tokens.push(ArithmeticToken::Operator(ArithmeticOperator::Multiply)),
            '/' => tokens.push(ArithmeticToken::Operator(ArithmeticOperator::Divide)),
            '%' => tokens.push(ArithmeticToken::Operator(ArithmeticOperator::Modulo)),

            '<' => {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '<' {
                        chars.next();
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::ShiftLeft));
                    } else if next_ch == '=' {
                        chars.next();
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::LessEqual));
                    } else {
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::LessThan));
                    }
                } else {
                    tokens.push(ArithmeticToken::Operator(ArithmeticOperator::LessThan));
                }
            }

            '>' => {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '>' {
                        chars.next();
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::ShiftRight));
                    } else if next_ch == '=' {
                        chars.next();
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::GreaterEqual));
                    } else {
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::GreaterThan));
                    }
                } else {
                    tokens.push(ArithmeticToken::Operator(ArithmeticOperator::GreaterThan));
                }
            }

            '=' => {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '=' {
                        chars.next();
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::Equal));
                    } else {
                        return Err(ArithmeticError::SyntaxError("Unexpected =".to_string()));
                    }
                } else {
                    return Err(ArithmeticError::SyntaxError("Unexpected =".to_string()));
                }
            }

            '!' => {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '=' {
                        chars.next();
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::NotEqual));
                    } else {
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::LogicalNot));
                    }
                } else {
                    tokens.push(ArithmeticToken::Operator(ArithmeticOperator::LogicalNot));
                }
            }

            '&' => {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '&' {
                        chars.next();
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::LogicalAnd));
                    } else {
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::BitwiseAnd));
                    }
                } else {
                    tokens.push(ArithmeticToken::Operator(ArithmeticOperator::BitwiseAnd));
                }
            }

            '|' => {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '|' {
                        chars.next();
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::LogicalOr));
                    } else {
                        tokens.push(ArithmeticToken::Operator(ArithmeticOperator::BitwiseOr));
                    }
                } else {
                    tokens.push(ArithmeticToken::Operator(ArithmeticOperator::BitwiseOr));
                }
            }

            '^' => tokens.push(ArithmeticToken::Operator(ArithmeticOperator::BitwiseXor)),
            '~' => tokens.push(ArithmeticToken::Operator(ArithmeticOperator::BitwiseNot)),

            // Numbers and variables
            '0'..='9' => {
                let mut num_str = String::new();
                num_str.push(ch);
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_ascii_digit() {
                        num_str.push(next_ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                match num_str.parse::<i64>() {
                    Ok(num) => tokens.push(ArithmeticToken::Number(num)),
                    Err(_) => return Err(ArithmeticError::SyntaxError("Invalid number".to_string())),
                }
            }

            // Variables (start with letter or underscore)
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut var_name = String::new();
                var_name.push(ch);
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        var_name.push(next_ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(ArithmeticToken::Variable(var_name));
            }

            _ => {
                return Err(ArithmeticError::SyntaxError(format!("Unexpected character: {}", ch)));
            }
        }
    }

    Ok(tokens)
}

/// Parse tokens into Reverse Polish Notation (RPN) using Shunting-yard algorithm
pub fn parse_to_rpn(tokens: Vec<ArithmeticToken>) -> Result<Vec<ArithmeticToken>, ArithmeticError> {
    let mut output = Vec::new();
    let mut operators = Vec::new();

    for token in tokens {
        match token {
            ArithmeticToken::Number(_) | ArithmeticToken::Variable(_) => {
                output.push(token);
            }

            ArithmeticToken::Operator(op) => {
                // Handle unary operators
                if op.is_unary() && (output.is_empty() ||
                    matches!(output.last(),
                        Some(ArithmeticToken::Operator(_) | ArithmeticToken::LeftParen))) {
                    // This is a unary operator
                    while !operators.is_empty() {
                        if let Some(ArithmeticToken::Operator(top_op)) = operators.last() {
                            if top_op.precedence() >= op.precedence() && !top_op.is_unary() {
                                output.push(operators.pop().unwrap());
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    operators.push(ArithmeticToken::Operator(op));
                } else {
                    // Binary operator
                    while !operators.is_empty() {
                        if let Some(ArithmeticToken::Operator(top_op)) = operators.last() {
                            if (top_op.precedence() > op.precedence()) ||
                               (top_op.precedence() == op.precedence() && !op.is_unary()) {
                                output.push(operators.pop().unwrap());
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    operators.push(ArithmeticToken::Operator(op));
                }
            }

            ArithmeticToken::LeftParen => {
                operators.push(token);
            }

            ArithmeticToken::RightParen => {
                let mut found_left = false;
                while let Some(op) = operators.pop() {
                    if op == ArithmeticToken::LeftParen {
                        found_left = true;
                        break;
                    } else {
                        output.push(op);
                    }
                }
                if !found_left {
                    return Err(ArithmeticError::UnmatchedParentheses);
                }
            }
        }
    }

    // Pop remaining operators
    while let Some(op) = operators.pop() {
        if op == ArithmeticToken::LeftParen {
            return Err(ArithmeticError::UnmatchedParentheses);
        }
        output.push(op);
    }

    Ok(output)
}

/// Evaluate an arithmetic expression in Reverse Polish Notation
pub fn evaluate_rpn(rpn_tokens: Vec<ArithmeticToken>, shell_state: &ShellState) -> Result<i64, ArithmeticError> {
    let mut stack = Vec::new();

    for token in rpn_tokens {
        match token {
            ArithmeticToken::Number(num) => {
                stack.push(num);
            }

            ArithmeticToken::Variable(var_name) => {
                if let Some(value) = shell_state.get_var(&var_name) {
                    match value.parse::<i64>() {
                        Ok(num) => stack.push(num),
                        Err(_) => return Err(ArithmeticError::UndefinedVariable(var_name)),
                    }
                } else {
                    return Err(ArithmeticError::UndefinedVariable(var_name));
                }
            }

            ArithmeticToken::Operator(op) => {
                if op.is_unary() {
                    if stack.is_empty() {
                        return Err(ArithmeticError::SyntaxError("Missing operand for unary operator".to_string()));
                    }
                    let operand = stack.pop().unwrap();
                    let result = match op {
                        ArithmeticOperator::LogicalNot => !operand,
                        ArithmeticOperator::BitwiseNot => !operand,
                        _ => unreachable!(),
                    };
                    stack.push(result);
                } else {
                    if stack.len() < 2 {
                        return Err(ArithmeticError::SyntaxError("Missing operands for binary operator".to_string()));
                    }
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    let result = match op {
                        ArithmeticOperator::Add => left + right,
                        ArithmeticOperator::Subtract => left - right,
                        ArithmeticOperator::Multiply => left * right,
                        ArithmeticOperator::Divide => {
                            if right == 0 {
                                return Err(ArithmeticError::DivisionByZero);
                            }
                            left / right
                        }
                        ArithmeticOperator::Modulo => {
                            if right == 0 {
                                return Err(ArithmeticError::DivisionByZero);
                            }
                            left % right
                        }
                        ArithmeticOperator::ShiftLeft => left << right,
                        ArithmeticOperator::ShiftRight => left >> right,
                        ArithmeticOperator::LessThan => if left < right { 1 } else { 0 },
                        ArithmeticOperator::LessEqual => if left <= right { 1 } else { 0 },
                        ArithmeticOperator::GreaterThan => if left > right { 1 } else { 0 },
                        ArithmeticOperator::GreaterEqual => if left >= right { 1 } else { 0 },
                        ArithmeticOperator::Equal => if left == right { 1 } else { 0 },
                        ArithmeticOperator::NotEqual => if left != right { 1 } else { 0 },
                        ArithmeticOperator::BitwiseAnd => left & right,
                        ArithmeticOperator::BitwiseXor => left ^ right,
                        ArithmeticOperator::BitwiseOr => left | right,
                        ArithmeticOperator::LogicalAnd => if left != 0 && right != 0 { 1 } else { 0 },
                        ArithmeticOperator::LogicalOr => if left != 0 || right != 0 { 1 } else { 0 },
                        _ => unreachable!(),
                    };
                    stack.push(result);
                }
            }

            ArithmeticToken::LeftParen | ArithmeticToken::RightParen => {
                return Err(ArithmeticError::SyntaxError("Unexpected parenthesis in RPN".to_string()));
            }
        }
    }

    if stack.len() != 1 {
        return Err(ArithmeticError::SyntaxError("Invalid expression".to_string()));
    }

    Ok(stack[0])
}

/// Main function to evaluate an arithmetic expression
pub fn evaluate_arithmetic_expression(expr: &str, shell_state: &ShellState) -> Result<i64, ArithmeticError> {
    if expr.trim().is_empty() {
        return Err(ArithmeticError::EmptyExpression);
    }

    let tokens = tokenize_expression(expr)?;
    let rpn_tokens = parse_to_rpn(tokens)?;
    let result = evaluate_rpn(rpn_tokens, shell_state)?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple_numbers() {
        let tokens = tokenize_expression("42").unwrap();
        assert_eq!(tokens, vec![ArithmeticToken::Number(42)]);
    }

    #[test]
    fn test_tokenize_operators() {
        let tokens = tokenize_expression("2+3").unwrap();
        assert_eq!(tokens, vec![
            ArithmeticToken::Number(2),
            ArithmeticToken::Operator(ArithmeticOperator::Add),
            ArithmeticToken::Number(3)
        ]);
    }

    #[test]
    fn test_tokenize_parentheses() {
        let tokens = tokenize_expression("(2+3)").unwrap();
        assert_eq!(tokens, vec![
            ArithmeticToken::LeftParen,
            ArithmeticToken::Number(2),
            ArithmeticToken::Operator(ArithmeticOperator::Add),
            ArithmeticToken::Number(3),
            ArithmeticToken::RightParen
        ]);
    }

    #[test]
    fn test_tokenize_variables() {
        let tokens = tokenize_expression("x+y").unwrap();
        assert_eq!(tokens, vec![
            ArithmeticToken::Variable("x".to_string()),
            ArithmeticToken::Operator(ArithmeticOperator::Add),
            ArithmeticToken::Variable("y".to_string())
        ]);
    }

    #[test]
    fn test_evaluate_simple() {
        let shell_state = ShellState::new();
        let result = evaluate_arithmetic_expression("42", &shell_state).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_evaluate_addition() {
        let shell_state = ShellState::new();
        let result = evaluate_arithmetic_expression("2+3", &shell_state).unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn test_evaluate_with_precedence() {
        let shell_state = ShellState::new();
        let result = evaluate_arithmetic_expression("2+3*4", &shell_state).unwrap();
        assert_eq!(result, 14); // 3*4 = 12, +2 = 14
    }

    #[test]
    fn test_evaluate_with_parentheses() {
        let shell_state = ShellState::new();
        let result = evaluate_arithmetic_expression("(2+3)*4", &shell_state).unwrap();
        assert_eq!(result, 20); // (2+3) = 5, *4 = 20
    }

    #[test]
    fn test_evaluate_comparison() {
        let shell_state = ShellState::new();
        let result = evaluate_arithmetic_expression("5>3", &shell_state).unwrap();
        assert_eq!(result, 1); // true

        let result = evaluate_arithmetic_expression("3>5", &shell_state).unwrap();
        assert_eq!(result, 0); // false
    }

    #[test]
    fn test_evaluate_variable() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("x", "10".to_string());
        let result = evaluate_arithmetic_expression("x + 5", &shell_state).unwrap();
        assert_eq!(result, 15);
    }

    #[test]
    fn test_evaluate_division_by_zero() {
        let shell_state = ShellState::new();
        let result = evaluate_arithmetic_expression("5/0", &shell_state);
        assert!(matches!(result, Err(ArithmeticError::DivisionByZero)));
    }

    #[test]
    fn test_evaluate_undefined_variable() {
        let shell_state = ShellState::new();
        let result = evaluate_arithmetic_expression("undefined + 5", &shell_state);
        assert!(matches!(result, Err(ArithmeticError::UndefinedVariable(_))));
    }
}