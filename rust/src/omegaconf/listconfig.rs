// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! ListConfig implementation for OmegaConf
//!
//! ListConfig is a list-like container that supports:
//! - Integer indices
//! - Type validation for elements
//! - Flags (readonly)
//! - Interpolation
//! - MISSING values

use std::sync::{Arc, RwLock, Weak};

use super::base::{
    Box as OmegaBox, Container, ContainerMetadata, Metadata, Node, NodeContent, NodeKey, NodeType,
    NodeValue,
};
use super::errors::{
    ConfigTypeError, KeyValidationError, ReadonlyConfigError, Result, ValidationError,
};
use super::nodes::AnyNode;

/// A list configuration node
#[derive(Debug)]
pub struct ListConfig {
    /// Container metadata
    metadata: ContainerMetadata,
    /// The content
    content: ListContent,
    /// Parent reference
    parent: Option<Weak<RwLock<dyn Node>>>,
}

/// The internal content of a ListConfig
pub enum ListContent {
    /// Actual list content
    List(Vec<Arc<RwLock<dyn Node>>>),
    /// None value
    None,
    /// Missing value
    Missing,
    /// Interpolation
    Interpolation(String),
}

impl Clone for ListContent {
    fn clone(&self) -> Self {
        match self {
            ListContent::List(vec) => ListContent::List(vec.clone()),
            ListContent::None => ListContent::None,
            ListContent::Missing => ListContent::Missing,
            ListContent::Interpolation(s) => ListContent::Interpolation(s.clone()),
        }
    }
}

impl std::fmt::Debug for ListContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListContent::List(vec) => f.debug_struct("List").field("len", &vec.len()).finish(),
            ListContent::None => write!(f, "None"),
            ListContent::Missing => write!(f, "Missing"),
            ListContent::Interpolation(s) => f.debug_tuple("Interpolation").field(s).finish(),
        }
    }
}

impl ListConfig {
    /// Create a new empty ListConfig
    pub fn new() -> Self {
        Self {
            metadata: ContainerMetadata::default(),
            content: ListContent::List(Vec::new()),
            parent: None,
        }
    }

    /// Create a ListConfig representing None
    pub fn none() -> Self {
        Self {
            metadata: ContainerMetadata::default(),
            content: ListContent::None,
            parent: None,
        }
    }

    /// Create a ListConfig representing MISSING
    pub fn missing() -> Self {
        Self {
            metadata: ContainerMetadata::default(),
            content: ListContent::Missing,
            parent: None,
        }
    }

    /// Create a ListConfig from an interpolation
    pub fn interpolation(expr: impl Into<String>) -> Self {
        Self {
            metadata: ContainerMetadata::default(),
            content: ListContent::Interpolation(expr.into()),
            parent: None,
        }
    }

