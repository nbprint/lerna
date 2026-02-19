// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Glob pattern matching utilities

/// A glob pattern for filtering names
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Glob {
    /// Patterns to include
    pub include: Vec<String>,
    /// Patterns to exclude
    pub exclude: Vec<String>,
}

impl Glob {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_include(mut self, patterns: Vec<String>) -> Self {
        self.include = patterns;
        self
    }

    pub fn with_exclude(mut self, patterns: Vec<String>) -> Self {
        self.exclude = patterns;
        self
    }

    /// Filter a list of names based on include and exclude patterns
    pub fn filter(&self, names: &[String]) -> Vec<String> {
        names
            .iter()
            .filter(|name| {
                self.matches_any(name, &self.include) && !self.matches_any(name, &self.exclude)
            })
            .cloned()
            .collect()
    }

    /// Check if a name matches any of the given glob patterns
    fn matches_any(&self, name: &str, patterns: &[String]) -> bool {
        for pattern in patterns {
            if glob_match(pattern, name) {
                return true;
            }
        }
        false
    }
}

/// Simple glob pattern matching (supports * and ?)
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern_chars = pattern.chars().peekable();
    let text_chars = text.chars().peekable();

    glob_match_impl(
        &mut pattern_chars.collect::<Vec<_>>(),
        &text_chars.collect::<Vec<_>>(),
    )
}

fn glob_match_impl(pattern: &[char], text: &[char]) -> bool {
    let mut pi = 0;
    let mut ti = 0;
    let mut star_pi: Option<usize> = None;
    let mut star_ti: Option<usize> = None;

    while ti < text.len() {
        if pi < pattern.len() && (pattern[pi] == '?' || pattern[pi] == text[ti]) {
            // Characters match or ? matches any single character
            pi += 1;
            ti += 1;
        } else if pi < pattern.len() && pattern[pi] == '*' {
            // * matches zero or more characters
            star_pi = Some(pi);
            star_ti = Some(ti);
            pi += 1;
        } else if let Some(spi) = star_pi {
            // Mismatch, but we have a previous * - backtrack
            pi = spi + 1;
            star_ti = Some(star_ti.unwrap() + 1);
            ti = star_ti.unwrap();
        } else {
            // No match
            return false;
        }
    }

    // Check remaining pattern characters (should all be *)
    while pi < pattern.len() && pattern[pi] == '*' {
        pi += 1;
    }

    pi == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("abc", "abc"));
        assert!(!glob_match("abc", "abd"));
        assert!(!glob_match("abc", "ab"));
        assert!(!glob_match("abc", "abcd"));
    }

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*", ""));
        assert!(glob_match("a*", "abc"));
        assert!(glob_match("*c", "abc"));
        assert!(glob_match("a*c", "abc"));
        assert!(glob_match("a*c", "ac"));
        assert!(glob_match("a*c", "aXYZc"));
        assert!(!glob_match("a*c", "ab"));
    }

    #[test]
    fn test_glob_match_question() {
        assert!(glob_match("?", "a"));
        assert!(!glob_match("?", ""));
        assert!(!glob_match("?", "ab"));
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "ac"));
        assert!(!glob_match("a?c", "abbc"));
    }

    #[test]
    fn test_glob_match_combined() {
        assert!(glob_match("a*b?c", "aXXXbYc"));
        assert!(glob_match("*.txt", "file.txt"));
        assert!(!glob_match("*.txt", "file.py"));
        assert!(glob_match("test_*", "test_foo"));
        assert!(glob_match("test_*", "test_"));
    }

    #[test]
    fn test_glob_filter() {
        let glob = Glob::new()
            .with_include(vec!["*.py".to_string(), "*.txt".to_string()])
            .with_exclude(vec!["test_*".to_string()]);

        let names = vec![
            "main.py".to_string(),
            "test_main.py".to_string(),
            "readme.txt".to_string(),
            "config.yaml".to_string(),
        ];

        let filtered = glob.filter(&names);
        assert_eq!(
            filtered,
            vec!["main.py".to_string(), "readme.txt".to_string()]
        );
    }

    #[test]
    fn test_glob_filter_empty() {
        let glob = Glob::new();
        let names = vec!["a".to_string(), "b".to_string()];
        // With no include patterns, nothing matches
        let filtered = glob.filter(&names);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_glob_filter_include_all() {
        let glob = Glob::new().with_include(vec!["*".to_string()]);
        let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let filtered = glob.filter(&names);
        assert_eq!(filtered, names);
    }
}
