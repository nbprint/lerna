// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! OmegaConf main API module
//!
//! This module provides the core OmegaConf API functions that mirror the Python API.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::base::{Node, NodeContent, NodeType, NodeValue};
use super::dictconfig::{DictConfig, DictContent};
use super::errors::{InterpolationResolutionError, MissingMandatoryValue, OmegaConfError, Result};
use super::listconfig::ListConfig;
use super::nodes::AnyNode;
use super::{is_missing_literal, MISSING};

/// List merge mode for merging configs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListMergeMode {
    /// Replaces the target list with the new one (default)
    Replace,
    /// Extends the target list with the new one
    Extend,
    /// Extends the target list items with items not present in it
    ExtendUnique,
}

impl Default for ListMergeMode {
    fn default() -> Self {
        ListMergeMode::Replace
    }
}

/// Structured config mode for to_container
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SCMode {
    /// Convert to plain dict
    Dict,
    /// Keep as OmegaConf DictConfig
    DictConfig,
    /// Create a dataclass or attrs class instance
    Instantiate,
}

impl Default for SCMode {
    fn default() -> Self {
        SCMode::Dict
    }
}

/// ConfigValue represents values that can be stored in a config
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    None,
    Missing,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<ConfigValue>),
    Dict(HashMap<String, ConfigValue>),
    Interpolation(String),
}

impl From<bool> for ConfigValue {
    fn from(v: bool) -> Self {
        ConfigValue::Bool(v)
    }
}

impl From<i64> for ConfigValue {
    fn from(v: i64) -> Self {
        ConfigValue::Int(v)
    }
}

impl From<i32> for ConfigValue {
    fn from(v: i32) -> Self {
        ConfigValue::Int(v as i64)
    }
}

impl From<f64> for ConfigValue {
    fn from(v: f64) -> Self {
        ConfigValue::Float(v)
    }
}

impl From<String> for ConfigValue {
    fn from(v: String) -> Self {
        if is_missing_literal(&v) {
            ConfigValue::Missing
        } else if v.starts_with("${") && v.ends_with("}") {
            ConfigValue::Interpolation(v)
        } else {
            ConfigValue::String(v)
        }
    }
}

impl From<&str> for ConfigValue {
    fn from(v: &str) -> Self {
        ConfigValue::from(v.to_string())
    }
}

impl From<Vec<ConfigValue>> for ConfigValue {
    fn from(v: Vec<ConfigValue>) -> Self {
        ConfigValue::List(v)
    }
}

impl From<HashMap<String, ConfigValue>> for ConfigValue {
    fn from(v: HashMap<String, ConfigValue>) -> Self {
        ConfigValue::Dict(v)
    }
}

impl ConfigValue {
    /// Check if this value is missing
    pub fn is_missing(&self) -> bool {
        matches!(self, ConfigValue::Missing)
    }

    /// Check if this value is None
    pub fn is_none(&self) -> bool {
        matches!(self, ConfigValue::None)
    }

    /// Check if this value is an interpolation
    pub fn is_interpolation(&self) -> bool {
        matches!(self, ConfigValue::Interpolation(_))
    }
}

/// Main OmegaConf API struct
pub struct OmegaConf;

impl OmegaConf {
    /// Create a DictConfig from a HashMap of ConfigValues
    pub fn create_dict(content: HashMap<String, ConfigValue>) -> DictConfig {
        let mut cfg = DictConfig::new();
        for (key, value) in content {
            let node = Self::config_value_to_node(value);
            let _ = cfg.set(&key, node);
        }
        cfg
    }

    /// Create a ListConfig from a vector of ConfigValues
    pub fn create_list(content: Vec<ConfigValue>) -> ListConfig {
        let mut cfg = ListConfig::new();
        for value in content {
            let node = Self::config_value_to_node(value);
            let _ = cfg.append(node);
        }
        cfg
    }

