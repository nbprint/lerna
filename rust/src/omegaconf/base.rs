// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Base types for OmegaConf nodes
//!
//! This module defines the core node hierarchy:
//! - Node: Abstract base for all nodes
//! - Box: Base for nodes that can contain other nodes
//! - Container: Base for DictConfig and ListConfig

use std::sync::{Arc, RwLock, Weak};

use super::errors::Result;
use super::flags::Flags;

/// Metadata for all nodes
#[derive(Debug, Clone)]
pub struct Metadata {
    /// Reference type for the node
    pub ref_type: String,
    /// Object type for the node
    pub object_type: Option<String>,
    /// Whether this node is optional
    pub optional: bool,
    /// Key of this node in its parent
    pub key: Option<NodeKey>,
    /// Flags for this node
    pub flags: Flags,
    /// Whether this node is a flags root
    pub flags_root: bool,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            ref_type: "Any".to_string(),
            object_type: None,
            optional: true,
            key: None,
            flags: Flags::new(),
            flags_root: false,
        }
    }
}

impl Metadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_key(mut self, key: NodeKey) -> Self {
        self.key = Some(key);
        self
    }

    pub fn with_ref_type(mut self, ref_type: impl Into<String>) -> Self {
        self.ref_type = ref_type.into();
        self
    }

    pub fn with_optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }
}

/// Extended metadata for containers (DictConfig, ListConfig)
#[derive(Debug, Clone)]
pub struct ContainerMetadata {
    /// Base metadata
    pub base: Metadata,
    /// Key type for dict containers
    pub key_type: Option<String>,
    /// Element type for containers
    pub element_type: Option<String>,
}

impl Default for ContainerMetadata {
    fn default() -> Self {
        Self {
            base: Metadata::default(),
            key_type: None,
            element_type: None,
        }
    }
}

impl ContainerMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_key_type(mut self, key_type: impl Into<String>) -> Self {
        self.key_type = Some(key_type.into());
        self
    }

    pub fn with_element_type(mut self, element_type: impl Into<String>) -> Self {
        self.element_type = Some(element_type.into());
        self
    }
}

/// A key in a node - can be string (for dicts) or integer (for lists)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeKey {
    String(String),
    Int(i64),
}

impl NodeKey {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            NodeKey::String(s) => Some(s),
            NodeKey::Int(_) => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            NodeKey::Int(i) => Some(*i),
            NodeKey::String(_) => None,
        }
    }
}

impl From<String> for NodeKey {
    fn from(s: String) -> Self {
        NodeKey::String(s)
    }
}

impl From<&str> for NodeKey {
    fn from(s: &str) -> Self {
        NodeKey::String(s.to_string())
    }
}

impl From<i64> for NodeKey {
    fn from(i: i64) -> Self {
        NodeKey::Int(i)
    }
}

impl From<i32> for NodeKey {
    fn from(i: i32) -> Self {
        NodeKey::Int(i as i64)
    }
}

impl From<usize> for NodeKey {
    fn from(i: usize) -> Self {
        NodeKey::Int(i as i64)
    }
}

impl std::fmt::Display for NodeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeKey::String(s) => write!(f, "{}", s),
            NodeKey::Int(i) => write!(f, "{}", i),
        }
    }
}

/// The value content of a node - can be actual content, None, or special strings (MISSING, interpolation)
#[derive(Debug, Clone, PartialEq)]
pub enum NodeContent {
    /// Actual content (stored as Box<dyn Any>)
    Value(NodeValue),
    /// None/null value
    None,
    /// Missing value ("???")
    Missing,
    /// Interpolation string ("${...}")
    Interpolation(String),
}

/// A value that can be stored in a node
#[derive(Debug, Clone, PartialEq)]
pub enum NodeValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    // Complex types are stored as Arc for sharing
    // Dict and List are handled by their respective container types
}

impl NodeValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            NodeValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            NodeValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            NodeValue::Float(f) => Some(*f),
            NodeValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            NodeValue::String(s) => Some(s),
            _ => None,
        }
    }
}

impl NodeContent {
    pub fn is_none(&self) -> bool {
        matches!(self, NodeContent::None)
    }

    pub fn is_missing(&self) -> bool {
        matches!(self, NodeContent::Missing)
    }

    pub fn is_interpolation(&self) -> bool {
        matches!(self, NodeContent::Interpolation(_))
    }

    pub fn is_special(&self) -> bool {
        matches!(
            self,
            NodeContent::None | NodeContent::Missing | NodeContent::Interpolation(_)
        )
    }
}

