//! Abstract Syntax Tree (AST) definitions for the Rush shell parser.
//!
//! This module contains the core data structures that represent parsed shell commands
//! and constructs. The AST serves as an intermediate representation between the lexical
//! tokens and the execution engine.
//!
//! # AST Structure
//!
//! The AST is built around the [`Ast`] enum, which represents different types of shell
//! constructs:
//!
//! ## Simple Commands
//! - **Pipeline**: A sequence of commands connected by pipes (`|`)
//! - **Assignment**: Variable assignment (`VAR=value`)
//! - **LocalAssignment**: Local variable declaration (`local VAR=value`)
//!
//! ## Control Flow
//! - **If**: Conditional execution with optional elif/else branches
//! - **Case**: Pattern matching construct
//! - **For**: Iteration over a list of items
//! - **While**: Loop while condition is true
//! - **Until**: Loop until condition is true
//!
//! ## Functions
//! - **FunctionDefinition**: Function declaration (`name() { ... }`)
//! - **FunctionCall**: Function invocation with arguments
//! - **Return**: Early return from function
//!
//! ## Logical Operators
//! - **And**: Short-circuit AND (`&&`)
//! - **Or**: Short-circuit OR (`||`)
//! - **Negation**: Logical NOT (`!`)
//!
//! ## Compound Commands
//! - **Subshell**: Commands executed in isolated state (`(...)`)
//! - **CommandGroup**: Commands executed in current state (`{...}`)
//! - **Sequence**: Multiple commands separated by `;` or newlines
//!
//! # Redirections
//!
//! The [`Redirection`] enum represents I/O redirection operations:
//! - Basic redirections: `<`, `>`, `>>`
//! - File descriptor operations: `N<`, `N>`, `N>>`, `N>&M`, `N<&M`, `N>&-`, `N<&-`, `N<>`
//! - Here-documents: `<<EOF`, `<<<string`
//! - Noclobber override: `>|`
//!
//! # Shell Commands
//!
//! The [`ShellCommand`] struct represents a single command in a pipeline, containing:
//! - Command arguments
//! - Ordered list of redirections (processed left-to-right per POSIX)
//! - Optional compound command (for subshells, groups, etc.)
//!
//! # Examples
//!
//! ```rust,ignore
//! // Simple command: echo hello
//! Ast::Pipeline(vec![ShellCommand {
//!     args: vec!["echo".to_string(), "hello".to_string()],
//!     redirections: vec![],
//!     compound: None,
//! }])
//!
//! // Pipeline: ls | grep txt
//! Ast::Pipeline(vec![
//!     ShellCommand { args: vec!["ls".to_string()], ... },
//!     ShellCommand { args: vec!["grep".to_string(), "txt".to_string()], ... },
//! ])
//!
//! // Conditional: if true; then echo yes; fi
//! Ast::If {
//!     branches: vec![(
//!         Box::new(Ast::Pipeline(...)),  // condition
//!         Box::new(Ast::Pipeline(...)),  // then branch
//!     )],
//!     else_branch: None,
//! }
//! ```

