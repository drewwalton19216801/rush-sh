use super::lexer::Token;

#[derive(Debug, Clone)]
pub enum Ast {
    Pipeline(Vec<ShellCommand>),
}

#[derive(Debug, Clone)]
pub struct ShellCommand {
    pub args: Vec<String>,
    pub input: Option<String>,
    pub output: Option<String>,
    pub append: Option<String>,
}

pub fn parse(tokens: Vec<Token>) -> Result<Ast, String> {
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