    /// Convert a ConfigValue to a Node
    fn config_value_to_node(value: ConfigValue) -> Arc<RwLock<dyn Node>> {
        match value {
            ConfigValue::None => Arc::new(RwLock::new(AnyNode::new(None))),
            ConfigValue::Missing => Arc::new(RwLock::new(AnyNode::missing())),
            ConfigValue::Bool(v) => Arc::new(RwLock::new(AnyNode::with_value(NodeValue::Bool(v)))),
            ConfigValue::Int(v) => Arc::new(RwLock::new(AnyNode::with_value(NodeValue::Int(v)))),
            ConfigValue::Float(v) => {
                Arc::new(RwLock::new(AnyNode::with_value(NodeValue::Float(v))))
            }
            ConfigValue::String(v) => {
                Arc::new(RwLock::new(AnyNode::with_value(NodeValue::String(v))))
            }
            ConfigValue::Bytes(v) => {
                Arc::new(RwLock::new(AnyNode::with_value(NodeValue::Bytes(v))))
            }
            ConfigValue::List(v) => {
                let child = Self::create_list(v);
                Arc::new(RwLock::new(child))
            }
            ConfigValue::Dict(v) => {
                let child = Self::create_dict(v);
                Arc::new(RwLock::new(child))
            }
            ConfigValue::Interpolation(v) => Arc::new(RwLock::new(AnyNode::interpolation(v))),
        }
    }

    /// Check if a node is missing at the given key
    pub fn is_missing_dict(cfg: &DictConfig, key: &str) -> bool {
        match cfg.get(key) {
            Some(node) => {
                let guard = node.read().unwrap();
                guard.is_missing()
            }
            None => false,
        }
    }

    /// Check if a node is missing at the given index
    pub fn is_missing_list(cfg: &ListConfig, index: usize) -> bool {
        match cfg.get(index) {
            Some(node) => {
                let guard = node.read().unwrap();
                guard.is_missing()
            }
            None => false,
        }
    }

    /// Check if a value is an interpolation
    pub fn is_interpolation_dict(cfg: &DictConfig, key: &str) -> bool {
        match cfg.get(key) {
            Some(node) => {
                let guard = node.read().unwrap();
                guard.is_interpolation()
            }
            None => false,
        }
    }

    /// Set the readonly flag on a DictConfig
    pub fn set_readonly_dict(cfg: &mut DictConfig, value: Option<bool>) {
        cfg.set_flag("readonly", value);
    }

    /// Get the readonly flag on a DictConfig
    pub fn is_readonly_dict(cfg: &DictConfig) -> Option<bool> {
        cfg.get_flag("readonly")
    }

    /// Set the struct flag on a DictConfig
    pub fn set_struct_dict(cfg: &mut DictConfig, value: Option<bool>) {
        cfg.set_flag("struct", value);
    }

    /// Get the struct flag on a DictConfig
    pub fn is_struct_dict(cfg: &DictConfig) -> Option<bool> {
        cfg.get_flag("struct")
    }

    /// Convert a DictConfig to a container (HashMap)
    pub fn to_container_dict(
        cfg: &DictConfig,
        resolve: bool,
        throw_on_missing: bool,
    ) -> Result<HashMap<String, ConfigValue>> {
        let mut result = HashMap::new();

        for key_ref in cfg.keys_iter() {
            let key = key_ref.clone();
            if let Some(node) = cfg.get(&key) {
                let guard = node.read().unwrap();
                let value = Self::node_to_config_value(&*guard, resolve, throw_on_missing)?;
                result.insert(key, value);
            }
        }

        Ok(result)
    }

    /// Convert a ListConfig to a container (Vec)
    pub fn to_container_list(
        cfg: &ListConfig,
        resolve: bool,
        throw_on_missing: bool,
    ) -> Result<Vec<ConfigValue>> {
        let mut result = Vec::new();

        for i in 0..cfg.len() {
            if let Some(node) = cfg.get(i) {
                let guard = node.read().unwrap();
                let value = Self::node_to_config_value(&*guard, resolve, throw_on_missing)?;
                result.push(value);
            }
        }

        Ok(result)
    }

