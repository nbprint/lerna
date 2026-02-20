// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! DictConfig implementation for OmegaConf
//!
//! DictConfig is a dictionary-like container that supports:
//! - String keys
//! - Type validation
//! - Flags (struct, readonly)
//! - Interpolation
//! - MISSING values

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

use super::base::{
    Box as OmegaBox, Container, ContainerMetadata, Metadata, Node, NodeContent, NodeKey, NodeType,
    NodeValue,
};
use super::errors::{ConfigTypeError, ReadonlyConfigError, Result, ValidationError};
use super::nodes::AnyNode;

/// A dictionary configuration node
#[derive(Debug)]
pub struct DictConfig {
    /// Container metadata
    metadata: ContainerMetadata,
    /// The content - can be dict, None, or special string
    pub content: DictContent,
    /// Parent reference
    parent: Option<Weak<RwLock<dyn Node>>>,
}

/// The internal content of a DictConfig
pub enum DictContent {
    /// Actual dictionary content
    Dict(HashMap<String, Arc<RwLock<dyn Node>>>),
    /// None value
    None,
    /// Missing value
    Missing,
    /// Interpolation
    Interpolation(String),
}

impl Clone for DictContent {
    fn clone(&self) -> Self {
        match self {
            DictContent::Dict(map) => DictContent::Dict(map.clone()),
            DictContent::None => DictContent::None,
            DictContent::Missing => DictContent::Missing,
            DictContent::Interpolation(s) => DictContent::Interpolation(s.clone()),
        }
    }
}

impl std::fmt::Debug for DictContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DictContent::Dict(map) => {
                let keys: Vec<_> = map.keys().collect();
                f.debug_struct("Dict").field("keys", &keys).finish()
            }
            DictContent::None => write!(f, "None"),
            DictContent::Missing => write!(f, "Missing"),
            DictContent::Interpolation(s) => f.debug_tuple("Interpolation").field(s).finish(),
        }
    }
}

impl DictConfig {
    /// Create a new empty DictConfig
    pub fn new() -> Self {
        Self {
            metadata: ContainerMetadata::default(),
            content: DictContent::Dict(HashMap::new()),
            parent: None,
        }
    }

    /// Create a DictConfig representing None
    pub fn none() -> Self {
        Self {
            metadata: ContainerMetadata::default(),
            content: DictContent::None,
            parent: None,
        }
    }

    /// Create a DictConfig representing MISSING
    pub fn missing() -> Self {
        Self {
            metadata: ContainerMetadata::default(),
            content: DictContent::Missing,
            parent: None,
        }
    }

    /// Create a DictConfig from an interpolation
    pub fn interpolation(expr: impl Into<String>) -> Self {
        Self {
            metadata: ContainerMetadata::default(),
            content: DictContent::Interpolation(expr.into()),
            parent: None,
        }
    }

    /// Create a DictConfig from a HashMap of values
    pub fn from_map<K, V>(map: HashMap<K, V>) -> Self
    where
        K: Into<String>,
        V: Into<NodeValue>,
    {
        let mut content = HashMap::new();
        for (k, v) in map {
            let key = k.into();
            let node = AnyNode::with_value(v.into());
            content.insert(
                key.clone(),
                Arc::new(RwLock::new(node)) as Arc<RwLock<dyn Node>>,
            );
        }
        Self {
            metadata: ContainerMetadata::default(),
            content: DictContent::Dict(content),
            parent: None,
        }
    }

    /// Get a value by string key
    pub fn get(&self, key: &str) -> Option<Arc<RwLock<dyn Node>>> {
        match &self.content {
            DictContent::Dict(dict) => dict.get(key).cloned(),
            _ => None,
        }
    }

    /// Set a value by string key
    pub fn set(&mut self, key: impl Into<String>, value: Arc<RwLock<dyn Node>>) -> Result<()> {
        // Check readonly
        if self.is_readonly() {
            return Err(ReadonlyConfigError::new("Cannot modify read-only DictConfig").into());
        }

        let key = key.into();

        // Check struct mode
        if self.is_struct() {
            if let DictContent::Dict(dict) = &self.content {
                if !dict.contains_key(&key) {
                    return Err(ConfigTypeError::new(format!(
                        "Struct mode: key '{}' not found in DictConfig",
                        key
                    ))
                    .into());
                }
            }
        }

        if let DictContent::Dict(ref mut dict) = self.content {
            // Set parent on the value
            if let Ok(mut node) = value.write() {
                node.set_key(Some(NodeKey::String(key.clone())));
                // Note: We can't set parent here without Arc<RwLock<Self>>
            }
            dict.insert(key, value);
            Ok(())
        } else {
            Err(ConfigTypeError::new("Cannot set value on non-dict DictConfig").into())
        }
    }

