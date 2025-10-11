//! Brace expansion implementation for POSIX shell
//! Supports patterns like {a,b,c}, {1..3}, file{a,b}.txt, and nested {{a,b},{c,d}}
use super::lexer::Token;

/// Main function to expand braces in a token stream
pub fn expand_braces(tokens: Vec<Token>) -> Result<Vec<Token>, String> {
    if tokens.is_empty() {
        return Ok(tokens);
    }

    let mut result = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::Word(word) => {
                // Check if this word contains braces
                if word.contains('{') && word.contains('}') {
                    // Expand this word and add all resulting tokens
                    let expanded_words = expand_braces_in_word(word)?;
                    for expanded_word in expanded_words {
                        result.push(Token::Word(expanded_word));
                    }
                } else {
                    // No braces, keep as is
                    result.push(tokens[i].clone());
                }
            }
            _ => {
                // Non-word tokens (pipes, redirects, etc.) pass through unchanged
                result.push(tokens[i].clone());
            }
        }
        i += 1;
    }

    Ok(result)
}

/// Expand braces within a single word
fn expand_braces_in_word(word: &str) -> Result<Vec<String>, String> {
    // First, check if there are any braces at all
    if !word.contains('{') || !word.contains('}') {
        return Ok(vec![word.to_string()]);
    }

    // Find all brace patterns using proper brace matching
    let patterns = find_brace_patterns(word)?;

    if patterns.is_empty() {
        return Ok(vec![word.to_string()]);
    }

    // Generate all combinations
    let expansions = generate_expansions(patterns)?;
    Ok(expansions)
}

/// Find all brace patterns in a word, properly handling nested braces
fn find_brace_patterns(word: &str) -> Result<Vec<(String, Vec<String>, String)>, String> {
    let mut patterns = Vec::new();
    let chars: Vec<char> = word.chars().collect();
    let mut pos = 0;
    let mut last_end = 0; // Track where the last pattern ended

    while pos < chars.len() {
        if chars[pos] == '{' {
            // Find the matching closing brace
            let start = pos;
            let mut depth = 0;
            let mut end = None;

            for (i, &ch) in chars.iter().enumerate().skip(start) {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
            }

            if let Some(end_pos) = end {
                // For the first pattern, prefix is everything before it
                // For subsequent patterns, prefix is everything since the last pattern ended
                let prefix: String = if patterns.is_empty() {
                    chars[..start].iter().collect()
                } else {
                    chars[last_end..start].iter().collect()
                };

                let content: String = chars[start + 1..end_pos].iter().collect();

                // Don't include suffix yet - we'll add it for the last pattern
                let suffix = String::new();

                // Parse the content to get alternatives
                let alternatives = parse_brace_content(&content)?;

                patterns.push((prefix, alternatives, suffix));

                // Update last_end to track where this pattern ended
                last_end = end_pos + 1;

                // Move past this pattern
                pos = end_pos + 1;
            } else {
                // Unmatched brace
                return Err("Unmatched braces in pattern".to_string());
            }
        } else {
            pos += 1;
        }
    }

    // Add the final suffix to the last pattern
    if !patterns.is_empty() && last_end < chars.len() {
        let final_suffix: String = chars[last_end..].iter().collect();
        let last_idx = patterns.len() - 1;
        patterns[last_idx].2 = final_suffix;
    }

    Ok(patterns)
}

/// Parse the content inside braces {content}
fn parse_brace_content(content: &str) -> Result<Vec<String>, String> {
    let mut alternatives = Vec::new();

    // Find the top-level comma-separated parts
    let parts = split_top_level(content, ',')?;
    for part in parts {
        let part = part.trim();

        // Check if this part contains braces (nested)
        if part.starts_with('{') && part.ends_with('}') {
            // This is a nested brace pattern, recursively parse it
            let inner_content = &part[1..part.len() - 1];
            let nested_alternatives = parse_brace_content(inner_content)?;
            alternatives.extend(nested_alternatives);
        } else if part.contains("..") {
            // Handle range expansion
            let range_alternatives = expand_range(part)?;
            alternatives.extend(range_alternatives);
        } else {
            alternatives.push(part.to_string());
        }
    }

    Ok(alternatives)
}

