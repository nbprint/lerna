// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Config merging module
//!
//! Implements deep merging of configuration dictionaries, following OmegaConf merge semantics.

use crate::config::{ConfigDict, ConfigValue};
use std::collections::HashSet;

/// Merge mode for config values
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MergeMode {
    /// Default merge - override values, merge dicts
    Default,
    /// Force override - replace entirely, don't merge
    Override,
    /// Extend - for lists, extend instead of replace
    Extend,
}

/// Merge two ConfigValue instances
///
/// Follows OmegaConf merge semantics:
/// - Dicts are merged recursively
/// - Other types override
/// - Special handling for ??? (MISSING) and None
pub fn merge_values(base: ConfigValue, override_val: ConfigValue, mode: MergeMode) -> ConfigValue {
    match (base, override_val.clone(), mode) {
        // MISSING in override means keep base
        (base, ConfigValue::Missing, _) => base,

        // Null in base, any override wins
        (ConfigValue::Null, override_val, _) => override_val,

        // Dict + Dict = merge recursively
        (
            ConfigValue::Dict(mut base_dict),
            ConfigValue::Dict(override_dict),
            MergeMode::Default,
        ) => {
            merge_dicts(&mut base_dict, &override_dict);
            ConfigValue::Dict(base_dict)
        }

        // Dict + Dict with Override mode = replace entirely
        (ConfigValue::Dict(_), ConfigValue::Dict(override_dict), MergeMode::Override) => {
            ConfigValue::Dict(override_dict)
        }

        // List + List with Extend mode = concatenate
        (ConfigValue::List(mut base_list), ConfigValue::List(override_list), MergeMode::Extend) => {
            base_list.extend(override_list);
            ConfigValue::List(base_list)
        }

        // Any other case: override wins
        (_, override_val, _) => override_val,
    }
}

/// Deep merge two ConfigDicts
///
/// The base dict is modified in place with values from override_dict.
pub fn merge_dicts(base: &mut ConfigDict, override_dict: &ConfigDict) {
    for (key, value) in override_dict.iter() {
        if let Some(base_val) = base.get(key) {
            let merged = merge_values(base_val.clone(), value.clone(), MergeMode::Default);
            base.insert(key.to_string(), merged);
        } else {
            base.insert(key.to_string(), value.clone());
        }
    }
}

/// Merge multiple config dicts in order
///
/// Later configs override earlier ones.
pub fn merge_configs(configs: &[ConfigDict]) -> ConfigDict {
    let mut result = ConfigDict::new();
    for config in configs {
        merge_dicts(&mut result, config);
    }
    result
}

/// Check if a key should be deleted (starts with ~)
pub fn is_deletion_key(key: &str) -> bool {
    key.starts_with('~')
}

/// Get the actual key from a deletion key
pub fn get_deletion_target(key: &str) -> &str {
    if key.starts_with('~') {
        &key[1..]
    } else {
        key
    }
}

/// Apply deletions to a config
pub fn apply_deletions(config: &mut ConfigDict, deletions: &[String]) {
    for deletion in deletions {
        let key = get_deletion_target(deletion);
        // Handle nested deletions with dot notation
        if key.contains('.') {
            let parts: Vec<&str> = key.split('.').collect();
            delete_nested(config, &parts);
        } else {
            config.remove(key);
        }
    }
}

fn delete_nested(config: &mut ConfigDict, parts: &[&str]) {
    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        config.remove(parts[0]);
        return;
    }

    // Navigate to parent and delete
    if let Some(ConfigValue::Dict(ref mut nested)) = config.get_mut(parts[0]) {
        delete_nested(nested, &parts[1..]);
    }
}

/// Apply an override to a config at a specific path
pub fn apply_override(config: &mut ConfigDict, path: &str, value: ConfigValue) {
    if path.is_empty() {
        // Root-level merge
        if let ConfigValue::Dict(dict) = value {
            merge_dicts(config, &dict);
        }
        return;
    }

    let parts: Vec<&str> = path.split('.').collect();
    set_nested(config, &parts, value);
}

fn set_nested(config: &mut ConfigDict, parts: &[&str], value: ConfigValue) {
    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        config.insert(parts[0].to_string(), value);
        return;
    }

    // Navigate or create intermediate dicts
    let key = parts[0].to_string();
    if !config.contains_key(&key) {
        config.insert(key.clone(), ConfigValue::Dict(ConfigDict::new()));
    }

    if let Some(ConfigValue::Dict(ref mut nested)) = config.get_mut(&key) {
        set_nested(nested, &parts[1..], value);
    }
}

/// Get a value from a nested path
pub fn get_nested(config: &ConfigDict, path: &str) -> Option<ConfigValue> {
    if path.is_empty() {
        return Some(ConfigValue::Dict(config.clone()));
    }

    let parts: Vec<&str> = path.split('.').collect();
    get_nested_parts(config, &parts)
}

fn get_nested_parts(config: &ConfigDict, parts: &[&str]) -> Option<ConfigValue> {
    if parts.is_empty() {
        return None;
    }

    let value = config.get(parts[0])?;

    if parts.len() == 1 {
        return Some(value.clone());
    }

    match value {
        ConfigValue::Dict(nested) => get_nested_parts(nested, &parts[1..]),
        _ => None,
    }
}

/// Collect all keys from a config (flattened with dot notation)
pub fn collect_keys(config: &ConfigDict, prefix: &str) -> Vec<String> {
    let mut keys = Vec::new();

    for (key, value) in config.iter() {
        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", prefix, key)
        };

        keys.push(full_key.clone());

        if let ConfigValue::Dict(nested) = value {
            keys.extend(collect_keys(nested, &full_key));
        }
    }

    keys
}

