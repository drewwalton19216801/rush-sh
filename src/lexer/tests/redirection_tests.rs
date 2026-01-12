//! Redirection tokenization tests

use super::super::*;
use crate::state::ShellState;

#[test]
fn test_redirections() {
    let shell_state = ShellState::new();
    let result = lex("printf hello > output.txt", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("printf".to_string()),
            Token::Word("hello".to_string()),
            Token::RedirOut,
            Token::Word("output.txt".to_string())
        ]
    );
}

#[test]
fn test_append_redirection() {
    let shell_state = ShellState::new();
    let result = lex("printf hello >> output.txt", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("printf".to_string()),
            Token::Word("hello".to_string()),
            Token::RedirAppend,
            Token::Word("output.txt".to_string())
        ]
    );
}

#[test]
fn test_input_redirection() {
    let shell_state = ShellState::new();
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
fn test_here_document_redirection() {
    let shell_state = ShellState::new();
    let result = lex("cat << EOF", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("cat".to_string()),
            Token::RedirHereDoc("EOF".to_string(), false)
        ]
    );
}

#[test]
fn test_here_string_redirection() {
    let shell_state = ShellState::new();
    let result = lex("cat <<< \"hello world\"", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("cat".to_string()),
            Token::RedirHereString("hello world".to_string())
        ]
    );
}

#[test]
fn test_here_document_with_quoted_delimiter() {
    let shell_state = ShellState::new();
    let result = lex("command << 'EOF'", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirHereDoc("EOF".to_string(), true) // Quoted delimiter
        ]
    );
}

#[test]
fn test_here_string_without_quotes() {
    let shell_state = ShellState::new();
    let result = lex("grep <<< pattern", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("grep".to_string()),
            Token::RedirHereString("pattern".to_string())
        ]
    );
}

#[test]
fn test_redirections_mixed() {
    let shell_state = ShellState::new();
    let result = lex(
        "cat < input.txt <<< \"fallback\" > output.txt",
        &shell_state,
    )
    .unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("cat".to_string()),
            Token::RedirIn,
            Token::Word("input.txt".to_string()),
            Token::RedirHereString("fallback".to_string()),
            Token::RedirOut,
            Token::Word("output.txt".to_string())
        ]
    );
}

// ===== File Descriptor Redirection Tests =====

#[test]
fn test_fd_output_redirection() {
    let shell_state = ShellState::new();
    let result = lex("command 2>errors.log", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdOut(2, "errors.log".to_string())
        ]
    );
}

#[test]
fn test_fd_input_redirection() {
    let shell_state = ShellState::new();
    let result = lex("command 3<input.txt", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdIn(3, "input.txt".to_string())
        ]
    );
}

#[test]
fn test_fd_append_redirection() {
    let shell_state = ShellState::new();
    let result = lex("command 2>>errors.log", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdAppend(2, "errors.log".to_string())
        ]
    );
}

#[test]
fn test_fd_duplication_output() {
    let shell_state = ShellState::new();
    let result = lex("command 2>&1", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdDup(2, 1)
        ]
    );
}

#[test]
fn test_fd_duplication_input() {
    let shell_state = ShellState::new();
    let result = lex("command 0<&3", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdDup(0, 3)
        ]
    );
}

#[test]
fn test_fd_close_output() {
    let shell_state = ShellState::new();
    let result = lex("command 2>&-", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdClose(2)
        ]
    );
}

#[test]
fn test_fd_close_input() {
    let shell_state = ShellState::new();
    let result = lex("command 3<&-", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdClose(3)
        ]
    );
}

#[test]
fn test_fd_read_write() {
    let shell_state = ShellState::new();
    let result = lex("command 3<>file.txt", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdInOut(3, "file.txt".to_string())
        ]
    );
}

#[test]
fn test_fd_read_write_default() {
    let shell_state = ShellState::new();
    let result = lex("command <>file.txt", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdInOut(0, "file.txt".to_string())
        ]
    );
}