    /// Convert a node to a ConfigValue
    fn node_to_config_value(
        node: &dyn Node,
        _resolve: bool,
        throw_on_missing: bool,
    ) -> Result<ConfigValue> {
        let content = node.content();

        match content {
            NodeContent::None => Ok(ConfigValue::None),
            NodeContent::Missing => {
                if throw_on_missing {
                    Err(OmegaConfError::from(MissingMandatoryValue::new(
                        "Missing mandatory value".to_string(),
                    )))
                } else {
                    Ok(ConfigValue::Missing)
                }
            }
            NodeContent::Interpolation(s) => {
                // TODO: If resolve is true, we should resolve the interpolation
                Ok(ConfigValue::Interpolation(s.clone()))
            }
            NodeContent::Value(v) => match v {
                NodeValue::Bool(b) => Ok(ConfigValue::Bool(*b)),
                NodeValue::Int(i) => Ok(ConfigValue::Int(*i)),
                NodeValue::Float(f) => Ok(ConfigValue::Float(*f)),
                NodeValue::String(s) => Ok(ConfigValue::String(s.clone())),
                NodeValue::Bytes(b) => Ok(ConfigValue::Bytes(b.clone())),
            },
        }
    }

    /// Convert a config to YAML string
    pub fn to_yaml_dict(cfg: &DictConfig, resolve: bool, sort_keys: bool) -> Result<String> {
        let container = Self::to_container_dict(cfg, resolve, false)?;
        let yaml = Self::config_value_to_yaml(&ConfigValue::Dict(container), sort_keys, 0)?;
        Ok(format!("{}\n", yaml))
    }

    /// Convert a list config to YAML string
    pub fn to_yaml_list(cfg: &ListConfig, resolve: bool) -> Result<String> {
        let container = Self::to_container_list(cfg, resolve, false)?;
        let yaml = Self::config_value_to_yaml(&ConfigValue::List(container), false, 0)?;
        Ok(format!("{}\n", yaml))
    }

    /// Convert a ConfigValue to YAML string
    fn config_value_to_yaml(value: &ConfigValue, sort_keys: bool, indent: usize) -> Result<String> {
        let indent_str = "  ".repeat(indent);

        match value {
            ConfigValue::None => Ok("null".to_string()),
            ConfigValue::Missing => Ok(MISSING.to_string()),
            ConfigValue::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
            ConfigValue::Int(i) => Ok(i.to_string()),
            ConfigValue::Float(f) => {
                if f.is_nan() {
                    Ok(".nan".to_string())
                } else if f.is_infinite() {
                    if *f > 0.0 {
                        Ok(".inf".to_string())
                    } else {
                        Ok("-.inf".to_string())
                    }
                } else {
                    Ok(f.to_string())
                }
            }
            ConfigValue::String(s) => {
                // Check if we need quoting
                if s.is_empty()
                    || s.contains(':')
                    || s.contains('#')
                    || s.contains('\n')
                    || s.starts_with(' ')
                    || s.ends_with(' ')
                    || s.starts_with('\'')
                    || s.starts_with('"')
                {
                    Ok(format!("'{}'", s.replace('\'', "''")))
                } else {
                    Ok(s.clone())
                }
            }
            ConfigValue::Bytes(_b) => {
                // For simplicity, skip base64 encoding for now
                Ok("!!binary |".to_string())
            }
            ConfigValue::Interpolation(s) => Ok(format!("'{}'", s)),
            ConfigValue::List(items) => {
                if items.is_empty() {
                    return Ok("[]".to_string());
                }
                let mut lines = Vec::new();
                for item in items {
                    let item_yaml = Self::config_value_to_yaml(item, sort_keys, indent + 1)?;
                    if matches!(item, ConfigValue::Dict(_) | ConfigValue::List(_)) {
                        // Complex nested value
                        let nested_lines: Vec<&str> = item_yaml.lines().collect();
                        if !nested_lines.is_empty() {
                            lines.push(format!("{}- {}", indent_str, nested_lines[0]));
                            for line in nested_lines.iter().skip(1) {
                                lines.push(format!("{}  {}", indent_str, line));
                            }
                        }
                    } else {
                        lines.push(format!("{}- {}", indent_str, item_yaml));
                    }
                }
                Ok(lines.join("\n"))
            }
            ConfigValue::Dict(map) => {
                if map.is_empty() {
                    return Ok("{}".to_string());
                }
                let mut lines = Vec::new();
                let mut keys: Vec<&String> = map.keys().collect();
                if sort_keys {
                    keys.sort();
                }
                for key in keys {
                    let v = &map[key];
                    let value_yaml = Self::config_value_to_yaml(v, sort_keys, indent + 1)?;
                    if matches!(v, ConfigValue::Dict(_) | ConfigValue::List(_)) {
                        // Complex nested value
                        lines.push(format!("{}{}:", indent_str, key));
                        for line in value_yaml.lines() {
                            lines.push(line.to_string());
                        }
                    } else {
                        lines.push(format!("{}{}: {}", indent_str, key, value_yaml));
                    }
                }
                Ok(lines.join("\n"))
            }
        }
    }

