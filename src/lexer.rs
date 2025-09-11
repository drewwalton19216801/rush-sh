use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
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
                let var_name: String = chars
                    .by_ref()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
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
        tokens.push(Token::Word(current));
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_basic_word() {
        let result = lex("ls").unwrap();
        assert_eq!(result, vec![Token::Word("ls".to_string())]);
    }

    #[test]
    fn test_multiple_words() {
        let result = lex("ls -la").unwrap();
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
        let result = lex("ls | grep txt").unwrap();
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
        let result = lex("echo hello > output.txt").unwrap();
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
        let result = lex("echo hello >> output.txt").unwrap();
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
        let result = lex("cat < input.txt").unwrap();
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
        let result = lex("echo \"hello world\"").unwrap();
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
        let result = lex("echo 'hello world'").unwrap();
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
        env::set_var("TEST_VAR", "expanded_value");
        let result = lex("echo $TEST_VAR").unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("expanded_value".to_string())
            ]
        );
        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_variable_expansion_nonexistent() {
        // Ensure TEST_VAR2 is not set
        env::remove_var("TEST_VAR2");
        let result = lex("echo $TEST_VAR2").unwrap();
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
        let result = lex("echo $").unwrap();
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
        env::set_var("USER", "alice");
        let result = lex("echo \"Hello $USER\"").unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("Hello alice".to_string())
            ]
        );
        env::remove_var("USER");
    }

    #[test]
    fn test_unclosed_double_quote() {
        // Lexer doesn't handle unclosed quotes as errors, just treats as literal
        let result = lex("echo \"hello").unwrap();
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
        let result = lex("").unwrap();
        assert_eq!(result, Vec::<Token>::new());
    }

    #[test]
    fn test_only_spaces() {
        let result = lex("   ").unwrap();
        assert_eq!(result, Vec::<Token>::new());
    }

    #[test]
    fn test_complex_pipeline() {
        let result = lex("cat input.txt | grep \"search term\" > output.txt").unwrap();
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
}
