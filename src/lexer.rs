use std::env;

use super::state::ShellState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(String),
    Pipe,
    RedirOut,
    RedirIn,
    RedirAppend,
    If,
    Then,
    Else,
    Elif,
    Fi,
    Case,
    In,
    Esac,
    DoubleSemicolon,
    Semicolon,
    RightParen,
    Newline,
}

fn is_keyword(word: &str) -> Option<Token> {
    match word {
        "if" => Some(Token::If),
        "then" => Some(Token::Then),
        "else" => Some(Token::Else),
        "elif" => Some(Token::Elif),
        "fi" => Some(Token::Fi),
        "case" => Some(Token::Case),
        "in" => Some(Token::In),
        "esac" => Some(Token::Esac),
        _ => None,
    }
}

pub fn lex(input: &str, shell_state: &ShellState) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut current = String::new();
    let mut in_double_quote = false;
    let mut in_single_quote = false;

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' if !in_double_quote && !in_single_quote => {
                if !current.is_empty() {
                    if let Some(keyword) = is_keyword(&current) {
                        tokens.push(keyword);
                    } else {
                        tokens.push(Token::Word(current.clone()));
                    }
                    current.clear();
                }
                chars.next();
            }
            '\n' if !in_double_quote && !in_single_quote => {
                if !current.is_empty() {
                    if let Some(keyword) = is_keyword(&current) {
                        tokens.push(keyword);
                    } else {
                        tokens.push(Token::Word(current.clone()));
                    }
                    current.clear();
                }
                tokens.push(Token::Newline);
                chars.next();
            }
            '"' => {
                if in_double_quote {
                    tokens.push(Token::Word(current.clone()));
                    current.clear();
                    in_double_quote = false;
                } else if !in_single_quote {
                    if !current.is_empty() {
                        if let Some(keyword) = is_keyword(&current) {
                            tokens.push(keyword);
                        } else {
                            tokens.push(Token::Word(current.clone()));
                        }
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
                        if let Some(keyword) = is_keyword(&current) {
                            tokens.push(keyword);
                        } else {
                            tokens.push(Token::Word(current.clone()));
                        }
                        current.clear();
                    }
                    in_single_quote = true;
                }
                chars.next();
            }
            '$' if !in_single_quote => {
                chars.next();
                let var_start = chars.clone();
                let var_name: String = chars
                    .by_ref()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !var_name.is_empty() {
                    if let Some(val) = shell_state.get_var(&var_name) {
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
                    if let Some(keyword) = is_keyword(&current) {
                        tokens.push(keyword);
                    } else {
                        tokens.push(Token::Word(current.clone()));
                    }
                    current.clear();
                }
                tokens.push(Token::Pipe);
                chars.next();
            }
            '>' => {
                if !current.is_empty() {
                    if let Some(keyword) = is_keyword(&current) {
                        tokens.push(keyword);
                    } else {
                        tokens.push(Token::Word(current.clone()));
                    }
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
                    if let Some(keyword) = is_keyword(&current) {
                        tokens.push(keyword);
                    } else {
                        tokens.push(Token::Word(current.clone()));
                    }
                    current.clear();
                }
                tokens.push(Token::RedirIn);
                chars.next();
            }
            ')' => {
                if !current.is_empty() {
                    if let Some(keyword) = is_keyword(&current) {
                        tokens.push(keyword);
                    } else {
                        tokens.push(Token::Word(current.clone()));
                    }
                    current.clear();
                }
                tokens.push(Token::RightParen);
                chars.next();
            }
            ';' => {
                if !current.is_empty() {
                    if let Some(keyword) = is_keyword(&current) {
                        tokens.push(keyword);
                    } else {
                        tokens.push(Token::Word(current.clone()));
                    }
                    current.clear();
                }
                chars.next();
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == ';' {
                        chars.next();
                        tokens.push(Token::DoubleSemicolon);
                    } else {
                        tokens.push(Token::Semicolon);
                    }
                } else {
                    tokens.push(Token::Semicolon);
                }
            }
            _ => {
                if ch == '~' && current.is_empty() {
                    if let Ok(home) = env::var("HOME") {
                        current.push_str(&home);
                    } else {
                        current.push('~');
                    }
                } else {
                    current.push(ch);
                }
                chars.next();
            }
        }
    }
    if !current.is_empty() {
        if let Some(keyword) = is_keyword(&current) {
            tokens.push(keyword);
        } else {
            tokens.push(Token::Word(current));
        }
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_word() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("ls", &shell_state).unwrap();
        assert_eq!(result, vec![Token::Word("ls".to_string())]);
    }

    #[test]
    fn test_multiple_words() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("ls -la", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("ls".to_string()),
                Token::Word("-la".to_string())
            ]
        );
    }

    #[test]
    fn test_pipe() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("ls | grep txt", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("ls".to_string()),
                Token::Pipe,
                Token::Word("grep".to_string()),
                Token::Word("txt".to_string())
            ]
        );
    }

    #[test]
    fn test_redirections() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("echo hello > output.txt", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello".to_string()),
                Token::RedirOut,
                Token::Word("output.txt".to_string())
            ]
        );
    }

    #[test]
    fn test_append_redirection() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("echo hello >> output.txt", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello".to_string()),
                Token::RedirAppend,
                Token::Word("output.txt".to_string())
            ]
        );
    }

    #[test]
    fn test_input_redirection() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("cat < input.txt", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("cat".to_string()),
                Token::RedirIn,
                Token::Word("input.txt".to_string())
            ]
        );
    }

    #[test]
    fn test_double_quotes() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("echo \"hello world\"", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello world".to_string())
            ]
        );
    }

    #[test]
    fn test_single_quotes() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("echo 'hello world'", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello world".to_string())
            ]
        );
    }

    #[test]
    fn test_variable_expansion() {
        let mut shell_state = crate::state::ShellState::new();
        shell_state.set_var("TEST_VAR", "expanded_value".to_string());
        let result = lex("echo $TEST_VAR", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("expanded_value".to_string())
            ]
        );
    }

    #[test]
    fn test_variable_expansion_nonexistent() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("echo $TEST_VAR2", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("$TEST_VAR2".to_string())
            ]
        );
    }

    #[test]
    fn test_empty_variable() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("echo $", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("$".to_string())
            ]
        );
    }

    #[test]
    fn test_mixed_quotes_and_variables() {
        let mut shell_state = crate::state::ShellState::new();
        shell_state.set_var("USER", "alice".to_string());
        let result = lex("echo \"Hello $USER\"", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("Hello alice".to_string())
            ]
        );
    }

    #[test]
    fn test_unclosed_double_quote() {
        // Lexer doesn't handle unclosed quotes as errors, just treats as literal
        let shell_state = crate::state::ShellState::new();
        let result = lex("echo \"hello", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello".to_string())
            ]
        );
    }

    #[test]
    fn test_empty_input() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("", &shell_state).unwrap();
        assert_eq!(result, Vec::<Token>::new());
    }

    #[test]
    fn test_only_spaces() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("   ", &shell_state).unwrap();
        assert_eq!(result, Vec::<Token>::new());
    }

    #[test]
    fn test_complex_pipeline() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("cat input.txt | grep \"search term\" > output.txt", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("cat".to_string()),
                Token::Word("input.txt".to_string()),
                Token::Pipe,
                Token::Word("grep".to_string()),
                Token::Word("search term".to_string()),
                Token::RedirOut,
                Token::Word("output.txt".to_string())
            ]
        );
    }

    #[test]
    fn test_if_tokens() {
        let shell_state = crate::state::ShellState::new();
        let result = lex("if true; then echo yes; fi", &shell_state).unwrap();
        assert_eq!(
            result,
            vec![
                Token::If,
                Token::Word("true".to_string()),
                Token::Semicolon,
                Token::Then,
                Token::Word("echo".to_string()),
                Token::Word("yes".to_string()),
                Token::Semicolon,
                Token::Fi,
            ]
        );
    }
}