    /// Set a primitive value
    pub fn set_value<V: Into<NodeValue>>(
        &mut self,
        key: impl Into<String>,
        value: V,
    ) -> Result<()> {
        let node = AnyNode::with_value(value.into());
        self.set(key, Arc::new(RwLock::new(node)))
    }

    /// Remove a key
    pub fn remove(&mut self, key: &str) -> Result<Option<Arc<RwLock<dyn Node>>>> {
        if self.is_readonly() {
            return Err(ReadonlyConfigError::new("Cannot delete from read-only DictConfig").into());
        }

        if self.is_struct() {
            return Err(ConfigTypeError::new(
                "DictConfig in struct mode does not support deletion",
            )
            .into());
        }

        if let DictContent::Dict(ref mut dict) = self.content {
            Ok(dict.remove(key))
        } else {
            Err(ConfigTypeError::new("Cannot remove from non-dict DictConfig").into())
        }
    }

    /// Check if a key exists
    pub fn contains_key(&self, key: &str) -> bool {
        match &self.content {
            DictContent::Dict(dict) => dict.contains_key(key),
            _ => false,
        }
    }

    /// Get all keys
    pub fn keys_iter(&self) -> impl Iterator<Item = &String> {
        match &self.content {
            DictContent::Dict(dict) => dict.keys().collect::<Vec<_>>().into_iter(),
            _ => vec![].into_iter(),
        }
    }

    /// Get number of entries
    pub fn len_internal(&self) -> usize {
        match &self.content {
            DictContent::Dict(dict) => dict.len(),
            _ => 0,
        }
    }

    /// Check if empty
    pub fn is_empty_internal(&self) -> bool {
        self.len_internal() == 0
    }

    /// Select a value using a dotted path
    pub fn select(&self, path: &str) -> Option<Arc<RwLock<dyn Node>>> {
        let parts: Vec<&str> = path.split('.').collect();
        self.select_path(&parts)
    }

    fn select_path(&self, parts: &[&str]) -> Option<Arc<RwLock<dyn Node>>> {
        if parts.is_empty() {
            return None;
        }

        let key = parts[0];
        let node = self.get(key)?;

        if parts.len() == 1 {
            return Some(node);
        }

        // Try to descend into nested DictConfig
        let node_guard = node.read().ok()?;
        // We need to check if this is a DictConfig somehow
        // For now, we'll handle this at the Python binding level
        drop(node_guard);
        None
    }

    /// Iterate over key-value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<RwLock<dyn Node>>)> {
        match &self.content {
            DictContent::Dict(dict) => dict.iter().collect::<Vec<_>>().into_iter(),
            _ => vec![].into_iter(),
        }
    }

    /// Deep merge another DictConfig into this one
    pub fn merge(&mut self, other: &DictConfig) -> Result<()> {
        if self.is_readonly() {
            return Err(ReadonlyConfigError::new("Cannot merge into read-only DictConfig").into());
        }

        // Handle special content in source
        match &other.content {
            DictContent::None => {
                self.content = DictContent::None;
                return Ok(());
            }
            DictContent::Interpolation(expr) => {
                self.content = DictContent::Interpolation(expr.clone());
                return Ok(());
            }
            DictContent::Missing => {
                // MISSING in source means keep destination
                return Ok(());
            }
            DictContent::Dict(_) => {}
        }

        // Merge dict content
        if let DictContent::Dict(other_dict) = &other.content {
            for (key, value) in other_dict.iter() {
                // Clone the value for insertion
                self.set(key.clone(), value.clone())?;
            }
        }

        Ok(())
    }
}

impl Default for DictConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DictConfig {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            content: self.content.clone(),
            parent: None, // Don't clone parent reference
        }
    }
}

impl Node for DictConfig {
    fn node_type(&self) -> NodeType {
        NodeType::Dict
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn metadata(&self) -> &Metadata {
        &self.metadata.base
    }

    fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.metadata.base
    }

    fn parent(&self) -> Option<Arc<RwLock<dyn Node>>> {
        self.parent.as_ref().and_then(|w| w.upgrade())
    }

    fn set_parent(&mut self, parent: Option<Weak<RwLock<dyn Node>>>) {
        self.parent = parent;
    }

    fn content(&self) -> &NodeContent {
        // Convert DictContent to NodeContent for the trait
        // This is a bit of a hack - we return a placeholder
        match &self.content {
            DictContent::None => &NodeContent::None,
            DictContent::Missing => &NodeContent::Missing,
            DictContent::Interpolation(_s) => {
                // We can't return a reference to a temporary, so we use a static
                // This is a limitation - proper implementation would need interior mutability
                &NodeContent::None // Placeholder
            }
            DictContent::Dict(_) => &NodeContent::None, // Placeholder
        }
    }

