// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Configuration path utilities

/// Normalize a configuration path by removing redundant separators and dots
pub fn normalize_config_path(path: &str) -> String {
    let parts: Vec<&str> = path
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    parts.join("/")
}

/// Get the parent directory of a configuration path
pub fn get_parent_path(path: &str) -> Option<String> {
    let normalized = normalize_config_path(path);
    if let Some(idx) = normalized.rfind('/') {
        Some(normalized[..idx].to_string())
    } else {
        None
    }
}

/// Get the basename (last component) of a configuration path
pub fn get_basename(path: &str) -> String {
    let normalized = normalize_config_path(path);
    if let Some(idx) = normalized.rfind('/') {
        normalized[idx + 1..].to_string()
    } else {
        normalized
    }
}

/// Join two configuration paths
pub fn join_config_paths(base: &str, child: &str) -> String {
    let base_norm = normalize_config_path(base);
    let child_norm = normalize_config_path(child);

    if base_norm.is_empty() {
        child_norm
    } else if child_norm.is_empty() {
        base_norm
    } else {
        format!("{}/{}", base_norm, child_norm)
    }
}

/// Check if a path is absolute (starts with a known scheme or /)
pub fn is_absolute_config_path(path: &str) -> bool {
    path.starts_with('/') || path.starts_with("pkg://") || path.starts_with("file://")
}

/// Strip a scheme from a path if present
pub fn strip_scheme(path: &str) -> &str {
    if let Some(idx) = path.find("://") {
        return &path[idx + 3..];
    }
    path
}

/// Get the scheme from a path if present
pub fn get_scheme(path: &str) -> Option<&str> {
    if let Some(idx) = path.find("://") {
        return Some(&path[..idx]);
    }
    None
}

/// Validate that a config group name is valid
pub fn is_valid_group_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Group names can have slashes for nested groups
    let parts: Vec<&str> = name.split('/').collect();
    for part in parts {
        if part.is_empty() {
            return false; // No empty segments
        }
        // Each segment must be a valid identifier
        let first = part.chars().next().unwrap();
        if !first.is_ascii_alphabetic() && first != '_' {
            return false;
        }
        if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return false;
        }
    }
    true
}

/// Validate that a config name is valid (with optional .yaml extension)
pub fn is_valid_config_name(name: &str) -> bool {
    let name = name.strip_suffix(".yaml").unwrap_or(name);
    if name.is_empty() {
        return false;
    }

    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_config_path() {
        assert_eq!(normalize_config_path("a/b/c"), "a/b/c");
        assert_eq!(normalize_config_path("a//b//c"), "a/b/c");
        assert_eq!(normalize_config_path("./a/b"), "a/b");
        assert_eq!(normalize_config_path("a/./b"), "a/b");
    }

    #[test]
    fn test_get_parent_path() {
        assert_eq!(get_parent_path("a/b/c"), Some("a/b".to_string()));
        assert_eq!(get_parent_path("a/b"), Some("a".to_string()));
        assert_eq!(get_parent_path("a"), None);
    }

    #[test]
    fn test_get_basename() {
        assert_eq!(get_basename("a/b/c"), "c");
        assert_eq!(get_basename("a/b"), "b");
        assert_eq!(get_basename("a"), "a");
    }

    #[test]
    fn test_join_config_paths() {
        assert_eq!(join_config_paths("a", "b"), "a/b");
        assert_eq!(join_config_paths("a/b", "c"), "a/b/c");
        assert_eq!(join_config_paths("", "b"), "b");
        assert_eq!(join_config_paths("a", ""), "a");
    }

    #[test]
    fn test_is_absolute_config_path() {
        assert!(is_absolute_config_path("/absolute/path"));
        assert!(is_absolute_config_path("pkg://module/config"));
        assert!(is_absolute_config_path("file:///etc/config"));
        assert!(!is_absolute_config_path("relative/path"));
    }

    #[test]
    fn test_strip_scheme() {
        assert_eq!(strip_scheme("pkg://module/config"), "module/config");
        assert_eq!(strip_scheme("file:///etc/config"), "/etc/config");
        assert_eq!(strip_scheme("relative/path"), "relative/path");
    }

    #[test]
    fn test_get_scheme() {
        assert_eq!(get_scheme("pkg://module/config"), Some("pkg"));
        assert_eq!(get_scheme("file:///etc/config"), Some("file"));
        assert_eq!(get_scheme("relative/path"), None);
    }

    #[test]
    fn test_is_valid_group_name() {
        assert!(is_valid_group_name("db"));
        assert!(is_valid_group_name("db/mysql"));
        assert!(is_valid_group_name("hydra/launcher"));
        assert!(!is_valid_group_name(""));
        assert!(!is_valid_group_name("123db"));
        assert!(!is_valid_group_name("db//mysql"));
    }

    #[test]
    fn test_is_valid_config_name() {
        assert!(is_valid_config_name("config"));
        assert!(is_valid_config_name("config.yaml"));
        assert!(is_valid_config_name("my_config"));
        assert!(is_valid_config_name("my-config"));
        assert!(!is_valid_config_name(""));
        assert!(!is_valid_config_name("123config"));
    }
}
