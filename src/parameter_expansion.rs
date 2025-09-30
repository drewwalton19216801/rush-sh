/// Parameter expansion implementation for POSIX sh compatibility
use super::state::ShellState;

/// Simple glob pattern matcher for POSIX shell parameter expansion
/// Supports * (matches any sequence of characters) and literal characters
fn glob_match(pattern: &str, text: &str) -> bool {
    glob_match_recursive(pattern, text, 0, 0)
}

fn glob_match_recursive(pattern: &str, text: &str, pi: usize, ti: usize) -> bool {
    // If we've consumed both pattern and text, it's a match
    if pi >= pattern.len() {
        return ti >= text.len();
    }

    // If we've consumed text but not pattern, only match if remaining pattern is all *
    if ti >= text.len() {
        return pattern[pi..].chars().all(|c| c == '*');
    }

    match pattern.chars().nth(pi).unwrap() {
        '*' => {
            // * matches zero or more characters
            // Try matching zero characters first, then one, then more
            if glob_match_recursive(pattern, text, pi + 1, ti) {
                return true;
            }
            // Try matching one more character
            if ti < text.len() {
                return glob_match_recursive(pattern, text, pi, ti + 1);
            }
            false
        }
        c => {
            // Literal character - must match exactly
            if c == text.chars().nth(ti).unwrap() {
                glob_match_recursive(pattern, text, pi + 1, ti + 1)
            } else {
                false
            }
        }
    }
}

/// Find the shortest prefix of text that matches the pattern
fn find_shortest_prefix_match(pattern: &str, text: &str) -> Option<usize> {
    if pattern.is_empty() {
        return Some(0);
    }

    for i in 0..=text.len() {
        let prefix = &text[..i];
        if glob_match(pattern, prefix) {
            return Some(i);
        }
    }
    None
}

/// Find the longest prefix of text that matches the pattern
fn find_longest_prefix_match(pattern: &str, text: &str) -> Option<usize> {
    if pattern.is_empty() {
        return Some(0);
    }

    let mut longest = None;
    for i in 0..=text.len() {
        let prefix = &text[..i];
        if glob_match(pattern, prefix) {
            longest = Some(i);
        }
    }
    longest
}

/// Find the shortest suffix of text that matches the pattern
fn find_shortest_suffix_match(pattern: &str, text: &str) -> Option<usize> {
    if pattern.is_empty() {
        return Some(text.len());
    }

    for i in (0..=text.len()).rev() {
        let suffix = &text[i..];
        if glob_match(pattern, suffix) {
            return Some(i);
        }
    }
    None
}

/// Find the longest suffix of text that matches the pattern
fn find_longest_suffix_match(pattern: &str, text: &str) -> Option<usize> {
    if pattern.is_empty() {
        return Some(text.len());
    }

    let mut longest = None;
    for i in (0..=text.len()).rev() {
        let suffix = &text[i..];
        if glob_match(pattern, suffix) {
            longest = Some(i);
        }
    }
    longest
}

/// Represents different types of parameter expansion modifiers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterModifier {
    /// No modifier - just ${VAR}
    None,
    /// ${VAR:-word} - use default if VAR is unset or null
    Default(String),
    /// ${VAR:=word} - assign default if VAR is unset or null
    AssignDefault(String),
    /// ${VAR:+word} - use alternative if VAR is set and not null
    Alternative(String),
    /// ${VAR:?word} - display error if VAR is unset or null
    Error(String),
    /// ${VAR:offset} - substring starting at offset
    Substring(usize),
    /// ${VAR:offset:length} - substring with length
    SubstringWithLength(usize, usize),
    /// ${VAR#pattern} - remove shortest match from beginning
    RemoveShortestPrefix(String),
    /// ${VAR##pattern} - remove longest match from beginning
    RemoveLongestPrefix(String),
    /// ${VAR%pattern} - remove shortest match from end
    RemoveShortestSuffix(String),
    /// ${VAR%%pattern} - remove longest match from end
    RemoveLongestSuffix(String),
    /// ${VAR/pattern/replacement} - substitute first match
    Substitute(String, String),
    /// ${VAR//pattern/replacement} - substitute all matches
    SubstituteAll(String, String),
    /// ${!prefix*} - names of variables starting with prefix
    IndirectPrefix,
    /// ${!prefix@} - names of variables starting with prefix (same as IndirectPrefix)
    IndirectPrefixAt,
}