    fn set_content(&mut self, content: NodeContent) -> Result<()> {
        match content {
            NodeContent::None => {
                self.content = DictContent::None;
            }
            NodeContent::Missing => {
                self.content = DictContent::Missing;
            }
            NodeContent::Interpolation(s) => {
                self.content = DictContent::Interpolation(s);
            }
            NodeContent::Value(_) => {
                return Err(ValidationError::new("Cannot set value content on DictConfig").into());
            }
        }
        Ok(())
    }

    fn is_none(&self) -> bool {
        matches!(self.content, DictContent::None)
    }

    fn is_missing(&self) -> bool {
        matches!(self.content, DictContent::Missing)
    }

    fn is_interpolation(&self) -> bool {
        matches!(self.content, DictContent::Interpolation(_))
    }
}

impl OmegaBox for DictConfig {
    fn re_parent(&mut self) {
        // Would need Arc<RwLock<Self>> to properly implement
        // For now, this is a no-op
    }
}

impl Container for DictConfig {
    fn container_metadata(&self) -> &ContainerMetadata {
        &self.metadata
    }

    fn container_metadata_mut(&mut self) -> &mut ContainerMetadata {
        &mut self.metadata
    }

    fn get_child(&self, key: &NodeKey) -> Option<Arc<RwLock<dyn Node>>> {
        match key {
            NodeKey::String(s) => self.get(s),
            NodeKey::Int(i) => self.get(&i.to_string()),
        }
    }

    fn set_child(&mut self, key: NodeKey, value: Arc<RwLock<dyn Node>>) -> Result<()> {
        match key {
            NodeKey::String(s) => self.set(s, value),
            NodeKey::Int(i) => self.set(i.to_string(), value),
        }
    }

    fn delete_child(&mut self, key: &NodeKey) -> Result<()> {
        match key {
            NodeKey::String(s) => {
                self.remove(s)?;
                Ok(())
            }
            NodeKey::Int(i) => {
                self.remove(&i.to_string())?;
                Ok(())
            }
        }
    }

    fn len(&self) -> usize {
        self.len_internal()
    }

    fn keys(&self) -> Vec<NodeKey> {
        self.keys_iter()
            .map(|s| NodeKey::String(s.clone()))
            .collect()
    }

    fn validate_get(&self, key: &NodeKey) -> Result<()> {
        // Dict configs accept string keys
        match key {
            NodeKey::String(_) => Ok(()),
            NodeKey::Int(_) => Ok(()), // We allow int keys as string conversion
        }
    }

    fn validate_set(&self, key: &NodeKey, _value: &dyn Node) -> Result<()> {
        if self.is_readonly() {
            return Err(ReadonlyConfigError::new("Cannot modify read-only DictConfig").into());
        }
        self.validate_get(key)
    }

    fn merge_with(&mut self, other: &dyn Container) -> Result<()> {
        // For now, just handle keys that exist in other
        for key in other.keys() {
            if let Some(value) = other.get_child(&key) {
                self.set_child(key, value)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictconfig_new() {
        let dc = DictConfig::new();
        assert!(dc.is_empty_internal());
        assert!(!dc.is_none());
        assert!(!dc.is_missing());
    }

    #[test]
    fn test_dictconfig_set_get() {
        let mut dc = DictConfig::new();
        dc.set_value("name", "test").unwrap();
        dc.set_value("count", 42i64).unwrap();

        let node = dc.get("name").unwrap();
        let guard = node.read().unwrap();
        // We can verify the node exists
        assert!(!guard.is_missing());
    }

    #[test]
    fn test_dictconfig_readonly() {
        let mut dc = DictConfig::new();
        dc.set_value("initial", 1i64).unwrap();
        dc.set_flag("readonly", Some(true));

        let result = dc.set_value("new_key", 2i64);
        assert!(result.is_err());
    }

    #[test]
    fn test_dictconfig_struct() {
        let mut dc = DictConfig::new();
        dc.set_value("existing", 1i64).unwrap();
        dc.set_flag("struct", Some(true));

        // Should fail to add new key
        let result = dc.set_value("new_key", 2i64);
        assert!(result.is_err());

        // Should succeed for existing key
        let result = dc.set_value("existing", 3i64);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dictconfig_none() {
        let dc = DictConfig::none();
        assert!(dc.is_none());
        assert!(!dc.is_missing());
    }

    #[test]
    fn test_dictconfig_missing() {
        let dc = DictConfig::missing();
        assert!(dc.is_missing());
        assert!(!dc.is_none());
    }
}