    /// Create a ListConfig from a vector of values
    pub fn from_vec<V: Into<NodeValue>>(items: Vec<V>) -> Self {
        let content: Vec<Arc<RwLock<dyn Node>>> = items
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                let mut node = AnyNode::with_value(v.into());
                node.set_key(Some(NodeKey::Int(i as i64)));
                Arc::new(RwLock::new(node)) as Arc<RwLock<dyn Node>>
            })
            .collect();
        Self {
            metadata: ContainerMetadata::default(),
            content: ListContent::List(content),
            parent: None,
        }
    }

    /// Get an item by index
    pub fn get(&self, index: usize) -> Option<Arc<RwLock<dyn Node>>> {
        match &self.content {
            ListContent::List(list) => list.get(index).cloned(),
            _ => None,
        }
    }

    /// Set an item by index
    pub fn set(&mut self, index: usize, value: Arc<RwLock<dyn Node>>) -> Result<()> {
        if self.is_readonly() {
            return Err(ReadonlyConfigError::new("Cannot modify read-only ListConfig").into());
        }

        if let ListContent::List(ref mut list) = self.content {
            if index >= list.len() {
                return Err(KeyValidationError::new(format!(
                    "Index {} out of range for list of length {}",
                    index,
                    list.len()
                ))
                .into());
            }

            // Set key on the value
            if let Ok(mut node) = value.write() {
                node.set_key(Some(NodeKey::Int(index as i64)));
            }
            list[index] = value;
            Ok(())
        } else {
            Err(ConfigTypeError::new("Cannot set value on non-list ListConfig").into())
        }
    }

    /// Set a primitive value at index
    pub fn set_value<V: Into<NodeValue>>(&mut self, index: usize, value: V) -> Result<()> {
        let node = AnyNode::with_value(value.into());
        self.set(index, Arc::new(RwLock::new(node)))
    }

    /// Append an item to the list
    pub fn append(&mut self, value: Arc<RwLock<dyn Node>>) -> Result<()> {
        if self.is_readonly() {
            return Err(ReadonlyConfigError::new("Cannot append to read-only ListConfig").into());
        }

        if let ListContent::List(ref mut list) = self.content {
            let index = list.len();
            if let Ok(mut node) = value.write() {
                node.set_key(Some(NodeKey::Int(index as i64)));
            }
            list.push(value);
            Ok(())
        } else {
            Err(ConfigTypeError::new("Cannot append to non-list ListConfig").into())
        }
    }

    /// Append a primitive value
    pub fn append_value<V: Into<NodeValue>>(&mut self, value: V) -> Result<()> {
        let node = AnyNode::with_value(value.into());
        self.append(Arc::new(RwLock::new(node)))
    }

    /// Insert an item at an index
    pub fn insert(&mut self, index: usize, value: Arc<RwLock<dyn Node>>) -> Result<()> {
        if self.is_readonly() {
            return Err(ReadonlyConfigError::new("Cannot insert into read-only ListConfig").into());
        }

        if let ListContent::List(ref mut list) = self.content {
            if index > list.len() {
                return Err(KeyValidationError::new(format!(
                    "Index {} out of range for list of length {}",
                    index,
                    list.len()
                ))
                .into());
            }

            if let Ok(mut node) = value.write() {
                node.set_key(Some(NodeKey::Int(index as i64)));
            }
            list.insert(index, value);

            // Update keys for subsequent elements
            self.update_keys();
            Ok(())
        } else {
            Err(ConfigTypeError::new("Cannot insert into non-list ListConfig").into())
        }
    }

    /// Remove an item by index
    pub fn remove(&mut self, index: usize) -> Result<Arc<RwLock<dyn Node>>> {
        if self.is_readonly() {
            return Err(ReadonlyConfigError::new("Cannot remove from read-only ListConfig").into());
        }

        if let ListContent::List(ref mut list) = self.content {
            if index >= list.len() {
                return Err(KeyValidationError::new(format!(
                    "Index {} out of range for list of length {}",
                    index,
                    list.len()
                ))
                .into());
            }

            let removed = list.remove(index);
            self.update_keys();
            Ok(removed)
        } else {
            Err(ConfigTypeError::new("Cannot remove from non-list ListConfig").into())
        }
    }

    /// Pop the last item
    pub fn pop(&mut self) -> Result<Option<Arc<RwLock<dyn Node>>>> {
        if self.is_readonly() {
            return Err(ReadonlyConfigError::new("Cannot pop from read-only ListConfig").into());
        }

        if let ListContent::List(ref mut list) = self.content {
            Ok(list.pop())
        } else {
            Err(ConfigTypeError::new("Cannot pop from non-list ListConfig").into())
        }
    }

    /// Extend with items from another iterator
    pub fn extend<I>(&mut self, items: I) -> Result<()>
    where
        I: IntoIterator<Item = Arc<RwLock<dyn Node>>>,
    {
        if self.is_readonly() {
            return Err(ReadonlyConfigError::new("Cannot extend read-only ListConfig").into());
        }

        if let ListContent::List(ref mut list) = self.content {
            for item in items {
                let index = list.len();
                if let Ok(mut node) = item.write() {
                    node.set_key(Some(NodeKey::Int(index as i64)));
                }
                list.push(item);
            }
            Ok(())
        } else {
            Err(ConfigTypeError::new("Cannot extend non-list ListConfig").into())
        }
    }

    /// Clear the list
    pub fn clear(&mut self) -> Result<()> {
        if self.is_readonly() {
            return Err(ReadonlyConfigError::new("Cannot clear read-only ListConfig").into());
        }

        if let ListContent::List(ref mut list) = self.content {
            list.clear();
            Ok(())
        } else {
            Err(ConfigTypeError::new("Cannot clear non-list ListConfig").into())
        }
    }

    /// Get the length
    pub fn len_internal(&self) -> usize {
        match &self.content {
            ListContent::List(list) => list.len(),
            _ => 0,
        }
    }

    /// Public len() method for convenience
    pub fn len(&self) -> usize {
        self.len_internal()
    }

    /// Check if empty
    pub fn is_empty_internal(&self) -> bool {
        self.len_internal() == 0
    }

    /// Public is_empty() method for convenience
    pub fn is_empty(&self) -> bool {
        self.is_empty_internal()
    }

    /// Update keys after insert/remove
    fn update_keys(&mut self) {
        if let ListContent::List(ref list) = self.content {
            for (i, item) in list.iter().enumerate() {
                if let Ok(mut node) = item.write() {
                    node.set_key(Some(NodeKey::Int(i as i64)));
                }
            }
        }
    }

    /// Iterate over items
    pub fn iter(&self) -> impl Iterator<Item = &Arc<RwLock<dyn Node>>> {
        match &self.content {
            ListContent::List(list) => list.iter().collect::<Vec<_>>().into_iter(),
            _ => vec![].into_iter(),
        }
    }
}

impl Default for ListConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ListConfig {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            content: self.content.clone(),
            parent: None, // Don't clone parent reference
        }
    }
}

impl Node for ListConfig {
    fn node_type(&self) -> NodeType {
        NodeType::List
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
        match &self.content {
            ListContent::None => &NodeContent::None,
            ListContent::Missing => &NodeContent::Missing,
            _ => &NodeContent::None, // Placeholder
        }
    }