/// Represents a parameter expansion expression
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterExpansion {
    pub var_name: String,
    pub modifier: ParameterModifier,
}

/// Parse a parameter expansion from ${...} syntax
pub fn parse_parameter_expansion(content: &str) -> Result<ParameterExpansion, String> {
    if content.is_empty() {
        return Err("Empty parameter expansion".to_string());
    }

    let chars = content.chars();
    let mut var_name = String::new();

    // Parse variable name
    for ch in chars {
        if ch == ':' || ch == '#' || ch == '%' || ch == '/' {
            // Found a modifier - put back the character for modifier parsing
            let modifier_str: String = content[var_name.len()..].to_string();
            let modifier = parse_modifier(&modifier_str)?;
            return Ok(ParameterExpansion { var_name, modifier });
        } else if ch == '!' {
            // Special case for indirect expansion ${!PREFIX*}
            // The '!' is part of the variable name, continue parsing
            var_name.push(ch);
        } else if ch.is_alphanumeric() || ch == '_' || ch == '*' {
            // Allow alphanumeric, underscore, and '*' (for indirect expansion)
            var_name.push(ch);
        } else {
            return Err(format!("Invalid character '{}' in variable name", ch));
        }
    }

    // No modifier found - check if this is an indirect expansion
    let (final_var_name, modifier) = if var_name.starts_with('!') {
        if var_name.ends_with('*') {
            // Strip the '*' from the var_name for IndirectPrefix
            (var_name[..var_name.len() - 1].to_string(), ParameterModifier::IndirectPrefix)
        } else if var_name.ends_with('@') {
            // Strip the '@' from the var_name for IndirectPrefixAt
            (var_name[..var_name.len() - 1].to_string(), ParameterModifier::IndirectPrefixAt)
        } else {
            return Err("Invalid indirect expansion: must end with * or @".to_string());
        }
    } else {
        (var_name, ParameterModifier::None)
    };

    Ok(ParameterExpansion {
        var_name: final_var_name,
        modifier,
    })
}