/// The type of a node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    /// A simple value node
    Value,
    /// A DictConfig container
    Dict,
    /// A ListConfig container
    List,
}

/// Abstract base for all OmegaConf nodes
pub trait Node: Send + Sync + std::fmt::Debug {
    /// Get the type of this node (Value, Dict, or List)
    fn node_type(&self) -> NodeType;

    /// Cast this node to Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;

    /// Get the node's metadata
    fn metadata(&self) -> &Metadata;

    /// Get mutable metadata
    fn metadata_mut(&mut self) -> &mut Metadata;

    /// Get the parent node
    fn parent(&self) -> Option<Arc<RwLock<dyn Node>>>;

    /// Set the parent node
    fn set_parent(&mut self, parent: Option<Weak<RwLock<dyn Node>>>);

    /// Get a flag value, checking parent chain
    fn get_flag(&self, name: &str) -> Option<bool> {
        // Check local flags first
        if let Some(value) = self.metadata().flags.get(name) {
            return Some(value);
        }

        // If this is a flags root, don't check parent
        if self.metadata().flags_root {
            return None;
        }

        // Check parent
        if let Some(parent) = self.parent() {
            if let Ok(parent) = parent.read() {
                return parent.get_flag(name);
            }
        }

        None
    }

    /// Set a flag value on this node
    fn set_flag(&mut self, name: &str, value: Option<bool>) {
        self.metadata_mut().flags.set(name, value);
    }

    /// Get the key of this node in its parent
    fn key(&self) -> Option<&NodeKey> {
        self.metadata().key.as_ref()
    }

    /// Set the key of this node
    fn set_key(&mut self, key: Option<NodeKey>) {
        self.metadata_mut().key = key;
    }

    /// Check if this node is optional
    fn is_optional(&self) -> bool {
        self.metadata().optional
    }

    /// Get the full key path from root to this node
    fn get_full_key(&self) -> String {
        let mut parts = Vec::new();

        if let Some(key) = self.key() {
            parts.push(key.to_string());
        }

        // Walk up parent chain
        let mut current_parent = self.parent();
        while let Some(parent_arc) = current_parent {
            if let Ok(parent) = parent_arc.read() {
                if let Some(key) = parent.key() {
                    parts.push(key.to_string());
                }
                current_parent = parent.parent();
            } else {
                break;
            }
        }

        parts.reverse();
        parts.join(".")
    }

    /// Check if this node has the readonly flag set
    fn is_readonly(&self) -> bool {
        self.get_flag("readonly").unwrap_or(false)
    }

    /// Check if this node has the struct flag set
    fn is_struct(&self) -> bool {
        self.get_flag("struct").unwrap_or(false)
    }

    /// Get the raw value content
    fn content(&self) -> &NodeContent;

    /// Set the raw value content
    fn set_content(&mut self, content: NodeContent) -> Result<()>;

    /// Check if this node is None
    fn is_none(&self) -> bool {
        self.content().is_none()
    }

    /// Check if this node is Missing
    fn is_missing(&self) -> bool {
        self.content().is_missing()
    }

    /// Check if this node is an interpolation
    fn is_interpolation(&self) -> bool {
        self.content().is_interpolation()
    }
}

/// Base for nodes that can contain other nodes (DictConfig, ListConfig, UnionNode)
pub trait Box: Node {
    /// Re-parent all child nodes
    fn re_parent(&mut self);
}

/// Base for container types (DictConfig, ListConfig)
pub trait Container: Box {
    /// Get the container metadata
    fn container_metadata(&self) -> &ContainerMetadata;

    /// Get mutable container metadata
    fn container_metadata_mut(&mut self) -> &mut ContainerMetadata;

    /// Get a child node by key
    fn get_child(&self, key: &NodeKey) -> Option<Arc<RwLock<dyn Node>>>;

    /// Set a child node
    fn set_child(&mut self, key: NodeKey, value: Arc<RwLock<dyn Node>>) -> Result<()>;

    /// Delete a child by key
    fn delete_child(&mut self, key: &NodeKey) -> Result<()>;

    /// Get the number of children
    fn len(&self) -> usize;

    /// Check if the container is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterate over keys
    fn keys(&self) -> Vec<NodeKey>;

    /// Validate a key for get operations
    fn validate_get(&self, key: &NodeKey) -> Result<()>;

    /// Validate a key-value pair for set operations
    fn validate_set(&self, key: &NodeKey, value: &dyn Node) -> Result<()>;

    /// Merge another container into this one
    fn merge_with(&mut self, other: &dyn Container) -> Result<()>;
}
