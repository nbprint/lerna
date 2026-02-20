// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Configuration value types for Hydra config loading

use std::collections::HashMap;
use std::fmt;

/// A configuration value that can be any of the supported types
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigValue {
    /// Null/None value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// String value
    String(String),
    /// List of values
    List(Vec<ConfigValue>),
    /// Dictionary/Map of values
    Dict(ConfigDict),
    /// Interpolation string (e.g., "${foo.bar}")
    Interpolation(String),
    /// Missing value marker
    Missing,
}

impl ConfigValue {
    /// Check if this value is null
    pub fn is_null(&self) -> bool {
        matches!(self, ConfigValue::Null)
    }

    /// Check if this value is missing
    pub fn is_missing(&self) -> bool {
        matches!(self, ConfigValue::Missing)
    }

    /// Get as boolean if this is a bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as integer if this is an int
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ConfigValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as float if this is a float
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(f) => Some(*f),
            ConfigValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Get as string if this is a string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            ConfigValue::Interpolation(s) => Some(s),
            _ => None,
        }
    }

    /// Get as list if this is a list
    pub fn as_list(&self) -> Option<&Vec<ConfigValue>> {
        match self {
            ConfigValue::List(l) => Some(l),
            _ => None,
        }
    }

    /// Get as dict if this is a dict
    pub fn as_dict(&self) -> Option<&ConfigDict> {
        match self {
            ConfigValue::Dict(d) => Some(d),
            _ => None,
        }
    }

    /// Get mutable dict if this is a dict
    pub fn as_dict_mut(&mut self) -> Option<&mut ConfigDict> {
        match self {
            ConfigValue::Dict(d) => Some(d),
            _ => None,
        }
    }

    /// Check if this is an interpolation
    pub fn is_interpolation(&self) -> bool {
        matches!(self, ConfigValue::Interpolation(_))
    }
}

impl Default for ConfigValue {
    fn default() -> Self {
        ConfigValue::Null
    }
}

impl From<bool> for ConfigValue {
    fn from(b: bool) -> Self {
        ConfigValue::Bool(b)
    }
}

impl From<i64> for ConfigValue {
    fn from(i: i64) -> Self {
        ConfigValue::Int(i)
    }
}

impl From<i32> for ConfigValue {
    fn from(i: i32) -> Self {
        ConfigValue::Int(i as i64)
    }
}

impl From<f64> for ConfigValue {
    fn from(f: f64) -> Self {
        ConfigValue::Float(f)
    }
}

impl From<String> for ConfigValue {
    fn from(s: String) -> Self {
        // Check if it's an interpolation
        if s.contains("${") && s.contains('}') {
            ConfigValue::Interpolation(s)
        } else {
            ConfigValue::String(s)
        }
    }
}

impl From<&str> for ConfigValue {
    fn from(s: &str) -> Self {
        ConfigValue::from(s.to_string())
    }
}

impl From<Vec<ConfigValue>> for ConfigValue {
    fn from(v: Vec<ConfigValue>) -> Self {
        ConfigValue::List(v)
    }
}

impl From<ConfigDict> for ConfigValue {
    fn from(d: ConfigDict) -> Self {
        ConfigValue::Dict(d)
    }
}

impl fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigValue::Null => write!(f, "null"),
            ConfigValue::Bool(b) => write!(f, "{}", b),
            ConfigValue::Int(i) => write!(f, "{}", i),
            ConfigValue::Float(fl) => write!(f, "{}", fl),
            ConfigValue::String(s) => write!(f, "{}", s),
            ConfigValue::Interpolation(s) => write!(f, "{}", s),
            ConfigValue::List(l) => {
                write!(f, "[")?;
                for (i, v) in l.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            ConfigValue::Dict(d) => write!(f, "{:?}", d),
            ConfigValue::Missing => write!(f, "???"),
        }
    }
}

/// A dictionary of configuration values
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConfigDict {
    /// The underlying storage - maintains insertion order
    entries: Vec<(String, ConfigValue)>,
    /// Fast lookup index
    index: HashMap<String, usize>,
}

impl ConfigDict {
    /// Create a new empty config dict
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a value at a key
    pub fn insert(&mut self, key: String, value: ConfigValue) {
        if let Some(&idx) = self.index.get(&key) {
            self.entries[idx].1 = value;
        } else {
            let idx = self.entries.len();
            self.entries.push((key.clone(), value));
            self.index.insert(key, idx);
        }
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        self.index.get(key).map(|&idx| &self.entries[idx].1)
    }

    /// Get a mutable value by key
    pub fn get_mut(&mut self, key: &str) -> Option<&mut ConfigValue> {
        if let Some(&idx) = self.index.get(key) {
            Some(&mut self.entries[idx].1)
        } else {
            None
        }
    }