/// Parse a parameter modifier from the modifier string
fn parse_modifier(modifier_str: &str) -> Result<ParameterModifier, String> {
    if modifier_str.is_empty() {
        return Ok(ParameterModifier::None);
    }

    let mut chars = modifier_str.chars();

    match chars.next().unwrap() {
        ':' => {
            match chars.next() {
                Some('=') => {
                    // ${VAR:=word}
                    let word = modifier_str[2..].to_string();
                    Ok(ParameterModifier::AssignDefault(word))
                }
                Some('-') => {
                    // ${VAR:-word}
                    let word = modifier_str[2..].to_string();
                    Ok(ParameterModifier::Default(word))
                }
                Some('+') => {
                    // ${VAR:+word}
                    let word = modifier_str[2..].to_string();
                    Ok(ParameterModifier::Alternative(word))
                }
                Some('?') => {
                    // ${VAR:?word}
                    let word = modifier_str[2..].to_string();
                    Ok(ParameterModifier::Error(word))
                }
                Some(ch) if ch.is_ascii_digit() => {
                    // ${VAR:offset} or ${VAR:offset:length}
                    // Parse the substring syntax by analyzing the full modifier string

                    // Extract the offset part (digits after the initial ':')
                    let after_colon = &modifier_str[1..]; // Skip the initial ':'
                    let offset_end = after_colon.find(':').unwrap_or(after_colon.len());
                    let offset_str = &after_colon[..offset_end];

                    if offset_str.is_empty() {
                        return Err("Missing offset in substring operation".to_string());
                    }

                    let offset: usize = offset_str.parse().map_err(|_| "Invalid offset number")?;

                    // Check if there's a length specification
                    if offset_end < after_colon.len() {
                        // There's more content after the offset
                        let after_offset = &after_colon[offset_end + 1..]; // Skip the ':' after offset
                        if !after_offset.is_empty()
                            && after_offset.chars().all(|c| c.is_ascii_digit())
                        {
                            let length: usize =
                                after_offset.parse().map_err(|_| "Invalid length number")?;
                            Ok(ParameterModifier::SubstringWithLength(offset, length))
                        } else {
                            Ok(ParameterModifier::Substring(offset))
                        }
                    } else {
                        Ok(ParameterModifier::Substring(offset))
                    }
                }
                _ => Err(format!("Invalid modifier: {}", modifier_str)),
            }
        }
        '#' => {
            if let Some(pattern) = modifier_str.strip_prefix("##") {
                // ${VAR##pattern}
                Ok(ParameterModifier::RemoveLongestPrefix(pattern.to_string()))
            } else if let Some(pattern) = modifier_str.strip_prefix('#') {
                // ${VAR#pattern} - treat everything after # as pattern
                Ok(ParameterModifier::RemoveShortestPrefix(pattern.to_string()))
            } else {
                Err(format!("Invalid prefix removal modifier: {}", modifier_str))
            }
        }
        '%' => {
            if let Some(pattern) = modifier_str.strip_prefix("%%") {
                // ${VAR%%pattern}
                Ok(ParameterModifier::RemoveLongestSuffix(pattern.to_string()))
            } else if let Some(pattern) = modifier_str.strip_prefix('%') {
                // ${VAR%pattern}
                Ok(ParameterModifier::RemoveShortestSuffix(pattern.to_string()))
            } else {
                Err(format!("Invalid suffix removal modifier: {}", modifier_str))
            }
        }
        '/' => {
            // Pattern substitution: ${VAR/pattern/replacement} or ${VAR//pattern/replacement}
            let remaining: String = chars.as_str().to_string();

            if modifier_str.starts_with("//") {
                // Substitute all - skip the first '/' and find the pattern/replacement separator
                let after_double_slash = &remaining[1..]; // Skip the first '/'
                if let Some(slash_pos) = after_double_slash.find('/') {
                    let pattern = after_double_slash[..slash_pos].to_string();
                    let replacement = after_double_slash[slash_pos + 1..].to_string();
                    Ok(ParameterModifier::SubstituteAll(pattern, replacement))
                } else {
                    Err("Invalid substitution syntax: missing replacement".to_string())
                }
            } else {
                // Regular substitution
                if let Some(slash_pos) = remaining.find('/') {
                    let pattern = remaining[..slash_pos].to_string();
                    let replacement = remaining[slash_pos + 1..].to_string();
                    Ok(ParameterModifier::Substitute(pattern, replacement))
                } else {
                    Err("Invalid substitution syntax: missing replacement".to_string())
                }
            }
        }
        '!' => {
            let prefix = modifier_str[1..].to_string();
            if prefix.ends_with('*') {
                Ok(ParameterModifier::IndirectPrefix)
            } else if prefix.ends_with('@') {
                Ok(ParameterModifier::IndirectPrefixAt)
            } else {
                Err("Invalid indirect expansion: must end with * or @".to_string())
            }
        }
        _ => Err(format!("Unknown modifier: {}", modifier_str)),
    }
}

