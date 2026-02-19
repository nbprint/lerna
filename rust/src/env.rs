// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Environment variable handling for configuration resolution.
//!
//! This module provides utilities for resolving environment variables
//! in configuration values, similar to OmegaConf's oc.env resolver.

use std::collections::HashMap;
use std::env;

/// Result of environment variable resolution
#[derive(Clone, Debug, PartialEq)]
pub enum EnvResult {
    /// Successfully resolved to a value
    Value(String),
    /// Variable not found
    NotFound(String),
    /// Resolution error
    Error(String),
}

impl EnvResult {
    /// Get the value if successful, or None
    pub fn ok(&self) -> Option<&str> {
        match self {
            EnvResult::Value(v) => Some(v),
            _ => None,
        }
    }

    /// Check if resolution was successful
    pub fn is_ok(&self) -> bool {
        matches!(self, EnvResult::Value(_))
    }

    /// Get the value or a default
    pub fn unwrap_or(&self, default: &str) -> String {
        match self {
            EnvResult::Value(v) => v.clone(),
            _ => default.to_string(),
        }
    }
}

/// Environment variable resolver with caching and default value support
#[derive(Clone, Debug, Default)]
pub struct EnvResolver {
    /// Cache of resolved environment variables
    cache: HashMap<String, EnvResult>,
    /// Whether to use caching
    use_cache: bool,
    /// Custom environment overrides (for testing)
    overrides: HashMap<String, String>,
}

impl EnvResolver {
    /// Create a new resolver
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            use_cache: true,
            overrides: HashMap::new(),
        }
    }

    /// Create a resolver without caching
    pub fn without_cache() -> Self {
        Self {
            cache: HashMap::new(),
            use_cache: false,
            overrides: HashMap::new(),
        }
    }

    /// Create a resolver with custom overrides (for testing)
    pub fn with_overrides(overrides: HashMap<String, String>) -> Self {
        Self {
            cache: HashMap::new(),
            use_cache: true,
            overrides,
        }
    }

    /// Get an environment variable value
    pub fn get(&mut self, key: &str) -> EnvResult {
        // Check cache first
        if self.use_cache {
            if let Some(cached) = self.cache.get(key) {
                return cached.clone();
            }
        }

        // Check overrides
        let result = if let Some(val) = self.overrides.get(key) {
            EnvResult::Value(val.clone())
        } else {
            // Get from actual environment
            match env::var(key) {
                Ok(val) => EnvResult::Value(val),
                Err(env::VarError::NotPresent) => EnvResult::NotFound(key.to_string()),
                Err(env::VarError::NotUnicode(_)) => EnvResult::Error(format!(
                    "Environment variable '{}' contains invalid Unicode",
                    key
                )),
            }
        };

        // Cache the result
        if self.use_cache {
            self.cache.insert(key.to_string(), result.clone());
        }

        result
    }

    /// Get an environment variable with a default value
    pub fn get_or_default(&mut self, key: &str, default: &str) -> String {
        self.get(key).unwrap_or(default)
    }

    /// Get an environment variable, raising an error if not found
    pub fn get_required(&mut self, key: &str) -> Result<String, String> {
        match self.get(key) {
            EnvResult::Value(v) => Ok(v),
            EnvResult::NotFound(k) => Err(format!("Environment variable '{}' not found", k)),
            EnvResult::Error(e) => Err(e),
        }
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Enable or disable caching
    pub fn enable_caching(&mut self, enabled: bool) {
        self.use_cache = enabled;
        if !enabled {
            self.cache.clear();
        }
    }

    /// Add an override
    pub fn set_override(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key_str = key.into();
        self.cache.remove(&key_str);
        self.overrides.insert(key_str, value.into());
    }
}

/// Parse an environment variable reference like ${oc.env:VAR} or ${oc.env:VAR,default}
pub fn parse_env_ref(s: &str) -> Option<(String, Option<String>)> {
    // Handle patterns like:
    // ${oc.env:VAR}
    // ${oc.env:VAR,default}
    // ${env:VAR}
    // ${env:VAR,default}

    let s = s.trim();
    if !s.starts_with("${") || !s.ends_with("}") {
        return None;
    }

    let inner = &s[2..s.len() - 1];

    // Check for oc.env: or env: prefix
    let var_part = if inner.starts_with("oc.env:") {
        &inner[7..]
    } else if inner.starts_with("env:") {
        &inner[4..]
    } else {
        return None;
    };

    // Split on comma for default value
    if let Some(comma_idx) = var_part.find(',') {
        let key = var_part[..comma_idx].trim().to_string();
        let default = var_part[comma_idx + 1..].trim().to_string();
        Some((key, Some(default)))
    } else {
        Some((var_part.trim().to_string(), None))
    }
}

/// Resolve an environment variable reference
pub fn resolve_env_ref(s: &str, resolver: &mut EnvResolver) -> Result<String, String> {
    if let Some((key, default)) = parse_env_ref(s) {
        match resolver.get(&key) {
            EnvResult::Value(v) => Ok(v),
            EnvResult::NotFound(_) => {
                if let Some(d) = default {
                    Ok(d)
                } else {
                    Err(format!("Environment variable '{}' not found", key))
                }
            }
            EnvResult::Error(e) => Err(e),
        }
    } else {
        Err(format!("Invalid environment reference: {}", s))
    }
}