/// Abstract Syntax Tree node representing a parsed shell construct.
///
/// Each variant represents a different type of shell command or control structure.
/// The AST is designed to be executed by the executor module, which traverses the
/// tree and performs the corresponding operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ast {
    /// A pipeline of one or more commands connected by pipes.
    ///
    /// Each command in the pipeline receives input from the previous command's
    /// output (except the first) and sends output to the next command (except the last).
    ///
    /// # Examples
    /// - Single command: `ls`
    /// - Pipeline: `ls | grep txt | sort`
    Pipeline(Vec<ShellCommand>),

    /// A sequence of commands separated by semicolons or newlines.
    ///
    /// Commands are executed in order, regardless of their exit status.
    ///
    /// # Examples
    /// - `cmd1; cmd2; cmd3`
    /// - Multiple lines in a script
    Sequence(Vec<Ast>),

    /// Variable assignment in the global scope.
    ///
    /// # Examples
    /// - `VAR=value`
    /// - `PATH=/usr/bin:$PATH`
    Assignment {
        /// Variable name
        var: String,
        /// Value to assign
        value: String,
    },

    /// Local variable assignment (function scope).
    ///
    /// Only valid within function bodies. Creates a variable that is local
    /// to the function and its callees.
    ///
    /// # Examples
    /// - `local VAR=value`
    /// - `local COUNT=0`
    LocalAssignment {
        /// Variable name
        var: String,
        /// Value to assign
        value: String,
    },

    /// Conditional execution with optional elif and else branches.
    ///
    /// Each branch consists of a condition and a body. Branches are evaluated
    /// in order until one condition succeeds, then its body is executed.
    ///
    /// # Examples
    /// - `if test -f file; then echo exists; fi`
    /// - `if [ $x -eq 0 ]; then echo zero; elif [ $x -gt 0 ]; then echo positive; else echo negative; fi`
    If {
        /// List of (condition, then-body) pairs for if/elif branches
        branches: Vec<(Box<Ast>, Box<Ast>)>,
        /// Optional else branch
        else_branch: Option<Box<Ast>>,
    },

    /// Pattern matching construct.
    ///
    /// Matches a word against a series of patterns and executes the corresponding
    /// commands for the first match. Supports glob patterns.
    ///
    /// # Examples
    /// - `case $var in pattern1) cmd1 ;; pattern2|pattern3) cmd2 ;; esac`
    Case {
        /// Word to match against patterns
        word: String,
        /// List of (patterns, commands) pairs
        cases: Vec<(Vec<String>, Ast)>,
        /// Optional default case (pattern: `*`)
        default: Option<Box<Ast>>,
    },

    /// For loop iterating over a list of items.
    ///
    /// Executes the body once for each item, with the loop variable set to
    /// the current item.
    ///
    /// # Examples
    /// - `for i in 1 2 3; do echo $i; done`
    /// - `for file in *.txt; do cat "$file"; done`
    For {
        /// Loop variable name
        variable: String,
        /// List of items to iterate over
        items: Vec<String>,
        /// Loop body
        body: Box<Ast>,
    },

    /// While loop executing while condition is true.
    ///
    /// Evaluates the condition before each iteration. Continues looping
    /// as long as the condition exits with status 0.
    ///
    /// # Examples
    /// - `while true; do echo loop; done`
    /// - `while [ $count -lt 10 ]; do count=$((count + 1)); done`
    While {
        /// Condition to evaluate
        condition: Box<Ast>,
        /// Loop body
        body: Box<Ast>,
    },

    /// Until loop executing until condition is true.
    ///
    /// Evaluates the condition before each iteration. Continues looping
    /// as long as the condition exits with non-zero status.
    ///
    /// # Examples
    /// - `until false; do echo loop; done`
    /// - `until [ -f ready.txt ]; do sleep 1; done`
    Until {
        /// Condition to evaluate
        condition: Box<Ast>,
        /// Loop body
        body: Box<Ast>,
    },

    /// Function definition.
    ///
    /// Defines a named function that can be called later. The function body
    /// is stored as an AST and executed when the function is invoked.
    ///
    /// # Examples
    /// - `myfunc() { echo hello; }`
    /// - `greet() { echo "Hello, $1"; }`
    FunctionDefinition {
        /// Function name
        name: String,
        /// Function body
        body: Box<Ast>,
    },

    /// Function call with arguments.
    ///
    /// Invokes a previously defined function with the given arguments.
    /// Arguments are accessible as positional parameters ($1, $2, etc.).
    ///
    /// # Examples
    /// - `myfunc`
    /// - `greet Alice`
    FunctionCall {
        /// Function name
        name: String,
        /// Arguments to pass
        args: Vec<String>,
    },

    /// Return statement for early exit from function.
    ///
    /// Exits the current function with an optional exit code.
    /// If no value is provided, returns the exit code of the last command.
    ///
    /// # Examples
    /// - `return`
    /// - `return 0`
    /// - `return 1`
    Return {
        /// Optional exit code (defaults to last command's exit code)
        value: Option<String>,
    },

    /// Logical AND operator (short-circuit evaluation).
    ///
    /// Executes the right side only if the left side succeeds (exit code 0).
    ///
    /// # Examples
    /// - `cmd1 && cmd2`
    /// - `test -f file && cat file`
    And {
        /// Left operand
        left: Box<Ast>,
        /// Right operand (executed only if left succeeds)
        right: Box<Ast>,
    },

    /// Logical OR operator (short-circuit evaluation).
    ///
    /// Executes the right side only if the left side fails (non-zero exit code).
    ///
    /// # Examples
    /// - `cmd1 || cmd2`
    /// - `test -f file || touch file`
    Or {
        /// Left operand
        left: Box<Ast>,
        /// Right operand (executed only if left fails)
        right: Box<Ast>,
    },

    /// Subshell execution: `(commands)`.
    ///
    /// Commands execute in an isolated copy of the shell state. Changes to
    /// variables, directory, etc. do not affect the parent shell.
    ///
    /// # Examples
    /// - `(cd /tmp; ls)`
    /// - `(export VAR=value; cmd)`
    Subshell {
        /// Commands to execute in subshell
        body: Box<Ast>,
    },

    /// Command group execution: `{ commands; }`.
    ///
    /// Commands execute in the current shell state. Changes to variables,
    /// directory, etc. affect the current shell.
    ///
    /// # Examples
    /// - `{ cmd1; cmd2; }`
    /// - `{ echo start; cmd; echo end; }`
    CommandGroup {
        /// Commands to execute in current shell
        body: Box<Ast>,
    },

    /// Command negation: `! command`.
    ///
    /// Inverts the exit code of the command (0 becomes non-zero, non-zero becomes 0).
    /// Also exempts the command from errexit behavior.
    ///
    /// # Examples
    /// - `! false`
    /// - `! grep pattern file`
    Negation {
        /// Command to negate
        command: Box<Ast>,
    },

    /// Asynchronous command execution: `command &`.
    ///
    /// Executes the command in the background, allowing the shell to continue
    /// processing subsequent commands without waiting for completion.
    ///
    /// # Examples
    /// - `sleep 10 &`
    /// - `long_running_task &`
    AsyncCommand {
        /// Command to execute asynchronously
        command: Box<Ast>,
    },
}