    /// Check if key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    /// Remove a key
    pub fn remove(&mut self, key: &str) -> Option<ConfigValue> {
        if let Some(&idx) = self.index.get(key) {
            self.index.remove(key);
            // Mark as removed but keep entry to maintain indices
            let old = std::mem::replace(&mut self.entries[idx].1, ConfigValue::Null);
            Some(old)
        } else {
            None
        }
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries
            .iter()
            .filter(|(k, _)| self.index.contains_key(k))
            .count()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterate over key-value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ConfigValue)> {
        self.entries
            .iter()
            .filter(|(k, _)| self.index.contains_key(k))
            .map(|(k, v)| (k.as_str(), v))
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.iter().map(|(k, _)| k)
    }

    /// Get all values
    pub fn values(&self) -> impl Iterator<Item = &ConfigValue> {
        self.iter().map(|(_, v)| v)
    }

    /// Select a value using a dotted path (e.g., "a.b.c")
    pub fn select(&self, path: &str) -> Option<ConfigValue> {
        let parts: Vec<&str> = path.split('.').collect();
        self.select_parts(&parts)
    }

    fn select_parts(&self, parts: &[&str]) -> Option<ConfigValue> {
        if parts.is_empty() {
            return None;
        }

        let key = parts[0];
        let value = self.get(key)?;

        if parts.len() == 1 {
            return Some(value.clone());
        }

        match value {
            ConfigValue::Dict(d) => d.select_parts(&parts[1..]),
            _ => None,
        }
    }

    /// Merge another dict into this one
    pub fn merge(&mut self, other: &ConfigDict) {
        for (key, value) in other.iter() {
            let should_deep_merge = matches!(
                (self.get(key), value),
                (Some(ConfigValue::Dict(_)), ConfigValue::Dict(_))
            );

            if should_deep_merge {
                if let ConfigValue::Dict(other_dict) = value {
                    if let Some(self_val) = self.get_mut(key) {
                        if let Some(self_dict) = self_val.as_dict_mut() {
                            self_dict.merge(other_dict);
                            continue;
                        }
                    }
                }
            }
            // Replace value
            self.insert(key.to_string(), value.clone());
        }
    }
}

/// Merge two ConfigDicts (convenience function)
pub fn merge_dicts(target: &mut ConfigDict, source: &ConfigDict) {
    target.merge(source);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_value_types() {
        assert!(ConfigValue::Null.is_null());
        assert!(ConfigValue::Missing.is_missing());
        assert_eq!(ConfigValue::Bool(true).as_bool(), Some(true));
        assert_eq!(ConfigValue::Int(42).as_int(), Some(42));
        assert_eq!(ConfigValue::Float(3.14).as_float(), Some(3.14));
        assert_eq!(
            ConfigValue::String("hello".to_string()).as_str(),
            Some("hello")
        );
    }

    #[test]
    fn test_config_dict_basic() {
        let mut dict = ConfigDict::new();
        dict.insert("name".to_string(), ConfigValue::String("test".to_string()));
        dict.insert("count".to_string(), ConfigValue::Int(42));

        assert_eq!(dict.get("name").unwrap().as_str(), Some("test"));
        assert_eq!(dict.get("count").unwrap().as_int(), Some(42));
        assert!(dict.get("missing").is_none());
    }

    #[test]
    fn test_config_dict_select() {
        let mut inner = ConfigDict::new();
        inner.insert("value".to_string(), ConfigValue::Int(42));

        let mut outer = ConfigDict::new();
        outer.insert("nested".to_string(), ConfigValue::Dict(inner));

        let result = outer.select("nested.value");
        assert_eq!(result.unwrap().as_int(), Some(42));
    }

    #[test]
    fn test_config_dict_merge() {
        let mut base = ConfigDict::new();
        base.insert("a".to_string(), ConfigValue::Int(1));
        base.insert("b".to_string(), ConfigValue::Int(2));

        let mut overlay = ConfigDict::new();
        overlay.insert("b".to_string(), ConfigValue::Int(20));
        overlay.insert("c".to_string(), ConfigValue::Int(3));

        base.merge(&overlay);

        assert_eq!(base.get("a").unwrap().as_int(), Some(1));
        assert_eq!(base.get("b").unwrap().as_int(), Some(20));
        assert_eq!(base.get("c").unwrap().as_int(), Some(3));
    }

    #[test]
    fn test_interpolation_detection() {
        let v = ConfigValue::from("${foo.bar}");
        assert!(v.is_interpolation());

        let v = ConfigValue::from("plain string");
        assert!(!v.is_interpolation());
    }
}
