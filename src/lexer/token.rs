//! Token definitions for the Rush shell lexer.
//!
//! This module contains the core token types that represent the lexical elements
//! of shell syntax. Tokens are the building blocks produced by the lexer and
//! consumed by the parser to construct an Abstract Syntax Tree (AST).
//!
//! # Token Categories
//!
//! ## Keywords
//! Control flow and structural keywords like `if`, `then`, `else`, `fi`, `case`,
//! `for`, `while`, `until`, `do`, `done`, `local`, `return`, `break`, `continue`.
//!
//! ## Operators
//! - **Pipe operators**: `|` (pipe), `||` (OR), `&&` (AND)
//! - **Redirection operators**: `>`, `>>`, `<`, `<<`, `<<<`, `>|`, `<>`
//! - **File descriptor operators**: `N>`, `N<`, `N>>`, `N>&M`, `N<&M`, `N>&-`, `N<&-`, `N<>`
//! - **Structural operators**: `;`, `;;`, `(`, `)`, `{`, `}`, `!`
//!
//! ## Words
//! Command names, arguments, variable names, and other textual content.
//!
//! ## Special Tokens
//! - **Newline**: Line terminators
//! - **Here-documents**: Multi-line input redirection
//! - **Here-strings**: Single-line input redirection
//!
//! # Examples
//!
//! ```
//! use rush_sh::lexer::Token;
//!
//! // A simple command token sequence
//! let tokens = vec![
//!     Token::Word("echo".to_string()),
//!     Token::Word("hello".to_string()),
//! ];
//!
//! // A pipeline with redirection
//! let pipeline = vec![
//!     Token::Word("cat".to_string()),
//!     Token::RedirIn,
//!     Token::Word("input.txt".to_string()),
//!     Token::Pipe,
//!     Token::Word("grep".to_string()),
//!     Token::Word("pattern".to_string()),
//! ];
//! ```

/// Represents a lexical token in shell syntax.
///
/// Each variant corresponds to a specific syntactic element that can appear
/// in shell commands. The lexer produces a stream of these tokens which the
/// parser then uses to build an AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// A word token representing command names, arguments, or other text.
    /// This is the most common token type.
    Word(String),

    /// Pipe operator `|` - connects stdout of one command to stdin of another.
    Pipe,

    /// Output redirection `>` - redirects stdout to a file.
    RedirOut,

    /// Output redirection with noclobber override `>|` - forces overwrite even with noclobber set.
    RedirOutClobber,

    /// Input redirection `<` - redirects stdin from a file.
    RedirIn,

    /// Append redirection `>>` - appends stdout to a file.
    RedirAppend,

    /// Here-document `<<DELIMITER` - multi-line input redirection.
    /// The boolean indicates if the delimiter was quoted (affects expansion).
    RedirHereDoc(String, bool),

    /// Here-string `<<<"content"` - single-line input redirection.
    RedirHereString(String),

    // File descriptor redirections
    /// Redirect file descriptor N from file: `N<file`
    RedirectFdIn(i32, String),

    /// Redirect file descriptor N to file: `N>file`
    RedirectFdOut(i32, String),

    /// Redirect file descriptor N to file with noclobber override: `N>|file`
    RedirectFdOutClobber(i32, String),

    /// Append file descriptor N to file: `N>>file`
    RedirectFdAppend(i32, String),

    /// Duplicate file descriptor: `N>&M` or `N<&M`
    RedirectFdDup(i32, i32),

    /// Close file descriptor: `N>&-` or `N<&-`
    RedirectFdClose(i32),

    /// Open file descriptor for read/write: `N<>file`
    RedirectFdInOut(i32, String),

    // Control flow keywords
    /// `if` keyword - starts a conditional statement.
    If,

    /// `then` keyword - begins the consequent clause of an if statement.
    Then,

    /// `else` keyword - begins the alternative clause of an if statement.
    Else,

    /// `elif` keyword - else-if for chained conditionals.
    Elif,

    /// `fi` keyword - ends an if statement.
    Fi,

    /// `case` keyword - starts a case statement.
    Case,

    /// `in` keyword - used in case and for statements.
    In,

    /// `esac` keyword - ends a case statement.
    Esac,

    /// Double semicolon `;;` - terminates a case clause.
    DoubleSemicolon,

    /// Semicolon `;` - command separator.
    Semicolon,

    /// Right parenthesis `)` - used in case patterns and subshells.
    RightParen,

    /// Left parenthesis `(` - starts a subshell or case pattern.
    LeftParen,

    /// Left brace `{` - starts a command group.
    LeftBrace,

    /// Right brace `}` - ends a command group.
    RightBrace,

    /// Newline - line terminator, also acts as command separator.
    Newline,

    /// `local` keyword - declares local variables in functions.
    Local,

    /// `return` keyword - returns from a function with an exit code.
    Return,

    /// `for` keyword - starts a for loop.
    For,

    /// `do` keyword - begins the body of a loop.
    Do,

    /// `done` keyword - ends a loop.
    Done,

    /// `while` keyword - starts a while loop.
    While,

    /// `until` keyword - starts an until loop.
    Until,

    /// `break` keyword - exits from a loop.
    Break,

    /// `continue` keyword - skips to next iteration of a loop.
    Continue,

    /// AND operator `&&` - executes next command only if previous succeeded.
    And,

    /// OR operator `||` - executes next command only if previous failed.
    Or,

    /// Bang operator `!` - negates the exit status of a command.
    Bang,
}