/// Expand a parameter expression using the given shell state
pub fn expand_parameter(
    expansion: &ParameterExpansion,
    shell_state: &ShellState,
) -> Result<String, String> {
    let value = match expansion.modifier {
        ParameterModifier::None => {
            // Simple variable expansion
            shell_state.get_var(&expansion.var_name)
        }
        ParameterModifier::Default(ref default) => {
            // ${VAR:-word} - use default if VAR is unset or null
            match shell_state.get_var(&expansion.var_name) {
                Some(val) if !val.is_empty() => Some(val),
                _ => Some(default.clone()),
            }
        }
        ParameterModifier::AssignDefault(ref default) => {
            // ${VAR:=word} - assign default if VAR is unset or null
            match shell_state.get_var(&expansion.var_name) {
                Some(val) if !val.is_empty() => Some(val),
                _ => {
                    // Assign the default value
                    Some(default.clone())
                }
            }
        }
        ParameterModifier::Alternative(ref alternative) => {
            // ${VAR:+word} - use alternative if VAR is set and not null
            match shell_state.get_var(&expansion.var_name) {
                Some(val) if !val.is_empty() => Some(alternative.clone()),
                _ => Some("".to_string()),
            }
        }
        ParameterModifier::Error(ref error_msg) => {
            // ${VAR:?word} - display error if VAR is unset or null
            match shell_state.get_var(&expansion.var_name) {
                Some(val) if !val.is_empty() => Some(val),
                _ => {
                    let msg = if error_msg.is_empty() {
                        format!("parameter '{}' not set", expansion.var_name)
                    } else {
                        error_msg.clone()
                    };
                    return Err(msg);
                }
            }
        }
        ParameterModifier::Substring(offset) => {
            // ${VAR:offset}
            if let Some(val) = shell_state.get_var(&expansion.var_name) {
                let start = offset.min(val.len());
                Some(val[start..].to_string())
            } else {
                Some("".to_string())
            }
        }
        ParameterModifier::SubstringWithLength(offset, length) => {
            // ${VAR:offset:length}
            if let Some(val) = shell_state.get_var(&expansion.var_name) {
                let start = offset.min(val.len());
                let end = (start + length).min(val.len());
                Some(val[start..end].to_string())
            } else {
                Some("".to_string())
            }
        }
        ParameterModifier::RemoveShortestPrefix(ref pattern) => {
            // ${VAR#pattern}
            if let Some(val) = shell_state.get_var(&expansion.var_name) {
                if let Some(match_end) = find_shortest_prefix_match(pattern, &val) {
                    Some(val[match_end..].to_string())
                } else {
                    Some(val.clone())
                }
            } else {
                Some("".to_string())
            }
        }
        ParameterModifier::RemoveLongestPrefix(ref pattern) => {
            // ${VAR##pattern}
            if let Some(val) = shell_state.get_var(&expansion.var_name) {
                if let Some(match_end) = find_longest_prefix_match(pattern, &val) {
                    Some(val[match_end..].to_string())
                } else {
                    Some(val.clone())
                }
            } else {
                Some("".to_string())
            }
        }
        ParameterModifier::RemoveShortestSuffix(ref pattern) => {
            // ${VAR%pattern}
            if let Some(val) = shell_state.get_var(&expansion.var_name) {
                if let Some(match_start) = find_shortest_suffix_match(pattern, &val) {
                    Some(val[..match_start].to_string())
                } else {
                    Some(val.clone())
                }
            } else {
                Some("".to_string())
            }
        }
        ParameterModifier::RemoveLongestSuffix(ref pattern) => {
            // ${VAR%%pattern}
            if let Some(val) = shell_state.get_var(&expansion.var_name) {
                if let Some(match_start) = find_longest_suffix_match(pattern, &val) {
                    Some(val[..match_start].to_string())
                } else {
                    Some(val.clone())
                }
            } else {
                Some("".to_string())
            }
        }
        ParameterModifier::Substitute(ref pattern, ref replacement) => {
            // ${VAR/pattern/replacement}
            if let Some(val) = shell_state.get_var(&expansion.var_name) {
                // Simple string-based substitution for now
                Some(val.replace(pattern, replacement))
            } else {
                Some("".to_string())
            }
        }
        ParameterModifier::SubstituteAll(ref pattern, ref replacement) => {
            // ${VAR//pattern/replacement}
            if let Some(val) = shell_state.get_var(&expansion.var_name) {
                // Simple string-based substitution for now
                Some(val.replace(pattern, replacement))
            } else {
                Some("".to_string())
            }
        }
        ParameterModifier::IndirectPrefix | ParameterModifier::IndirectPrefixAt => {
            // ${!prefix*}
            // For now, return empty string as this is complex to implement
            // TODO: Implement indirect expansion properly
            Some("".to_string())
        }
    };

    Ok(value.unwrap_or_else(|| "".to_string()))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_variable() {
        let result = parse_parameter_expansion("VAR").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(result.modifier, ParameterModifier::None);
    }

    #[test]
    fn test_parse_default_modifier() {
        let result = parse_parameter_expansion("VAR:-default").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(
            result.modifier,
            ParameterModifier::Default("default".to_string())
        );
    }

    #[test]
    fn test_parse_assign_default_modifier() {
        let result = parse_parameter_expansion("VAR:=default").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(
            result.modifier,
            ParameterModifier::AssignDefault("default".to_string())
        );
    }

    #[test]
    fn test_parse_alternative_modifier() {
        let result = parse_parameter_expansion("VAR:+alt").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(
            result.modifier,
            ParameterModifier::Alternative("alt".to_string())
        );
    }

    #[test]
    fn test_parse_error_modifier() {
        let result = parse_parameter_expansion("VAR:?error").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(
            result.modifier,
            ParameterModifier::Error("error".to_string())
        );
    }

    #[test]
    fn test_parse_substring() {
        let result = parse_parameter_expansion("VAR:5").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(result.modifier, ParameterModifier::Substring(5));
    }

    #[test]
    fn test_parse_substring_with_length() {
        let result = parse_parameter_expansion("VAR:2:3").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(
            result.modifier,
            ParameterModifier::SubstringWithLength(2, 3)
        );
    }


    #[test]
    fn test_parse_remove_shortest_prefix() {
        let result = parse_parameter_expansion("VAR#prefix").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(
            result.modifier,
            ParameterModifier::RemoveShortestPrefix("prefix".to_string())
        );
    }

    #[test]
    fn test_parse_remove_longest_prefix() {
        let result = parse_parameter_expansion("VAR##prefix").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(
            result.modifier,
            ParameterModifier::RemoveLongestPrefix("prefix".to_string())
        );
    }

    #[test]
    fn test_parse_remove_shortest_suffix() {
        let result = parse_parameter_expansion("VAR%suffix").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(
            result.modifier,
            ParameterModifier::RemoveShortestSuffix("suffix".to_string())
        );
    }

    #[test]
    fn test_parse_remove_longest_suffix() {
        let result = parse_parameter_expansion("VAR%%suffix").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(
            result.modifier,
            ParameterModifier::RemoveLongestSuffix("suffix".to_string())
        );
    }

    #[test]
    fn test_parse_substitute() {
        let result = parse_parameter_expansion("VAR/old/new").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(
            result.modifier,
            ParameterModifier::Substitute("old".to_string(), "new".to_string())
        );
    }

    #[test]
    fn test_parse_substitute_all() {
        let result = parse_parameter_expansion("VAR//old/new").unwrap();
        assert_eq!(result.var_name, "VAR");
        assert_eq!(
            result.modifier,
            ParameterModifier::SubstituteAll("old".to_string(), "new".to_string())
        );
    }

    #[test]
    fn test_parse_indirect_prefix() {
        let result = parse_parameter_expansion("!PREFIX*").unwrap();
        assert_eq!(result.var_name, "!PREFIX");
        assert_eq!(result.modifier, ParameterModifier::IndirectPrefix);
    }

    #[test]
    fn test_parse_empty() {
        let result = parse_parameter_expansion("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_character() {
        let result = parse_parameter_expansion("VAR@test");
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_simple_variable() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("TEST_VAR", "hello world".to_string());

        let expansion = ParameterExpansion {
            var_name: "TEST_VAR".to_string(),
            modifier: ParameterModifier::None,
        };

        let result = expand_parameter(&expansion, &shell_state).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_expand_default_modifier() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("TEST_VAR", "value".to_string());

        let expansion = ParameterExpansion {
            var_name: "TEST_VAR".to_string(),
            modifier: ParameterModifier::Default("default".to_string()),
        };

        let result = expand_parameter(&expansion, &shell_state).unwrap();
        assert_eq!(result, "value");
    }

    #[test]
    fn test_expand_default_modifier_unset() {
        let shell_state = ShellState::new();

        let expansion = ParameterExpansion {
            var_name: "UNSET_VAR".to_string(),
            modifier: ParameterModifier::Default("default".to_string()),
        };

        let result = expand_parameter(&expansion, &shell_state).unwrap();
        assert_eq!(result, "default");
    }

    #[test]
    fn test_expand_substring() {
        let mut shell_state = ShellState::new();
        shell_state.set_var("TEST_VAR", "hello world".to_string());

        let expansion = ParameterExpansion {
            var_name: "TEST_VAR".to_string(),
            modifier: ParameterModifier::Substring(6),
        };

        let result = expand_parameter(&expansion, &shell_state).unwrap();
        assert_eq!(result, "world");
    }

}
