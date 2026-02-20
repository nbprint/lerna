// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Utility functions for grammar and string operations

/// Characters that must be escaped in configuration values
const ESC_CHARS: &str = "\\()[]{}:=, \t";

/// Escape special characters in a string for use in configuration values
///
/// This is the Rust equivalent of the Python `escape_special_characters` function.
pub fn escape_special_characters(s: &str) -> String {
    // Quick check if there's anything to escape
    if !s.chars().any(|c| ESC_CHARS.contains(c)) {
        return s.to_string();
    }

    let mut result = String::with_capacity(s.len() * 2);

    // First pass: escape backslashes
    let with_escaped_backslashes: String = s
        .chars()
        .map(|c| {
            if c == '\\' {
                "\\\\".to_string()
            } else {
                c.to_string()
            }
        })
        .collect();

    // Second pass: escape other special characters
    for c in with_escaped_backslashes.chars() {
        if ESC_CHARS.contains(c) && c != '\\' {
            result.push('\\');
        }
        result.push(c);
    }

    result
}

/// Check if a character is a special character that needs escaping
pub fn is_special_char(c: char) -> bool {
    ESC_CHARS.contains(c)
}

/// Unescape a string by removing escape sequences
pub fn unescape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if ESC_CHARS.contains(next) {
                    result.push(chars.next().unwrap());
                    continue;
                }
            }
        }
        result.push(c);
    }

    result
}

/// Validate that a key name is valid (no special characters except dots and underscores)
pub fn is_valid_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }

    // First character must be a letter or underscore
    let first = key.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }

    // Rest must be alphanumeric, underscore, or dot
    key.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

/// Split a dotted key into parts
pub fn split_key(key: &str) -> Vec<&str> {
    key.split('.').collect()
}

/// Join key parts into a dotted key
pub fn join_key(parts: &[&str]) -> String {
    parts.join(".")
}

/// Normalize a config file name by adding .yaml extension if needed
pub fn normalize_file_name(filename: &str) -> String {
    if filename.ends_with(".yaml") || filename.ends_with(".yml") {
        filename.to_string()
    } else {
        format!("{}.yaml", filename)
    }
}

/// Get a valid filename by stripping invalid characters
/// This is similar to Django's get_valid_filename
pub fn get_valid_filename(s: &str) -> String {
    let s = s.trim().replace(' ', "_");
    // Remove any characters that are not alphanumeric, underscores, dots, or hyphens
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.' || *c == '-')
        .collect()
}

/// Sanitize a string for use in a file path
pub fn sanitize_path_component(s: &str) -> String {
    s.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_special_characters_no_escape() {
        assert_eq!(escape_special_characters("hello"), "hello");
        assert_eq!(escape_special_characters("hello_world"), "hello_world");
    }

    #[test]
    fn test_escape_special_characters_spaces() {
        assert_eq!(escape_special_characters("hello world"), "hello\\ world");
    }

    #[test]
    fn test_escape_special_characters_brackets() {
        assert_eq!(escape_special_characters("a[0]"), "a\\[0\\]");
    }

    #[test]
    fn test_escape_special_characters_parens() {
        assert_eq!(escape_special_characters("f(x)"), "f\\(x\\)");
    }

    #[test]
    fn test_escape_special_characters_braces() {
        assert_eq!(escape_special_characters("{a:1}"), "\\{a\\:1\\}");
    }

    #[test]
    fn test_escape_special_characters_equals() {
        assert_eq!(escape_special_characters("a=b"), "a\\=b");
    }

    #[test]
    fn test_escape_special_characters_backslash() {
        assert_eq!(escape_special_characters("a\\b"), "a\\\\b");
    }

    #[test]
    fn test_unescape_string() {
        assert_eq!(unescape_string("hello\\ world"), "hello world");
        assert_eq!(unescape_string("a\\[0\\]"), "a[0]");
        assert_eq!(unescape_string("a\\\\b"), "a\\b");
    }

    #[test]
    fn test_is_valid_key() {
        assert!(is_valid_key("db"));
        assert!(is_valid_key("db.host"));
        assert!(is_valid_key("db.host.port"));
        assert!(is_valid_key("_private"));
        assert!(is_valid_key("db_host"));
        assert!(!is_valid_key(""));
        assert!(!is_valid_key("123abc"));
        assert!(!is_valid_key("db[0]"));
    }

    #[test]
    fn test_split_key() {
        assert_eq!(split_key("db.host.port"), vec!["db", "host", "port"]);
        assert_eq!(split_key("db"), vec!["db"]);
    }

    #[test]
    fn test_join_key() {
        assert_eq!(join_key(&["db", "host", "port"]), "db.host.port");
        assert_eq!(join_key(&["db"]), "db");
    }

    #[test]
    fn test_normalize_file_name() {
        assert_eq!(normalize_file_name("config"), "config.yaml");
        assert_eq!(normalize_file_name("config.yaml"), "config.yaml");
        assert_eq!(normalize_file_name("config.yml"), "config.yml");
        assert_eq!(normalize_file_name("db/mysql"), "db/mysql.yaml");
    }

    #[test]
    fn test_get_valid_filename() {
        assert_eq!(get_valid_filename("my_app"), "my_app");
        assert_eq!(get_valid_filename("my app"), "my_app");
        assert_eq!(get_valid_filename("  my app  "), "my_app");
        assert_eq!(get_valid_filename("app@123"), "app123");
        assert_eq!(get_valid_filename("file.py"), "file.py");
        assert_eq!(get_valid_filename("test-file"), "test-file");
    }

    #[test]
    fn test_sanitize_path_component() {
        assert_eq!(sanitize_path_component("file"), "file");
        assert_eq!(sanitize_path_component("path/to/file"), "path_to_file");
        assert_eq!(sanitize_path_component("file:name"), "file_name");
        assert_eq!(sanitize_path_component("file<>name"), "file__name");
    }
}