/// Split string by delimiter but respect braces (don't split inside braces)
fn split_top_level(input: &str, _delimiter: char) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut brace_depth = 0;
    let chars = input.chars().peekable();

    for ch in chars {
        match ch {
            '{' => {
                brace_depth += 1;
                current.push(ch);
            }
            '}' => {
                brace_depth -= 1;
                current.push(ch);
            }
            ',' if brace_depth == 0 => {
                // Top-level comma
                parts.push(current);
                current = String::new();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if brace_depth != 0 {
        return Err("Unmatched braces in pattern".to_string());
    }

    if !current.is_empty() {
        parts.push(current);
    }

    Ok(parts)
}

/// Expand a range pattern like "1..3" or "a..c"
fn expand_range(pattern: &str) -> Result<Vec<String>, String> {
    let parts: Vec<&str> = pattern.split("..").collect();
    if parts.len() != 2 {
        return Err(format!("Invalid range pattern: {}", pattern));
    }

    let start = parts[0].trim();
    let end = parts[1].trim();

    // Check if both parts are single characters (for alphabetic ranges)
    if start.len() == 1
        && end.len() == 1
        && let (Some(start_ch), Some(end_ch)) = (start.chars().next(), end.chars().next())
        && start_ch.is_ascii_alphabetic()
        && end_ch.is_ascii_alphabetic()
    {
        return expand_char_range(start_ch, end_ch);
    }

    // Check if both parts are numeric (for numeric ranges)
    if let (Ok(start_num), Ok(end_num)) = (start.parse::<i64>(), end.parse::<i64>()) {
        return expand_numeric_range(start_num, end_num);
    }

    // If neither, treat as literal strings
    Ok(vec![start.to_string(), end.to_string()])
}

/// Expand a character range like 'a'..'c'
fn expand_char_range(start: char, end: char) -> Result<Vec<String>, String> {
    if !start.is_ascii_alphabetic() || !end.is_ascii_alphabetic() {
        return Err(format!("Invalid character range: {}..{}", start, end));
    }

    let mut result = Vec::new();
    let start_byte = start as u8;
    let end_byte = end as u8;

    if start_byte > end_byte {
        return Err(format!("Invalid range: {} > {}", start, end));
    }

    for byte in start_byte..=end_byte {
        if let Some(ch) = char::from_u32(byte as u32) {
            result.push(ch.to_string());
        }
    }

    Ok(result)
}

/// Expand a numeric range like 1..3
fn expand_numeric_range(start: i64, end: i64) -> Result<Vec<String>, String> {
    if start > end {
        return Err(format!("Invalid range: {} > {}", start, end));
    }

    let mut result = Vec::new();
    for num in start..=end {
        result.push(num.to_string());
    }

    Ok(result)
}

/// Generate all combinations of expansions with prefixes and suffixes
fn generate_expansions(
    patterns: Vec<(String, Vec<String>, String)>,
) -> Result<Vec<String>, String> {
    if patterns.is_empty() {
        return Ok(Vec::new());
    }

    // If there's only one pattern, simple expansion
    if patterns.len() == 1 {
        let (prefix, alternatives, suffix) = &patterns[0];
        let mut result = Vec::new();
        for alt in alternatives {
            result.push(format!("{}{}{}", prefix, alt, suffix));
        }
        return Ok(result);
    }

    // Multiple patterns: need to check if they should be combined (cartesian product)
    // or expanded independently

    // Check if patterns are consecutive (no gap between suffix of one and prefix of next)
    let mut is_consecutive = true;
    for i in 0..patterns.len() - 1 {
        let (_, _, suffix) = &patterns[i];
        let (prefix, _, _) = &patterns[i + 1];

        // If the suffix of pattern i contains the prefix of pattern i+1, they're consecutive
        // For example: {a,b} has suffix "{1,2}" and {1,2} has prefix "{a,b}"
        if !suffix.is_empty() && !prefix.is_empty() {
            // They overlap, so they're not truly consecutive
            is_consecutive = false;
            break;
        }
    }

    if is_consecutive {
        // Generate cartesian product
        // Start with the first pattern's prefix
        let base_prefix = &patterns[0].0;

        // Build up combinations recursively
        let mut current_results = vec![base_prefix.clone()];

        for (i, (_prefix, alternatives, suffix)) in patterns.iter().enumerate() {
            let mut next_results = Vec::new();

            for current in &current_results {
                for alt in alternatives {
                    let mut new_str = current.clone();

                    // For the first pattern, we already have the prefix
                    if i > 0 {
                        // Remove the overlapping prefix (it's already in current)
                        // The prefix of pattern i is actually the suffix of pattern i-1
                        // which we already added
                    }

                    new_str.push_str(alt);

                    // Add suffix only if it's the last pattern or if it doesn't overlap
                    if i == patterns.len() - 1 {
                        new_str.push_str(suffix);
                    }

                    next_results.push(new_str);
                }
            }

            current_results = next_results;
        }

        return Ok(current_results);
    }

    // Not consecutive, expand independently
    let mut result = Vec::new();
    for (prefix, alternatives, suffix) in patterns {
        for alt in alternatives {
            result.push(format!("{}{}{}", prefix, alt, suffix));
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_top_level_simple() {
        let result = split_top_level("a,b,c", ',').unwrap();
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_split_top_level_with_braces() {
        let result = split_top_level("a,{b,c},d", ',').unwrap();
        assert_eq!(result, vec!["a", "{b,c}", "d"]);
    }

    #[test]
    fn test_split_top_level_nested_braces() {
        let result = split_top_level("{a,b},{c,d}", ',').unwrap();
        assert_eq!(result, vec!["{a,b}", "{c,d}"]);
    }

    #[test]
    fn test_expand_char_range() {
        let result = expand_char_range('a', 'c').unwrap();
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_expand_numeric_range() {
        let result = expand_numeric_range(1, 3).unwrap();
        assert_eq!(result, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_expand_braces_in_word_simple() {
        let result = expand_braces_in_word("a{b,c}d").unwrap();
        assert_eq!(result, vec!["abd", "acd"]);
    }

    #[test]
    fn test_expand_braces_in_word_with_ranges() {
        let result = expand_braces_in_word("{1..3}").unwrap();
        assert_eq!(result, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_expand_braces_in_word_mixed() {
        let result = expand_braces_in_word("file{a,b}.txt").unwrap();
        assert_eq!(result, vec!["filea.txt", "fileb.txt"]);
    }

    #[test]
    fn test_expand_braces_in_word_nested() {
        let result = expand_braces_in_word("{{a,b},{c,d}}").unwrap();
        assert_eq!(result, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_parse_brace_content_simple_nested() {
        let result = parse_brace_content("{a,b}").unwrap();
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn test_parse_brace_content_nested() {
        let result = parse_brace_content("{a,b},{c,d}").unwrap();
        assert_eq!(result, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_expand_braces_no_braces() {
        let result = expand_braces_in_word("hello").unwrap();
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn test_expand_braces_empty() {
        let tokens = vec![];
        let result = expand_braces(tokens).unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_expand_braces_word_without_braces() {
        let tokens = vec![Token::Word("hello".to_string())];
        let result = expand_braces(tokens).unwrap();
        assert_eq!(result, vec![Token::Word("hello".to_string())]);
    }

    #[test]
    fn test_expand_braces_mixed_tokens() {
        let tokens = vec![
            Token::Word("{a,b}".to_string()),
            Token::Pipe,
            Token::Word("cat".to_string()),
        ];
        let result = expand_braces(tokens).unwrap();
        assert_eq!(
            result,
            vec![
                Token::Word("a".to_string()),
                Token::Word("b".to_string()),
                Token::Pipe,
                Token::Word("cat".to_string()),
            ]
        );
    }
}