/// Map a keyword string to its corresponding shell Token.
///
/// This function is used during lexical analysis to identify reserved words
/// that should be treated as keywords rather than regular word tokens.
///
/// # Arguments
///
/// * `word` - The string to check for keyword status
///
/// # Returns
///
/// `Some(Token::X)` if `word` matches a recognized shell keyword (for example: `if`, `then`,
/// `else`, `elif`, `fi`, `case`, `in`, `esac`, `local`, `return`, `for`, `while`, `until`,
/// `break`, `continue`, `do`, `done`), `None` otherwise.
///
/// # Examples
///
/// ```
/// // Note: is_keyword is a private function
/// // This example is for documentation only
/// ```
pub(super) fn is_keyword(word: &str) -> Option<Token> {
    match word {
        "if" => Some(Token::If),
        "then" => Some(Token::Then),
        "else" => Some(Token::Else),
        "elif" => Some(Token::Elif),
        "fi" => Some(Token::Fi),
        "case" => Some(Token::Case),
        "in" => Some(Token::In),
        "esac" => Some(Token::Esac),
        "local" => Some(Token::Local),
        "return" => Some(Token::Return),
        "for" => Some(Token::For),
        "while" => Some(Token::While),
        "until" => Some(Token::Until),
        "break" => Some(Token::Break),
        "continue" => Some(Token::Continue),
        "do" => Some(Token::Do),
        "done" => Some(Token::Done),
        _ => None,
    }
}

/// Check if a word is a shell keyword (public API for builtins).
///
/// This includes both keywords recognized by the lexer and special tokens.
/// Used by the `type` builtin to identify reserved words.
///
/// # Arguments
///
/// * `word` - The string to check
///
/// # Returns
///
/// `true` if the word is a shell keyword, `false` otherwise.
///
/// # Examples
///
/// ```
/// use rush_sh::lexer::is_shell_keyword;
///
/// assert!(is_shell_keyword("if"));
/// assert!(is_shell_keyword("while"));
/// assert!(is_shell_keyword("{"));
/// assert!(!is_shell_keyword("echo"));
/// ```
pub fn is_shell_keyword(word: &str) -> bool {
    // Check lexer keywords first
    if is_keyword(word).is_some() {
        return true;
    }

    // Check additional POSIX keywords and special tokens
    // These are handled as separate tokens but should be recognized as keywords by `type`
    matches!(word, "until" | "{" | "}" | "!")
}