/// Find keys that differ between two configs
pub fn diff_keys(config1: &ConfigDict, config2: &ConfigDict) -> Vec<String> {
    let keys1: HashSet<String> = collect_keys(config1, "").into_iter().collect();
    let keys2: HashSet<String> = collect_keys(config2, "").into_iter().collect();

    let mut diff: Vec<String> = keys1.symmetric_difference(&keys2).cloned().collect();

    // Also check for value differences on common keys
    for key in keys1.intersection(&keys2) {
        let val1 = get_nested(config1, key);
        let val2 = get_nested(config2, key);
        if val1 != val2 {
            diff.push(key.clone());
        }
    }

    diff.sort();
    diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_simple() {
        let mut base = ConfigDict::new();
        base.insert("a".to_string(), ConfigValue::Int(1));
        base.insert("b".to_string(), ConfigValue::Int(2));

        let mut override_dict = ConfigDict::new();
        override_dict.insert("b".to_string(), ConfigValue::Int(20));
        override_dict.insert("c".to_string(), ConfigValue::Int(3));

        merge_dicts(&mut base, &override_dict);

        assert_eq!(base.get("a"), Some(&ConfigValue::Int(1)));
        assert_eq!(base.get("b"), Some(&ConfigValue::Int(20)));
        assert_eq!(base.get("c"), Some(&ConfigValue::Int(3)));
    }

    #[test]
    fn test_merge_nested() {
        let mut inner = ConfigDict::new();
        inner.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );
        inner.insert("port".to_string(), ConfigValue::Int(3306));

        let mut base = ConfigDict::new();
        base.insert("db".to_string(), ConfigValue::Dict(inner));

        let mut override_inner = ConfigDict::new();
        override_inner.insert("port".to_string(), ConfigValue::Int(5432));

        let mut override_dict = ConfigDict::new();
        override_dict.insert("db".to_string(), ConfigValue::Dict(override_inner));

        merge_dicts(&mut base, &override_dict);

        if let Some(ConfigValue::Dict(db)) = base.get("db") {
            assert_eq!(
                db.get("host"),
                Some(&ConfigValue::String("localhost".to_string()))
            );
            assert_eq!(db.get("port"), Some(&ConfigValue::Int(5432)));
        } else {
            panic!("Expected db to be a dict");
        }
    }

    #[test]
    fn test_merge_configs() {
        let mut cfg1 = ConfigDict::new();
        cfg1.insert("a".to_string(), ConfigValue::Int(1));

        let mut cfg2 = ConfigDict::new();
        cfg2.insert("b".to_string(), ConfigValue::Int(2));

        let mut cfg3 = ConfigDict::new();
        cfg3.insert("a".to_string(), ConfigValue::Int(10));

        let result = merge_configs(&[cfg1, cfg2, cfg3]);

        assert_eq!(result.get("a"), Some(&ConfigValue::Int(10)));
        assert_eq!(result.get("b"), Some(&ConfigValue::Int(2)));
    }

    #[test]
    fn test_apply_deletions() {
        let mut config = ConfigDict::new();
        config.insert("a".to_string(), ConfigValue::Int(1));
        config.insert("b".to_string(), ConfigValue::Int(2));
        config.insert("c".to_string(), ConfigValue::Int(3));

        apply_deletions(&mut config, &["~a".to_string(), "~c".to_string()]);

        assert_eq!(config.get("a"), None);
        assert_eq!(config.get("b"), Some(&ConfigValue::Int(2)));
        assert_eq!(config.get("c"), None);
    }

    #[test]
    fn test_apply_override() {
        let mut config = ConfigDict::new();
        config.insert("a".to_string(), ConfigValue::Int(1));

        apply_override(&mut config, "b.c.d", ConfigValue::Int(42));

        if let Some(val) = get_nested(&config, "b.c.d") {
            assert_eq!(val, ConfigValue::Int(42));
        } else {
            panic!("Expected nested value");
        }
    }

    #[test]
    fn test_get_nested() {
        let mut inner = ConfigDict::new();
        inner.insert("value".to_string(), ConfigValue::Int(42));

        let mut middle = ConfigDict::new();
        middle.insert("inner".to_string(), ConfigValue::Dict(inner));

        let mut config = ConfigDict::new();
        config.insert("outer".to_string(), ConfigValue::Dict(middle));

        let result = get_nested(&config, "outer.inner.value");
        assert_eq!(result, Some(ConfigValue::Int(42)));
    }

    #[test]
    fn test_collect_keys() {
        let mut inner = ConfigDict::new();
        inner.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );

        let mut config = ConfigDict::new();
        config.insert("db".to_string(), ConfigValue::Dict(inner));
        config.insert("port".to_string(), ConfigValue::Int(3306));

        let keys = collect_keys(&config, "");
        assert!(keys.contains(&"db".to_string()));
        assert!(keys.contains(&"db.host".to_string()));
        assert!(keys.contains(&"port".to_string()));
    }

    #[test]
    fn test_missing_preserves_base() {
        let base = ConfigValue::Int(42);
        let result = merge_values(base, ConfigValue::Missing, MergeMode::Default);
        assert_eq!(result, ConfigValue::Int(42));
    }

    #[test]
    fn test_list_extend() {
        let base = ConfigValue::List(vec![ConfigValue::Int(1), ConfigValue::Int(2)]);
        let override_val = ConfigValue::List(vec![ConfigValue::Int(3)]);

        let result = merge_values(base, override_val, MergeMode::Extend);

        if let ConfigValue::List(list) = result {
            assert_eq!(list.len(), 3);
        } else {
            panic!("Expected list");
        }
    }
}