/// Represents a single I/O redirection operation.
///
/// Redirections are processed in left-to-right order as they appear in the command,
/// per POSIX specification. Each redirection modifies the file descriptor table
/// before command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Redirection {
    /// Input from file: `< file` or `0< file`.
    ///
    /// Redirects standard input (fd 0) from the specified file.
    Input(String),

    /// Output to file: `> file` or `1> file`.
    ///
    /// Redirects standard output (fd 1) to the specified file, truncating it.
    /// Respects the noclobber option if set.
    Output(String),

    /// Output to file with noclobber override: `>| file`.
    ///
    /// Redirects standard output to the specified file, truncating it.
    /// Ignores the noclobber option.
    OutputClobber(String),

    /// Append to file: `>> file` or `1>> file`.
    ///
    /// Redirects standard output (fd 1) to the specified file, appending to it.
    Append(String),

    /// Input from file with explicit fd: `N< file`.
    ///
    /// Redirects the specified file descriptor from the file.
    FdInput(i32, String),

    /// Output to file with explicit fd: `N> file`.
    ///
    /// Redirects the specified file descriptor to the file, truncating it.
    /// Respects the noclobber option if set.
    FdOutput(i32, String),

    /// Output to file with explicit fd and noclobber override: `N>| file`.
    ///
    /// Redirects the specified file descriptor to the file, truncating it.
    /// Ignores the noclobber option.
    FdOutputClobber(i32, String),

    /// Append to file with explicit fd: `N>> file`.
    ///
    /// Redirects the specified file descriptor to the file, appending to it.
    FdAppend(i32, String),

    /// Duplicate file descriptor: `N>&M` or `N<&M`.
    ///
    /// Makes file descriptor N a copy of file descriptor M.
    /// Both descriptors refer to the same open file description.
    FdDuplicate(i32, i32),

    /// Close file descriptor: `N>&-` or `N<&-`.
    ///
    /// Closes the specified file descriptor.
    FdClose(i32),

    /// Open file for read/write: `N<> file`.
    ///
    /// Opens the file for both reading and writing on the specified fd.
    FdInputOutput(i32, String),

    /// Here-document: `<< EOF ... EOF`.
    ///
    /// Provides input from a multi-line string literal.
    /// The first string is the delimiter, the second is the content.
    /// The boolean indicates whether the delimiter was quoted (affects expansion).
    HereDoc(String, String),

    /// Here-string: `<<< "string"`.
    ///
    /// Provides input from a single-line string.
    HereString(String),
}

/// Represents a single command in a pipeline.
///
/// A shell command consists of:
/// - Arguments (command name and parameters)
/// - Redirections (I/O redirection operations)
/// - Optional compound command (for subshells, groups, etc.)
///
/// If `compound` is present, it takes precedence over `args` during execution.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellCommand {
    /// Command arguments (first element is the command name).
    ///
    /// For simple commands like `ls -la`, this would be `["ls", "-la"]`.
    pub args: Vec<String>,

    /// All redirections in order of appearance.
    ///
    /// Redirections are processed left-to-right per POSIX specification.
    /// For example, in `cmd >file1 2>&1 >file2`, the redirections are:
    /// 1. Redirect stdout to file1
    /// 2. Duplicate stderr to stdout (which points to file1)
    /// 3. Redirect stdout to file2 (stderr still points to file1)
    pub redirections: Vec<Redirection>,

    /// Optional compound command (subshell, command group, etc.).
    ///
    /// If present, this takes precedence over `args` during execution.
    /// Used for constructs like `(subshell) | cmd` or `{ group; } | cmd`.
    pub compound: Option<Box<Ast>>,
}