    /// Merge multiple DictConfigs into one (simplified version)
    pub fn merge_dicts(
        configs: Vec<&DictConfig>,
        _list_merge_mode: ListMergeMode,
    ) -> Result<DictConfig> {
        if configs.is_empty() {
            return Ok(DictConfig::new());
        }

        // Start with a new config
        let mut result = DictConfig::new();

        for cfg in configs {
            for key_ref in cfg.keys_iter() {
                let key = key_ref.clone();
                if let Some(node) = cfg.get(&key) {
                    let _ = result.set(&key, node.clone());
                }
            }
        }

        Ok(result)
    }

    /// Select a value from a DictConfig using a key
    pub fn select_dict(
        cfg: &DictConfig,
        key: &str,
        throw_on_missing: bool,
    ) -> Result<Option<ConfigValue>> {
        // Simple single-key selection for now
        match cfg.get(key) {
            None => Ok(None),
            Some(node) => {
                let guard = node.read().unwrap();

                if guard.is_missing() && throw_on_missing {
                    return Err(OmegaConfError::from(MissingMandatoryValue::new(format!(
                        "Missing mandatory value at key: {}",
                        key
                    ))));
                }

                let value = Self::node_to_config_value(&*guard, false, throw_on_missing)?;
                Ok(Some(value))
            }
        }
    }

    /// Update a value in a DictConfig
    pub fn update_dict(cfg: &mut DictConfig, key: &str, value: ConfigValue) -> Result<()> {
        let node = Self::config_value_to_node(value);
        cfg.set(key, node)
    }

    /// Resolve all interpolations in a DictConfig in-place
    /// This replaces ${...} references with their actual values
    pub fn resolve_dict(cfg: &mut DictConfig) -> Result<()> {
        // Clone keys to avoid borrowing issues during iteration
        let keys: Vec<String> = cfg.keys_iter().cloned().collect();

        for key in keys {
            if let Some(node) = cfg.get(&key) {
                let resolved_value = Self::resolve_node(&node, cfg)?;
                if let Some(value) = resolved_value {
                    let new_node = Self::config_value_to_node(value);
                    // We need to bypass readonly for resolution
                    if let DictContent::Dict(ref mut dict) = cfg.content {
                        dict.insert(key, new_node);
                    }
                }
            }
        }
        Ok(())
    }