#[test]
fn test_multiple_fd_redirections() {
    let shell_state = ShellState::new();
    let result = lex("command 2>err.log 3<input.txt 4>>append.log", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdOut(2, "err.log".to_string()),
            Token::RedirectFdIn(3, "input.txt".to_string()),
            Token::RedirectFdAppend(4, "append.log".to_string())
        ]
    );
}

#[test]
fn test_fd_redirection_with_pipe() {
    let shell_state = ShellState::new();
    let result = lex("command 2>&1 | grep error", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdDup(2, 1),
            Token::Pipe,
            Token::Word("grep".to_string()),
            Token::Word("error".to_string())
        ]
    );
}

#[test]
fn test_fd_numbers_0_through_9() {
    let shell_state = ShellState::new();

    // Test fd 0
    let result = lex("cmd 0<file", &shell_state).unwrap();
    assert_eq!(result[1], Token::RedirectFdIn(0, "file".to_string()));

    // Test fd 9
    let result = lex("cmd 9>file", &shell_state).unwrap();
    assert_eq!(result[1], Token::RedirectFdOut(9, "file".to_string()));
}

#[test]
fn test_fd_swap_pattern() {
    let shell_state = ShellState::new();
    let result = lex("command 3>&1 1>&2 2>&3 3>&-", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdDup(3, 1),
            Token::RedirectFdDup(1, 2),
            Token::RedirectFdDup(2, 3),
            Token::RedirectFdClose(3)
        ]
    );
}

#[test]
fn test_backward_compat_simple_output() {
    let shell_state = ShellState::new();
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
fn test_backward_compat_simple_input() {
    let shell_state = ShellState::new();
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
fn test_backward_compat_append() {
    let shell_state = ShellState::new();
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
fn test_fd_with_spaces() {
    let shell_state = ShellState::new();
    let result = lex("command 2> errors.log", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdOut(2, "errors.log".to_string())
        ]
    );
}

#[test]
fn test_fd_no_space() {
    let shell_state = ShellState::new();
    let result = lex("command 2>errors.log", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdOut(2, "errors.log".to_string())
        ]
    );
}

#[test]
fn test_fd_dup_to_self() {
    let shell_state = ShellState::new();
    let result = lex("command 1>&1", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdDup(1, 1)
        ]
    );
}

#[test]
fn test_stderr_to_stdout() {
    let shell_state = ShellState::new();
    let result = lex("ls /nonexistent 2>&1", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("ls".to_string()),
            Token::Word("/nonexistent".to_string()),
            Token::RedirectFdDup(2, 1)
        ]
    );
}

#[test]
fn test_stdout_to_stderr() {
    let shell_state = ShellState::new();
    let result = lex("echo error 1>&2", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("error".to_string()),
            Token::RedirectFdDup(1, 2)
        ]
    );
}

#[test]
fn test_combined_redirections() {
    let shell_state = ShellState::new();
    let result = lex("command >output.txt 2>&1", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirOut,
            Token::Word("output.txt".to_string()),
            Token::RedirectFdDup(2, 1)
        ]
    );
}

#[test]
fn test_fd_with_variable_filename() {
    let shell_state = ShellState::new();
    let result = lex("command 2>$LOGFILE", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdOut(2, "$LOGFILE".to_string())
        ]
    );
}

#[test]
fn test_invalid_fd_dup_no_target() {
    let shell_state = ShellState::new();
    let result = lex("command 2>&", &shell_state);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("expected fd number or '-' after >&")
    );
}

#[test]
fn test_invalid_fd_close_input_no_dash() {
    let shell_state = ShellState::new();
    let result = lex("command 3<&", &shell_state);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("expected fd number or '-' after <&")
    );
}

#[test]
fn test_fd_inout_no_filename() {
    let shell_state = ShellState::new();
    let result = lex("command 3<>", &shell_state);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected filename after <>"));
}

