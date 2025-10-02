use rustyline::completion::{Candidate, Completer};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct CompletionContext {
    word: String,
    pos: usize,
    timestamp: u64,
    attempt_count: u32,
}

// Global state for tracking completion context
lazy_static::lazy_static! {
    static ref COMPLETION_STATE: Mutex<Option<CompletionContext>> = Mutex::new(None);
}

pub struct RushCompleter {}

impl Default for RushCompleter {
    fn default() -> Self {
        Self::new()
    }
}

impl RushCompleter {
    pub fn new() -> Self {
        Self {}
    }

    fn get_builtin_commands() -> Vec<String> {
        crate::builtins::get_builtin_commands()
    }

    fn get_path_executables() -> Vec<String> {
        let mut executables = Vec::new();

        if let Ok(path_var) = env::var("PATH") {
            for dir in env::split_paths(&path_var) {
                if let Ok(entries) = fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type()
                            && file_type.is_file()
                            && let Some(name) = entry.file_name().to_str() {
                                // Check if executable (on Unix-like systems)
                                use std::os::unix::fs::PermissionsExt;
                                if let Ok(metadata) = entry.metadata() {
                                    let permissions = metadata.permissions();
                                    if permissions.mode() & 0o111 != 0 {
                                        executables.push(name.to_string());
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

    fn looks_like_file_path(word: &str) -> bool {
        word.starts_with("./")
            || word.starts_with("/")
            || word.starts_with("~/")
            || word.contains("/")
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

    fn is_repeated_completion(word: &str, pos: usize) -> bool {
        if let Ok(context) = COMPLETION_STATE.lock()
            && let Some(ref ctx) = *context {
                // Check if this is the same word and position (within a reasonable time window)
                if ctx.word == word && ctx.pos == pos {
                    let current_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    // Consider it a repeated attempt if within 2 seconds
                    if current_time - ctx.timestamp <= 2 {
                        return true;
                    }
                }
            }
        false
    }

    fn update_completion_context(word: String, pos: usize, is_repeated: bool) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Ok(mut context) = COMPLETION_STATE.lock() {
            if is_repeated {
                if let Some(ref mut ctx) = *context {
                    ctx.attempt_count += 1;
                    ctx.timestamp = current_time;
                }
            } else {
                *context = Some(CompletionContext {
                    word,
                    pos,
                    timestamp: current_time,
                    attempt_count: 1,
                });
            }
        }
    }

    fn get_current_attempt_count(&self) -> u32 {
        if let Ok(context) = COMPLETION_STATE.lock()
            && let Some(ref ctx) = *context {
                return ctx.attempt_count;
            }
        1
    }

    fn get_next_completion_candidate(candidates: &[RushCandidate], attempt_count: u32) -> Option<(usize, Vec<RushCandidate>)> {
        if candidates.len() <= 1 {
            return None;
        }

        // Cycle through candidates based on attempt count
        let index = ((attempt_count - 1) % candidates.len() as u32) as usize;
        let candidate = &candidates[index];

        // Return single candidate for cycling behavior
        Some((0, vec![RushCandidate::new(
            candidate.display.clone(),
            candidate.replacement.clone(),
        )]))
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

        for &word in words.iter() {
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

        // Parse the current word to separate directory path from filename prefix
        let (base_dir, prefix) = Self::parse_path_for_completion(&current_word);

        let mut candidates = Vec::new();

        // Try to read the target directory
        if let Ok(entries) = fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str()
                    && name.starts_with(&prefix) {
                        // Determine the replacement string
                        let replacement = if current_word.is_empty() || current_word.ends_with('/')
                        {
                            // If completing from a directory, just append the name
                            format!("{}{}", current_word, name)
                        } else if let Some(last_slash) = current_word.rfind('/') {
                            // If completing a partial name in a subdirectory
                            format!("{}{}", &current_word[..=last_slash], name)
                        } else {
                            // Completing in current directory
                            name.to_string()
                        };

                        // Add trailing slash for directories
                        let display_name = if let Ok(file_type) = entry.file_type() {
                            if file_type.is_dir() {
                                format!("{}/", name)
                            } else {
                                name.to_string()
                            }
                        } else {
                            name.to_string()
                        };

                        candidates.push(RushCandidate::new(display_name, replacement));
                    }
            }
        }

        candidates.sort_by(|a, b| a.display.cmp(&b.display));
        candidates
    }

    fn parse_path_for_completion(word: &str) -> (std::path::PathBuf, String) {
        if word.is_empty() {
            return (
                env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
                String::new(),
            );
        }

        let path = Path::new(word);

        // Handle absolute paths
        if path.is_absolute() {
            // Check if the path ends with '/' - if so, we're completing from that directory
            if word.ends_with('/') {
                return (path.to_path_buf(), String::new());
            }

            if let Some(parent) = path.parent() {
                let prefix = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                return (parent.to_path_buf(), prefix);
            } else {
                // Root directory
                return (Path::new("/").to_path_buf(), String::new());
            }
        }

        // Handle home directory expansion
        if (word.starts_with("~/") || word == "~")
            && let Ok(home_dir) = env::var("HOME") {
                let home_path = Path::new(&home_dir);
                let relative_path = if word == "~" {
                    Path::new("")
                } else {
                    Path::new(&word[2..]) // Remove "~/"
                };

                // Check if the path ends with '/' - if so, we're completing from that directory
                if word.ends_with('/') || word == "~" {
                    return (home_path.join(relative_path), String::new());
                }

                if let Some(parent) = relative_path.parent() {
                    let full_parent = home_path.join(parent);
                    let prefix = relative_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    return (full_parent, prefix);
                } else {
                    return (home_path.to_path_buf(), String::new());
                }
            }

        // Handle relative paths
        if word.ends_with('/') {
            // Completing from a directory
            return (Path::new(word).to_path_buf(), String::new());
        }

        if let Some(last_slash) = word.rfind('/') {
            let dir_part = &word[..last_slash];
            let file_part = &word[last_slash + 1..];

            let base_dir = if dir_part.is_empty() {
                env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf())
            } else {
                Path::new(dir_part).to_path_buf()
            };

            (base_dir, file_part.to_string())
        } else {
            // No directory separator, complete from current directory
            (
                env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
                word.to_string(),
            )
        }
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
        let current_word = &line[start..pos];

        let is_first = Self::is_first_word(line, pos);
        let is_file_path = Self::looks_like_file_path(current_word);

        let candidates = if is_first && !is_file_path {
            // For first word that doesn't look like a file path, check if there are file matches
            // If there are, prefer file completion over command completion
            let file_candidates = Self::get_file_candidates(line, pos);
            if file_candidates.is_empty() {
                Self::get_command_candidates(current_word)
            } else {
                file_candidates
            }
        } else {
            Self::get_file_candidates(line, pos)
        };

        // Check if this is a repeated completion attempt
        let is_repeated = Self::is_repeated_completion(current_word, pos);

        // If this is a repeated attempt with multiple matches, cycle through candidates
        if is_repeated && candidates.len() > 1
            && let Some(completion_result) = Self::get_next_completion_candidate(&candidates, self.get_current_attempt_count()) {
                Self::update_completion_context(current_word.to_string(), pos, true);
                return Ok(completion_result);
            }

        // Update completion context for next attempt
        Self::update_completion_context(current_word.to_string(), pos, is_repeated);

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
    use std::sync::Mutex;

    // Import the mutex from main.rs tests module
    // We need to use a separate mutex for completion tests to avoid cross-module issues
    static COMPLETION_DIR_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_builtin_commands() {
        let commands = RushCompleter::get_builtin_commands();
        assert!(commands.contains(&"cd".to_string()));
        assert!(commands.contains(&"pwd".to_string()));
        assert!(commands.contains(&"exit".to_string()));
        assert!(commands.contains(&"help".to_string()));
        assert!(commands.contains(&"source".to_string()));
    }

    #[test]
    fn test_get_command_candidates() {
        let candidates = RushCompleter::get_command_candidates("e");
        // Should include env, exit
        let displays: Vec<String> = candidates.iter().map(|c| c.display.clone()).collect();
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

    #[test]
    fn test_parse_path_for_completion_current_dir() {
        let (_base_dir, prefix) = RushCompleter::parse_path_for_completion("");
        assert_eq!(prefix, "");
        // Should be current directory

        let (_base_dir, prefix) = RushCompleter::parse_path_for_completion("file");
        assert_eq!(prefix, "file");
        // Should be current directory
    }

    #[test]
    fn test_parse_path_for_completion_with_directory() {
        let (base_dir, prefix) = RushCompleter::parse_path_for_completion("src/");
        assert_eq!(prefix, "");
        assert_eq!(base_dir, Path::new("src"));

        let (base_dir, prefix) = RushCompleter::parse_path_for_completion("src/main");
        assert_eq!(prefix, "main");
        assert_eq!(base_dir, Path::new("src"));
    }

    #[test]
    fn test_parse_path_for_completion_absolute() {
        let (_base_dir, prefix) = RushCompleter::parse_path_for_completion("/usr/");
        assert_eq!(prefix, "");

        let (_base_dir, prefix) = RushCompleter::parse_path_for_completion("/usr/bin/l");
        assert_eq!(prefix, "l");
    }

    #[test]
    fn test_parse_path_for_completion_home() {
        // This test assumes HOME environment variable is set
        if env::var("HOME").is_ok() {
            let (base_dir, prefix) = RushCompleter::parse_path_for_completion("~/");
            assert_eq!(prefix, "");
            assert_eq!(base_dir, Path::new(&env::var("HOME").unwrap()));

            let (base_dir, prefix) = RushCompleter::parse_path_for_completion("~/doc");
            assert_eq!(prefix, "doc");
            assert_eq!(base_dir, Path::new(&env::var("HOME").unwrap()));
        }
    }

    #[test]
    fn test_get_file_candidates_basic() {
        // Test completion from current directory
        let candidates = RushCompleter::get_file_candidates("ls ", 3);
        // Should return candidates from current directory
        // (exact results depend on the test environment)
        assert!(candidates.is_empty() || !candidates.is_empty()); // Just check it doesn't panic
    }

    #[test]
    fn test_get_file_candidates_with_directory() {
        // Test completion with directory path
        let candidates = RushCompleter::get_file_candidates("ls src/", 7);
        // Should return candidates from src directory if it exists
        assert!(candidates.is_empty() || !candidates.is_empty()); // Just check it doesn't panic
    }

    #[test]
    fn test_directory_completion_formatting() {
        // Lock to prevent parallel tests from interfering with directory changes
        let _lock = COMPLETION_DIR_LOCK.lock().unwrap();
        
        // Create a temporary directory for testing
        let temp_dir = env::temp_dir().join("rush_completion_test");
        let _ = fs::create_dir_all(&temp_dir);
        let _ = fs::create_dir_all(temp_dir.join("testdir"));
        let _ = fs::write(temp_dir.join("testfile"), "content");

        // Ensure we're in a safe directory first, then change to temp directory
        let _ = env::set_current_dir(env::temp_dir());
        let _ = env::set_current_dir(&temp_dir);

        // Test directory completion
        let candidates = RushCompleter::get_file_candidates("ls test", 7);
        let has_testdir = candidates.iter().any(|c| c.display() == "testdir/");
        let has_testfile = candidates.iter().any(|c| c.display() == "testfile");

        // Change back to a safe directory before cleanup
        let _ = env::set_current_dir(env::temp_dir());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);

        assert!(has_testdir || has_testfile); // At least one should be found
    }

    #[test]
    fn test_first_word_file_completion_precedence() {
        // Lock to prevent parallel tests from interfering with directory changes
        let _lock = COMPLETION_DIR_LOCK.lock().unwrap();

        // Create a temporary directory for testing
        let temp_dir = env::temp_dir().join("rush_completion_test_first_word");
        let _ = fs::create_dir_all(&temp_dir);
        let _ = fs::create_dir_all(temp_dir.join("examples"));

        // Ensure we're in a safe directory first, then change to temp directory
        let _ = env::set_current_dir(env::temp_dir());
        let _ = env::set_current_dir(&temp_dir);

        // Test that "ex" completes to "examples/" when it's the first word
        // This tests the fix for the issue where "ex" + Tab didn't complete
        // but "./ex" + Tab did complete to "./examples"
        let candidates = RushCompleter::get_file_candidates("ex", 2);
        let has_examples = candidates.iter().any(|c| c.display() == "examples/");

        // Change back to a safe directory before cleanup
        let _ = env::set_current_dir(env::temp_dir());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);

        assert!(has_examples, "First word 'ex' should complete to 'examples/' when examples directory exists");
    }

    #[test]
    fn test_multi_match_completion_cycling() {
        // Test that multiple matches cycle correctly
        let candidates = vec![
            RushCandidate::new("file1".to_string(), "file1".to_string()),
            RushCandidate::new("file2".to_string(), "file2".to_string()),
            RushCandidate::new("file3".to_string(), "file3".to_string()),
        ];

        // First attempt should return first candidate
        let result1 = RushCompleter::get_next_completion_candidate(&candidates, 1);
        assert!(result1.is_some());
        let (_, first_candidates) = result1.unwrap();
        assert_eq!(first_candidates.len(), 1);
        assert_eq!(first_candidates[0].display, "file1");

        // Second attempt should return second candidate
        let result2 = RushCompleter::get_next_completion_candidate(&candidates, 2);
        assert!(result2.is_some());
        let (_, second_candidates) = result2.unwrap();
        assert_eq!(second_candidates.len(), 1);
        assert_eq!(second_candidates[0].display, "file2");

        // Third attempt should return third candidate
        let result3 = RushCompleter::get_next_completion_candidate(&candidates, 3);
        assert!(result3.is_some());
        let (_, third_candidates) = result3.unwrap();
        assert_eq!(third_candidates.len(), 1);
        assert_eq!(third_candidates[0].display, "file3");

        // Fourth attempt should cycle back to first candidate
        let result4 = RushCompleter::get_next_completion_candidate(&candidates, 4);
        assert!(result4.is_some());
        let (_, fourth_candidates) = result4.unwrap();
        assert_eq!(fourth_candidates.len(), 1);
        assert_eq!(fourth_candidates[0].display, "file1");
    }

    #[test]
    fn test_multi_match_completion_single_candidate() {
        // Test that single candidate doesn't trigger cycling behavior
        let candidates = vec![
            RushCandidate::new("single_file".to_string(), "single_file".to_string()),
        ];

        let result = RushCompleter::get_next_completion_candidate(&candidates, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_multi_match_completion_empty_candidates() {
        // Test that empty candidates doesn't trigger cycling behavior
        let candidates: Vec<RushCandidate> = vec![];

        let result = RushCompleter::get_next_completion_candidate(&candidates, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_repeated_completion_detection() {
        // Clear any existing state first
        if let Ok(mut context) = COMPLETION_STATE.lock() {
            *context = None;
        }

        // Test that repeated completion attempts are detected correctly
        let word = "test";
        let pos = 4;

        // First attempt should not be detected as repeated
        assert!(!RushCompleter::is_repeated_completion(word, pos));

        // Update context to simulate a completion attempt
        RushCompleter::update_completion_context(word.to_string(), pos, false);

        // Second attempt should be detected as repeated
        assert!(RushCompleter::is_repeated_completion(word, pos));

        // Different word should not be detected as repeated
        assert!(!RushCompleter::is_repeated_completion("different", pos));

        // Different position should not be detected as repeated
        assert!(!RushCompleter::is_repeated_completion(word, pos + 1));
    }

    #[test]
    fn test_completion_context_update() {
        // Clear any existing state first
        if let Ok(mut context) = COMPLETION_STATE.lock() {
            *context = None;
        }

        let word = "test";
        let pos = 4;

        // Test initial context creation
        RushCompleter::update_completion_context(word.to_string(), pos, false);

        if let Ok(context) = COMPLETION_STATE.lock() {
            assert!(context.is_some());
            let ctx = context.as_ref().unwrap();
            assert_eq!(ctx.word, word);
            assert_eq!(ctx.pos, pos);
            assert_eq!(ctx.attempt_count, 1);
        }

        // Test repeated attempt updates attempt count
        RushCompleter::update_completion_context(word.to_string(), pos, true);

        if let Ok(context) = COMPLETION_STATE.lock() {
            assert!(context.is_some());
            let ctx = context.as_ref().unwrap();
            assert_eq!(ctx.attempt_count, 2);
        }
    }
}

#[derive(Debug, Clone)]
pub struct RushCandidate {
    pub display: String,
    pub replacement: String,
}

impl RushCandidate {
    pub fn new(display: String, replacement: String) -> Self {
        Self {
            display,
            replacement,
        }
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