    /// Resolve a single node, returning the resolved value if it was an interpolation
    fn resolve_node(
        node: &Arc<RwLock<dyn Node>>,
        root: &DictConfig,
    ) -> Result<Option<ConfigValue>> {
        let guard = node
            .read()
            .map_err(|_| OmegaConfError::from(super::errors::KeyError::new("Lock error")))?;

        if guard.is_interpolation() {
            // Get the interpolation string
            if let Some(any_node) = guard.as_any().downcast_ref::<super::nodes::AnyNode>() {
                if let super::base::NodeContent::Interpolation(expr) = any_node.node_content() {
                    // Parse and resolve the interpolation
                    return Self::resolve_interpolation(expr, root).map(Some);
                }
            }
            Ok(None)
        } else if guard.node_type() == NodeType::Dict {
            // Recursively resolve nested dicts
            drop(guard);
            let node_ref = node
                .read()
                .map_err(|_| OmegaConfError::from(super::errors::KeyError::new("Lock error")))?;
            if let Some(dict) = node_ref.as_any().downcast_ref::<DictConfig>() {
                let mut dict_clone = dict.clone();
                Self::resolve_dict(&mut dict_clone)?;
            }
            Ok(None)
        } else {
            Ok(None)
        }
    }

    /// Resolve an interpolation expression like "${foo.bar}"
    fn resolve_interpolation(expr: &str, root: &DictConfig) -> Result<ConfigValue> {
        // Strip ${} wrapper
        let path = if expr.starts_with("${") && expr.ends_with("}") {
            &expr[2..expr.len() - 1]
        } else {
            expr
        };

        // Handle special resolvers (env, etc.)
        if path.starts_with("env:") {
            let env_var = &path[4..];
            return match std::env::var(env_var) {
                Ok(val) => Ok(ConfigValue::String(val)),
                Err(_) => Err(OmegaConfError::from(InterpolationResolutionError::new(
                    format!("Environment variable '{}' not found", env_var),
                ))),
            };
        }

        // Navigate to the referenced value
        let parts: Vec<&str> = path.split('.').collect();
        Self::select_path(root, &parts)
    }

    /// Select a value from a DictConfig using a path
    fn select_path(cfg: &DictConfig, parts: &[&str]) -> Result<ConfigValue> {
        if parts.is_empty() {
            return Err(OmegaConfError::from(super::errors::KeyError::new(
                "Empty path",
            )));
        }

        let key = parts[0];
        let node = cfg.get(key).ok_or_else(|| {
            OmegaConfError::from(super::errors::KeyError::new(format!(
                "Key '{}' not found",
                key
            )))
        })?;

        let guard = node
            .read()
            .map_err(|_| OmegaConfError::from(super::errors::KeyError::new("Lock error")))?;

        if parts.len() == 1 {
            // Leaf node - convert to ConfigValue
            return Self::node_to_config_value(&*guard, true, true);
        }

        // Need to descend into nested DictConfig
        if guard.node_type() == NodeType::Dict {
            if let Some(nested) = guard.as_any().downcast_ref::<DictConfig>() {
                return Self::select_path(nested, &parts[1..]);
            }
        }

        Err(OmegaConfError::from(super::errors::KeyError::new(format!(
            "Cannot navigate into non-dict at '{}'",
            key
        ))))
    }