    fn set_content(&mut self, content: NodeContent) -> Result<()> {
        match content {
            NodeContent::None => {
                self.content = ListContent::None;
            }
            NodeContent::Missing => {
                self.content = ListContent::Missing;
            }
            NodeContent::Interpolation(s) => {
                self.content = ListContent::Interpolation(s);
            }
            NodeContent::Value(_) => {
                return Err(ValidationError::new("Cannot set value content on ListConfig").into());
            }
        }
        Ok(())
    }

    fn is_none(&self) -> bool {
        matches!(self.content, ListContent::None)
    }

    fn is_missing(&self) -> bool {
        matches!(self.content, ListContent::Missing)
    }

    fn is_interpolation(&self) -> bool {
        matches!(self.content, ListContent::Interpolation(_))
    }
}

impl OmegaBox for ListConfig {
    fn re_parent(&mut self) {
        // Would need Arc<RwLock<Self>> to properly implement
    }
}

impl Container for ListConfig {
    fn container_metadata(&self) -> &ContainerMetadata {
        &self.metadata
    }

    fn container_metadata_mut(&mut self) -> &mut ContainerMetadata {
        &mut self.metadata
    }

    fn get_child(&self, key: &NodeKey) -> Option<Arc<RwLock<dyn Node>>> {
        match key {
            NodeKey::Int(i) if *i >= 0 => self.get(*i as usize),
            NodeKey::String(s) => s.parse::<usize>().ok().and_then(|i| self.get(i)),
            _ => None,
        }
    }

    fn set_child(&mut self, key: NodeKey, value: Arc<RwLock<dyn Node>>) -> Result<()> {
        match key {
            NodeKey::Int(i) if i >= 0 => self.set(i as usize, value),
            NodeKey::String(s) => {
                let i = s
                    .parse::<usize>()
                    .map_err(|_| KeyValidationError::new(format!("Invalid list index: {}", s)))?;
                self.set(i, value)
            }
            _ => Err(KeyValidationError::new("Invalid list index").into()),
        }
    }

    fn delete_child(&mut self, key: &NodeKey) -> Result<()> {
        match key {
            NodeKey::Int(i) if *i >= 0 => {
                self.remove(*i as usize)?;
                Ok(())
            }
            NodeKey::String(s) => {
                let i = s
                    .parse::<usize>()
                    .map_err(|_| KeyValidationError::new(format!("Invalid list index: {}", s)))?;
                self.remove(i)?;
                Ok(())
            }
            _ => Err(KeyValidationError::new("Invalid list index").into()),
        }
    }

    fn len(&self) -> usize {
        self.len_internal()
    }

    fn keys(&self) -> Vec<NodeKey> {
        (0..self.len_internal())
            .map(|i| NodeKey::Int(i as i64))
            .collect()
    }

    fn validate_get(&self, key: &NodeKey) -> Result<()> {
        match key {
            NodeKey::Int(i) if *i >= 0 => Ok(()),
            NodeKey::Int(_) => {
                Err(KeyValidationError::new("ListConfig indices must be non-negative").into())
            }
            _ => Err(KeyValidationError::new(
                "ListConfig indices must be integers or slices, not $KEY_TYPE",
            )
            .into()),
        }
    }

    fn validate_set(&self, key: &NodeKey, _value: &dyn Node) -> Result<()> {
        if self.is_readonly() {
            return Err(ReadonlyConfigError::new("Cannot modify read-only ListConfig").into());
        }
        self.validate_get(key)
    }

    fn merge_with(&mut self, other: &dyn Container) -> Result<()> {
        // For lists, we typically replace the content
        if let ListContent::List(ref mut list) = self.content {
            list.clear();
            for key in other.keys() {
                if let Some(value) = other.get_child(&key) {
                    self.append(value)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listconfig_new() {
        let lc = ListConfig::new();
        assert!(lc.is_empty_internal());
        assert!(!lc.is_none());
        assert!(!lc.is_missing());
    }

    #[test]
    fn test_listconfig_append() {
        let mut lc = ListConfig::new();
        lc.append_value(1i64).unwrap();
        lc.append_value(2i64).unwrap();
        lc.append_value(3i64).unwrap();

        assert_eq!(lc.len_internal(), 3);
    }

    #[test]
    fn test_listconfig_from_vec() {
        let lc = ListConfig::from_vec(vec![1i64, 2i64, 3i64]);
        assert_eq!(lc.len_internal(), 3);
    }

    #[test]
    fn test_listconfig_readonly() {
        let mut lc = ListConfig::new();
        lc.append_value(1i64).unwrap();
        lc.set_flag("readonly", Some(true));

        let result = lc.append_value(2i64);
        assert!(result.is_err());
    }

    #[test]
    fn test_listconfig_pop() {
        let mut lc = ListConfig::new();
        lc.append_value(1i64).unwrap();
        lc.append_value(2i64).unwrap();

        let popped = lc.pop().unwrap();
        assert!(popped.is_some());
        assert_eq!(lc.len_internal(), 1);
    }

    #[test]
    fn test_listconfig_none() {
        let lc = ListConfig::none();
        assert!(lc.is_none());
    }

    #[test]
    fn test_listconfig_missing() {
        let lc = ListConfig::missing();
        assert!(lc.is_missing());
    }
}