#[test]
fn test_fd_output_no_filename() {
    let shell_state = ShellState::new();
    let result = lex("command 2>", &shell_state);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected filename after >"));
}

#[test]
fn test_fd_input_no_filename() {
    let shell_state = ShellState::new();
    let result = lex("command 3<", &shell_state);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected filename after <"));
}

#[test]
fn test_fd_append_no_filename() {
    let shell_state = ShellState::new();
    let result = lex("command 2>>", &shell_state);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected filename after >>"));
}

// ===== OutputClobber (>|) Tests =====

#[test]
fn test_output_clobber_basic() {
    let shell_state = ShellState::new();
    let result = lex("echo test >| output.txt", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("test".to_string()),
            Token::RedirOutClobber,
            Token::Word("output.txt".to_string())
        ]
    );
}

#[test]
fn test_output_clobber_with_fd_number() {
    let shell_state = ShellState::new();
    let result = lex("echo test 2>| errors.log", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("test".to_string()),
            Token::RedirectFdOutClobber(2, "errors.log".to_string())
        ]
    );
}

#[test]
fn test_output_clobber_with_fd_number_no_space() {
    let shell_state = ShellState::new();
    let result = lex("command 3>|file.txt", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdOutClobber(3, "file.txt".to_string())
        ]
    );
}

#[test]
fn test_output_clobber_missing_filename() {
    let shell_state = ShellState::new();
    let result = lex("echo test >|", &shell_state);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected filename after >|"));
}

#[test]
fn test_output_clobber_with_quoted_filename() {
    let shell_state = ShellState::new();
    let result = lex("echo test >| \"output file.txt\"", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("test".to_string()),
            Token::RedirOutClobber,
            Token::Word("output file.txt".to_string())
        ]
    );
}

#[test]
fn test_output_clobber_with_fd_and_quoted_filename() {
    let shell_state = ShellState::new();
    let result = lex("command 2>| 'error log.txt'", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirectFdOutClobber(2, "error log.txt".to_string())
        ]
    );
}

#[test]
fn test_output_clobber_multiple_fds() {
    let shell_state = ShellState::new();
    let result = lex("command >| out.txt 2>| err.txt", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("command".to_string()),
            Token::RedirOutClobber,
            Token::Word("out.txt".to_string()),
            Token::RedirectFdOutClobber(2, "err.txt".to_string())
        ]
    );
}

#[test]
fn test_output_clobber_with_pipe() {
    let shell_state = ShellState::new();
    let result = lex("echo test >| output.txt | cat", &shell_state).unwrap();
    assert_eq!(
        result,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("test".to_string()),
            Token::RedirOutClobber,
            Token::Word("output.txt".to_string()),
            Token::Pipe,
            Token::Word("cat".to_string())
        ]
    );
}

#[test]
fn test_output_clobber_fd_0_through_9() {
    let shell_state = ShellState::new();
    
    // Test fd 0 (unusual but valid)
    let result = lex("cmd 0>| file", &shell_state).unwrap();
    assert_eq!(result[1], Token::RedirectFdOutClobber(0, "file".to_string()));
    
    // Test fd 9
    let result = lex("cmd 9>| file", &shell_state).unwrap();
    assert_eq!(result[1], Token::RedirectFdOutClobber(9, "file".to_string()));
}

#[test]
fn test_output_clobber_consistency_with_regular_redirect() {
    let shell_state = ShellState::new();
    
    // Regular redirect without fd
    let result1 = lex("echo test > output.txt", &shell_state).unwrap();
    // Clobber redirect without fd
    let result2 = lex("echo test >| output.txt", &shell_state).unwrap();
    
    // Both should have same structure, just different redirect token
    assert_eq!(result1.len(), result2.len());
    assert_eq!(result1[0], result2[0]); // echo
    assert_eq!(result1[1], result2[1]); // test
    assert_eq!(result1[3], result2[3]); // output.txt
}