    /// Load a YAML file and return a DictConfig
    pub fn load(path: &std::path::Path) -> Result<DictConfig> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            OmegaConfError::from(super::errors::IOError::new(format!(
                "Failed to read file '{}': {}",
                path.display(),
                e
            )))
        })?;

        Self::from_yaml(&content)
    }

    /// Parse YAML string into a DictConfig
    pub fn from_yaml(yaml: &str) -> Result<DictConfig> {
        let parsed: serde_yaml::Value = serde_yaml::from_str(yaml).map_err(|e| {
            OmegaConfError::from(super::errors::ValidationError::new(format!(
                "Failed to parse YAML: {}",
                e
            )))
        })?;

        Self::yaml_value_to_dictconfig(parsed)
    }

    /// Convert a serde_yaml::Value to a DictConfig
    fn yaml_value_to_dictconfig(value: serde_yaml::Value) -> Result<DictConfig> {
        match value {
            serde_yaml::Value::Mapping(map) => {
                let mut content = HashMap::new();
                for (k, v) in map {
                    if let serde_yaml::Value::String(key) = k {
                        let config_value = Self::yaml_value_to_config_value(v)?;
                        content.insert(key, config_value);
                    }
                }
                Ok(OmegaConf::create_dict(content))
            }
            serde_yaml::Value::Null => Ok(DictConfig::none()),
            _ => Err(OmegaConfError::from(super::errors::ValidationError::new(
                "Expected YAML mapping at root",
            ))),
        }
    }

    /// Convert a serde_yaml::Value to a ConfigValue
    fn yaml_value_to_config_value(value: serde_yaml::Value) -> Result<ConfigValue> {
        match value {
            serde_yaml::Value::Null => Ok(ConfigValue::None),
            serde_yaml::Value::Bool(b) => Ok(ConfigValue::Bool(b)),
            serde_yaml::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(ConfigValue::Int(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(ConfigValue::Float(f))
                } else {
                    Ok(ConfigValue::String(n.to_string()))
                }
            }
            serde_yaml::Value::String(s) => Ok(ConfigValue::from(s.as_str())),
            serde_yaml::Value::Sequence(seq) => {
                let items: Result<Vec<ConfigValue>> = seq
                    .into_iter()
                    .map(Self::yaml_value_to_config_value)
                    .collect();
                Ok(ConfigValue::List(items?))
            }
            serde_yaml::Value::Mapping(map) => {
                let mut dict = HashMap::new();
                for (k, v) in map {
                    if let serde_yaml::Value::String(key) = k {
                        let config_value = Self::yaml_value_to_config_value(v)?;
                        dict.insert(key, config_value);
                    }
                }
                Ok(ConfigValue::Dict(dict))
            }
            #[allow(unreachable_patterns)]
            _ => Err(OmegaConfError::from(super::errors::ValidationError::new(
                "Unsupported YAML value type",
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_dict() {
        let mut content = HashMap::new();
        content.insert("key".to_string(), ConfigValue::String("value".to_string()));
        content.insert("num".to_string(), ConfigValue::Int(42));

        let cfg = OmegaConf::create_dict(content);
        assert!(cfg.contains_key("key"));
        assert!(cfg.contains_key("num"));
    }

    #[test]
    fn test_create_list() {
        let content = vec![
            ConfigValue::Int(1),
            ConfigValue::Int(2),
            ConfigValue::Int(3),
        ];

        let cfg = OmegaConf::create_list(content);
        assert_eq!(cfg.len(), 3);
    }

    #[test]
    fn test_missing_value() {
        let mut content = HashMap::new();
        content.insert("key".to_string(), ConfigValue::Missing);

        let cfg = OmegaConf::create_dict(content);
        assert!(OmegaConf::is_missing_dict(&cfg, "key"));
    }

    #[test]
    fn test_to_yaml_simple() {
        let mut content = HashMap::new();
        content.insert("key".to_string(), ConfigValue::String("value".to_string()));

        let cfg = OmegaConf::create_dict(content);
        let yaml = OmegaConf::to_yaml_dict(&cfg, false, false).unwrap();
        assert!(yaml.contains("key: value"));
    }

    #[test]
    fn test_config_value_from_string() {
        let missing = ConfigValue::from("???");
        assert!(matches!(missing, ConfigValue::Missing));

        let interp = ConfigValue::from("${foo.bar}");
        assert!(matches!(interp, ConfigValue::Interpolation(_)));

        let regular = ConfigValue::from("hello");
        assert!(matches!(regular, ConfigValue::String(s) if s == "hello"));
    }
}
