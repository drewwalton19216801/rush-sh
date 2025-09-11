use rustyline::completion::{Completer, Candidate};
use rustyline::validate::Validator;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::{Context, Helper};
use std::env;
use std::fs;
use std::path::Path;

pub struct RushCompleter {}

impl RushCompleter {
    pub fn new() -> Self {
        Self {}
    }

    fn get_builtin_commands() -> Vec<String> {
        vec![
            "cd".to_string(),
            "echo".to_string(),
            "pwd".to_string(),
            "env".to_string(),
            "exit".to_string(),
            "help".to_string(),
            "source".to_string(),
        ]
    }

    fn get_path_executables() -> Vec<String> {
        let mut executables = Vec::new();

        if let Ok(path_var) = env::var("PATH") {
            for dir in env::split_paths(&path_var) {
                if let Ok(entries) = fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_file() {
                                if let Some(name) = entry.file_name().to_str() {
                                    // Check if executable (on Unix-like systems)
                                    #[cfg(unix)]
                                    {
                                        use std::os::unix::fs::PermissionsExt;
                                        if let Ok(metadata) = entry.metadata() {
                                            let permissions = metadata.permissions();
                                            if permissions.mode() & 0o111 != 0 {
                                                executables.push(name.to_string());
                                            }
                                        }
                                    }
                                    #[cfg(windows)]
                                    {
                                        // On Windows, check for .exe, .bat, .cmd extensions
                                        if name.ends_with(".exe") || name.ends_with(".bat") || name.ends_with(".cmd") {
                                            let name_without_ext = name.trim_end_matches(".exe")
                                                .trim_end_matches(".bat")
                                                .trim_end_matches(".cmd");
                                            executables.push(name_without_ext.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        executables.sort();
        executables.dedup();
        executables
    }

    fn is_first_word(line: &str, pos: usize) -> bool {
        let before_cursor = &line[..pos];
        let words_before: Vec<&str> = before_cursor.split_whitespace().collect();
        words_before.is_empty() || (words_before.len() == 1 && !before_cursor.ends_with(' '))
    }

    fn get_command_candidates(prefix: &str) -> Vec<RushCandidate> {
        let mut candidates = Vec::new();

        // Add built-ins
        for builtin in Self::get_builtin_commands() {
            if builtin.starts_with(prefix) {
                candidates.push(RushCandidate::new(builtin.clone(), builtin));
            }
        }

        // Add PATH executables
        for executable in Self::get_path_executables() {
            if executable.starts_with(prefix) {
                candidates.push(RushCandidate::new(executable.clone(), executable));
            }
        }

        candidates.sort_by(|a, b| a.display.cmp(&b.display));
        candidates.dedup_by(|a, b| a.display == b.display);
        candidates
    }

    fn get_file_candidates(line: &str, pos: usize) -> Vec<RushCandidate> {
        let before_cursor = &line[..pos];
        let words: Vec<&str> = before_cursor.split_whitespace().collect();

        if words.is_empty() {
            return vec![];
        }

        // Find the current word being completed
        let mut current_word = String::new();
        let mut start_pos = 0;

        for (_i, &word) in words.iter().enumerate() {
            let word_start = line[start_pos..].find(word).unwrap_or(0) + start_pos;
            let word_end = word_start + word.len();

            if pos >= word_start && pos <= word_end {
                current_word = word.to_string();
                break;
            }
            start_pos = word_end;
        }

        // If we're at the end and there's a space, we're starting a new word
        if before_cursor.ends_with(' ') {
            current_word = "".to_string();
        }

        // For now, use simple file completion from current directory
        // TODO: Implement full directory traversal support
        let mut candidates = Vec::new();
        let current_dir = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());

        if let Ok(entries) = fs::read_dir(&current_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(&current_word) {
                        candidates.push(RushCandidate::new(name.to_string(), name.to_string()));
                    }
                }
            }
        }

        candidates.sort_by(|a, b| a.display.cmp(&b.display));
        candidates
    }
}

impl Completer for RushCompleter {
    type Candidate = RushCandidate;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<RushCandidate>)> {
        let prefix = &line[..pos];
        let last_space = prefix.rfind(' ').unwrap_or(0);
        let start = if last_space > 0 { last_space + 1 } else { 0 };

        let candidates = if Self::is_first_word(line, pos) {
            Self::get_command_candidates(&line[start..pos])
        } else {
            Self::get_file_candidates(line, pos)
        };

        Ok((start, candidates))
    }
}

impl Validator for RushCompleter {}

impl Highlighter for RushCompleter {}

impl Hinter for RushCompleter {
    type Hint = String;
}

impl Helper for RushCompleter {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_commands() {
        let commands = RushCompleter::get_builtin_commands();
        assert!(commands.contains(&"cd".to_string()));
        assert!(commands.contains(&"echo".to_string()));
        assert!(commands.contains(&"pwd".to_string()));
        assert!(commands.contains(&"exit".to_string()));
        assert!(commands.contains(&"help".to_string()));
        assert!(commands.contains(&"source".to_string()));
    }

    #[test]
    fn test_get_command_candidates() {
        let candidates = RushCompleter::get_command_candidates("e");
        // Should include echo, env, exit
        let displays: Vec<String> = candidates.iter().map(|c| c.display.clone()).collect();
        assert!(displays.contains(&"echo".to_string()));
        assert!(displays.contains(&"env".to_string()));
        assert!(displays.contains(&"exit".to_string()));
    }

    #[test]
    fn test_get_command_candidates_exact() {
        let candidates = RushCompleter::get_command_candidates("cd");
        let displays: Vec<String> = candidates.iter().map(|c| c.display.clone()).collect();
        assert!(displays.contains(&"cd".to_string()));
    }

    #[test]
    fn test_is_first_word() {
        assert!(RushCompleter::is_first_word("", 0));
        assert!(RushCompleter::is_first_word("c", 1));
        assert!(RushCompleter::is_first_word("cd", 2));
        assert!(!RushCompleter::is_first_word("cd ", 3));
        assert!(!RushCompleter::is_first_word("cd /", 4));
    }

    #[test]
    fn test_rush_candidate_display() {
        let candidate = RushCandidate::new("test".to_string(), "replacement".to_string());
        assert_eq!(candidate.display(), "test");
        assert_eq!(candidate.replacement(), "replacement");
    }
}

#[derive(Debug, Clone)]
pub struct RushCandidate {
    pub display: String,
    pub replacement: String,
}

impl RushCandidate {
    pub fn new(display: String, replacement: String) -> Self {
        Self { display, replacement }
    }
}

impl Candidate for RushCandidate {
    fn display(&self) -> &str {
        &self.display
    }

    fn replacement(&self) -> &str {
        &self.replacement
    }
}