/// Find all environment variable references in a string
pub fn find_env_refs(s: &str) -> Vec<(usize, usize, String, Option<String>)> {
    let mut results = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '$' && chars[i + 1] == '{' {
            // Find the closing brace
            let start = i;
            let mut depth = 0;
            let mut j = i;
            while j < chars.len() {
                if chars[j] == '{' {
                    depth += 1;
                } else if chars[j] == '}' {
                    depth -= 1;
                    if depth == 0 {
                        let ref_str: String = chars[start..=j].iter().collect();
                        if let Some((key, default)) = parse_env_ref(&ref_str) {
                            results.push((start, j + 1, key, default));
                        }
                        i = j;
                        break;
                    }
                }
                j += 1;
            }
        }
        i += 1;
    }

    results
}

/// Resolve all environment variable references in a string
pub fn resolve_env_string(s: &str, resolver: &mut EnvResolver) -> Result<String, String> {
    let refs = find_env_refs(s);
    if refs.is_empty() {
        return Ok(s.to_string());
    }

    let mut result = s.to_string();
    // Process in reverse order to maintain correct positions
    for (start, end, key, default) in refs.into_iter().rev() {
        let replacement = match resolver.get(&key) {
            EnvResult::Value(v) => v,
            EnvResult::NotFound(_) => {
                if let Some(d) = default {
                    d
                } else {
                    return Err(format!("Environment variable '{}' not found", key));
                }
            }
            EnvResult::Error(e) => return Err(e),
        };
        result = format!("{}{}{}", &result[..start], replacement, &result[end..]);
    }

    Ok(result)
}

/// Get all environment variables as a HashMap
pub fn get_all_env() -> HashMap<String, String> {
    env::vars().collect()
}

/// Check if an environment variable is set
pub fn is_env_set(key: &str) -> bool {
    env::var(key).is_ok()
}

/// Get multiple environment variables at once
pub fn get_many_env(keys: &[&str]) -> HashMap<String, Option<String>> {
    keys.iter()
        .map(|k| (k.to_string(), env::var(k).ok()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_resolver_get() {
        let mut resolver = EnvResolver::with_overrides(
            [("TEST_VAR".to_string(), "test_value".to_string())].into(),
        );

        let result = resolver.get("TEST_VAR");
        assert_eq!(result, EnvResult::Value("test_value".to_string()));
    }

    #[test]
    fn test_env_resolver_not_found() {
        let mut resolver = EnvResolver::new();
        let result = resolver.get("DEFINITELY_NOT_SET_12345");

        assert!(matches!(result, EnvResult::NotFound(_)));
    }

    #[test]
    fn test_env_resolver_get_or_default() {
        let mut resolver = EnvResolver::new();
        let result = resolver.get_or_default("DEFINITELY_NOT_SET_12345", "default_val");

        assert_eq!(result, "default_val");
    }

    #[test]
    fn test_env_resolver_caching() {
        let mut resolver = EnvResolver::with_overrides(
            [("CACHED_VAR".to_string(), "cached_value".to_string())].into(),
        );

        // First call
        let _ = resolver.get("CACHED_VAR");

        // Change the override
        resolver
            .overrides
            .insert("CACHED_VAR".to_string(), "new_value".to_string());

        // Should still return cached value
        let result = resolver.get("CACHED_VAR");
        assert_eq!(result, EnvResult::Value("cached_value".to_string()));
    }

    #[test]
    fn test_parse_env_ref_simple() {
        let result = parse_env_ref("${oc.env:MY_VAR}");
        assert_eq!(result, Some(("MY_VAR".to_string(), None)));
    }

    #[test]
    fn test_parse_env_ref_with_default() {
        let result = parse_env_ref("${oc.env:MY_VAR,default_value}");
        assert_eq!(
            result,
            Some(("MY_VAR".to_string(), Some("default_value".to_string())))
        );
    }

    #[test]
    fn test_parse_env_ref_short_form() {
        let result = parse_env_ref("${env:MY_VAR}");
        assert_eq!(result, Some(("MY_VAR".to_string(), None)));
    }

    #[test]
    fn test_parse_env_ref_invalid() {
        assert_eq!(parse_env_ref("not an env ref"), None);
        assert_eq!(parse_env_ref("${other:VAR}"), None);
    }

    #[test]
    fn test_find_env_refs() {
        let s = "Hello ${oc.env:NAME} your age is ${env:AGE,30}";
        let refs = find_env_refs(s);

        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].2, "NAME");
        assert_eq!(refs[0].3, None);
        assert_eq!(refs[1].2, "AGE");
        assert_eq!(refs[1].3, Some("30".to_string()));
    }

    #[test]
    fn test_resolve_env_string() {
        let mut resolver =
            EnvResolver::with_overrides([("NAME".to_string(), "World".to_string())].into());

        let result = resolve_env_string("Hello ${oc.env:NAME}!", &mut resolver);
        assert_eq!(result, Ok("Hello World!".to_string()));
    }

    #[test]
    fn test_resolve_env_string_with_default() {
        let mut resolver = EnvResolver::new();

        let result = resolve_env_string("Age: ${env:AGE,25}", &mut resolver);
        assert_eq!(result, Ok("Age: 25".to_string()));
    }

    #[test]
    fn test_resolve_env_string_missing_no_default() {
        let mut resolver = EnvResolver::new();

        let result = resolve_env_string("${oc.env:MISSING_VAR_12345}", &mut resolver);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_env_set() {
        // PATH should always be set
        assert!(is_env_set("PATH"));
        assert!(!is_env_set("DEFINITELY_NOT_SET_XYZ123"));
    }
}
