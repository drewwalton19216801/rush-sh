use std::env;

#[derive(Debug, Clone)]
pub enum Token {
    Word(String),
    Pipe,
    RedirOut,
    RedirIn,
    RedirAppend,
}

pub fn lex(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut current = String::new();
    let mut in_double_quote = false;
    let mut in_single_quote = false;

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' if !in_double_quote && !in_single_quote => {
                if !current.is_empty() {
                    tokens.push(Token::Word(current.clone()));
                    current.clear();
                }
                chars.next();
            }
            '"' => {
                if in_double_quote {
                    tokens.push(Token::Word(current.clone()));
                    current.clear();
                    in_double_quote = false;
                } else if !in_single_quote {
                    if !current.is_empty() {
                        tokens.push(Token::Word(current.clone()));
                        current.clear();
                    }
                    in_double_quote = true;
                }
                chars.next();
            }
            '\'' => {
                if in_single_quote {
                    tokens.push(Token::Word(current.clone()));
                    current.clear();
                    in_single_quote = false;
                } else if !in_double_quote {
                    if !current.is_empty() {
                        tokens.push(Token::Word(current.clone()));
                        current.clear();
                    }
                    in_single_quote = true;
                }
                chars.next();
            }
            '$' if !in_single_quote => {
                chars.next();
                let var_start = chars.clone();
                let var_name: String = chars.by_ref().take_while(|c| c.is_alphanumeric()).collect();
                if !var_name.is_empty() {
                    if let Ok(val) = env::var(&var_name) {
                        current.push_str(&val);
                    } else {
                        current.push('$');
                        current.push_str(&var_name);
                    }
                } else {
                    chars = var_start;
                    current.push('$');
                }
            }
            '|' => {
                if !current.is_empty() {
                    tokens.push(Token::Word(current.clone()));
                    current.clear();
                }
                tokens.push(Token::Pipe);
                chars.next();
            }
            '>' => {
                if !current.is_empty() {
                    tokens.push(Token::Word(current.clone()));
                    current.clear();
                }
                chars.next();
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '>' {
                        chars.next();
                        tokens.push(Token::RedirAppend);
                    } else {
                        tokens.push(Token::RedirOut);
                    }
                } else {
                    tokens.push(Token::RedirOut);
                }
            }
            '<' => {
                if !current.is_empty() {
                    tokens.push(Token::Word(current.clone()));
                    current.clear();
                }
                tokens.push(Token::RedirIn);
                chars.next();
            }
            _ => {
                current.push(ch);
                chars.next();
            }
        }
    }
    if !current.is_empty() {
        tokens.push(Token::Word(current));
    }
    Ok(tokens)
}
