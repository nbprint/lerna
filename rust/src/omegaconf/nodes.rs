// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Value nodes for OmegaConf
//!
//! Value nodes hold primitive values with type information and validation.

use std::sync::{Arc, RwLock, Weak};

use super::base::{Metadata, Node, NodeContent, NodeType, NodeValue};
use super::errors::{OmegaConfError, Result, ValidationError};

/// Base implementation for value nodes
#[derive(Debug)]
pub struct ValueNodeBase {
    pub metadata: Metadata,
    pub content: NodeContent,
    pub parent: Option<Weak<RwLock<dyn Node>>>,
}

impl ValueNodeBase {
    pub fn new(content: NodeContent, metadata: Metadata) -> Self {
        Self {
            metadata,
            content,
            parent: None,
        }
    }

    pub fn with_value(value: NodeValue) -> Self {
        Self {
            metadata: Metadata::default(),
            content: NodeContent::Value(value),
            parent: None,
        }
    }
}

/// A node that can hold any value type
#[derive(Debug)]
pub struct AnyNode {
    base: ValueNodeBase,
}

impl AnyNode {
    pub fn new(value: Option<NodeValue>) -> Self {
        let content = match value {
            Some(v) => NodeContent::Value(v),
            None => NodeContent::None,
        };
        Self {
            base: ValueNodeBase::new(content, Metadata::default()),
        }
    }

    pub fn with_value<T: Into<NodeValue>>(value: T) -> Self {
        Self {
            base: ValueNodeBase::with_value(value.into()),
        }
    }

    pub fn missing() -> Self {
        Self {
            base: ValueNodeBase::new(NodeContent::Missing, Metadata::default()),
        }
    }

    pub fn interpolation(expr: impl Into<String>) -> Self {
        Self {
            base: ValueNodeBase::new(NodeContent::Interpolation(expr.into()), Metadata::default()),
        }
    }

    /// Get a reference to the node content
    pub fn node_content(&self) -> &NodeContent {
        &self.base.content
    }
}

impl Node for AnyNode {
    fn node_type(&self) -> NodeType {
        NodeType::Value
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn metadata(&self) -> &Metadata {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.base.metadata
    }

    fn parent(&self) -> Option<Arc<RwLock<dyn Node>>> {
        self.base.parent.as_ref().and_then(|w| w.upgrade())
    }

    fn set_parent(&mut self, parent: Option<Weak<RwLock<dyn Node>>>) {
        self.base.parent = parent;
    }

    fn content(&self) -> &NodeContent {
        &self.base.content
    }

    fn set_content(&mut self, content: NodeContent) -> Result<()> {
        self.base.content = content;
        Ok(())
    }
}

/// A node that holds a string value
#[derive(Debug)]
pub struct StringNode {
    base: ValueNodeBase,
}

impl StringNode {
    pub fn new(value: Option<String>) -> Self {
        let content = match value {
            Some(v) => NodeContent::Value(NodeValue::String(v)),
            None => NodeContent::None,
        };
        let mut metadata = Metadata::default();
        metadata.ref_type = "str".to_string();
        Self {
            base: ValueNodeBase::new(content, metadata),
        }
    }

    pub fn with_value(value: impl Into<String>) -> Self {
        let mut metadata = Metadata::default();
        metadata.ref_type = "str".to_string();
        Self {
            base: ValueNodeBase::new(
                NodeContent::Value(NodeValue::String(value.into())),
                metadata,
            ),
        }
    }

    pub fn value(&self) -> Option<&str> {
        match &self.base.content {
            NodeContent::Value(NodeValue::String(s)) => Some(s),
            _ => None,
        }
    }

    fn validate(&self, _value: &NodeValue) -> Result<()> {
        // Strings can accept most types via conversion
        Ok(())
    }
}

impl Node for StringNode {
    fn node_type(&self) -> NodeType {
        NodeType::Value
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn metadata(&self) -> &Metadata {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.base.metadata
    }

    fn parent(&self) -> Option<Arc<RwLock<dyn Node>>> {
        self.base.parent.as_ref().and_then(|w| w.upgrade())
    }

    fn set_parent(&mut self, parent: Option<Weak<RwLock<dyn Node>>>) {
        self.base.parent = parent;
    }

    fn content(&self) -> &NodeContent {
        &self.base.content
    }

    fn set_content(&mut self, content: NodeContent) -> Result<()> {
        // Validate if it's a value
        if let NodeContent::Value(ref v) = content {
            self.validate(v)?;
        }
        self.base.content = content;
        Ok(())
    }
}

/// A node that holds an integer value
#[derive(Debug)]
pub struct IntegerNode {
    base: ValueNodeBase,
}

impl IntegerNode {
    pub fn new(value: Option<i64>) -> Self {
        let content = match value {
            Some(v) => NodeContent::Value(NodeValue::Int(v)),
            None => NodeContent::None,
        };
        let mut metadata = Metadata::default();
        metadata.ref_type = "int".to_string();
        Self {
            base: ValueNodeBase::new(content, metadata),
        }
    }

    pub fn with_value(value: i64) -> Self {
        let mut metadata = Metadata::default();
        metadata.ref_type = "int".to_string();
        Self {
            base: ValueNodeBase::new(NodeContent::Value(NodeValue::Int(value)), metadata),
        }
    }

    pub fn value(&self) -> Option<i64> {
        match &self.base.content {
            NodeContent::Value(NodeValue::Int(i)) => Some(*i),
            _ => None,
        }
    }

    fn validate(&self, value: &NodeValue) -> Result<()> {
        match value {
            NodeValue::Int(_) => Ok(()),
            NodeValue::String(s) => {
                // Try to parse as int
                s.parse::<i64>().map(|_| ()).map_err(|_| {
                    ValidationError::new(format!("Cannot convert '{}' to int", s)).into()
                })
            }
            NodeValue::Float(f) => {
                // Allow float if it's a whole number
                if *f == (*f as i64) as f64 {
                    Ok(())
                } else {
                    Err(ValidationError::new(format!("Cannot convert {} to int", f)).into())
                }
            }
            _ => Err(ValidationError::new("Cannot convert value to int").into()),
        }
    }
}

impl Node for IntegerNode {
    fn node_type(&self) -> NodeType {
        NodeType::Value
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn metadata(&self) -> &Metadata {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.base.metadata
    }

    fn parent(&self) -> Option<Arc<RwLock<dyn Node>>> {
        self.base.parent.as_ref().and_then(|w| w.upgrade())
    }

    fn set_parent(&mut self, parent: Option<Weak<RwLock<dyn Node>>>) {
        self.base.parent = parent;
    }

    fn content(&self) -> &NodeContent {
        &self.base.content
    }

    fn set_content(&mut self, content: NodeContent) -> Result<()> {
        if let NodeContent::Value(ref v) = content {
            self.validate(v)?;
        }
        self.base.content = content;
        Ok(())
    }
}

/// A node that holds a float value
#[derive(Debug)]
pub struct FloatNode {
    base: ValueNodeBase,
}

impl FloatNode {
    pub fn new(value: Option<f64>) -> Self {
        let content = match value {
            Some(v) => NodeContent::Value(NodeValue::Float(v)),
            None => NodeContent::None,
        };
        let mut metadata = Metadata::default();
        metadata.ref_type = "float".to_string();
        Self {
            base: ValueNodeBase::new(content, metadata),
        }
    }

    pub fn with_value(value: f64) -> Self {
        let mut metadata = Metadata::default();
        metadata.ref_type = "float".to_string();
        Self {
            base: ValueNodeBase::new(NodeContent::Value(NodeValue::Float(value)), metadata),
        }
    }

    pub fn value(&self) -> Option<f64> {
        match &self.base.content {
            NodeContent::Value(NodeValue::Float(f)) => Some(*f),
            NodeContent::Value(NodeValue::Int(i)) => Some(*i as f64),
            _ => None,
        }
    }
}

impl Node for FloatNode {
    fn node_type(&self) -> NodeType {
        NodeType::Value
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn metadata(&self) -> &Metadata {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.base.metadata
    }

    fn parent(&self) -> Option<Arc<RwLock<dyn Node>>> {
        self.base.parent.as_ref().and_then(|w| w.upgrade())
    }

    fn set_parent(&mut self, parent: Option<Weak<RwLock<dyn Node>>>) {
        self.base.parent = parent;
    }

    fn content(&self) -> &NodeContent {
        &self.base.content
    }

    fn set_content(&mut self, content: NodeContent) -> Result<()> {
        self.base.content = content;
        Ok(())
    }
}

/// A node that holds a boolean value
#[derive(Debug)]
pub struct BooleanNode {
    base: ValueNodeBase,
}

impl BooleanNode {
    pub fn new(value: Option<bool>) -> Self {
        let content = match value {
            Some(v) => NodeContent::Value(NodeValue::Bool(v)),
            None => NodeContent::None,
        };
        let mut metadata = Metadata::default();
        metadata.ref_type = "bool".to_string();
        Self {
            base: ValueNodeBase::new(content, metadata),
        }
    }

    pub fn with_value(value: bool) -> Self {
        let mut metadata = Metadata::default();
        metadata.ref_type = "bool".to_string();
        Self {
            base: ValueNodeBase::new(NodeContent::Value(NodeValue::Bool(value)), metadata),
        }
    }

    pub fn value(&self) -> Option<bool> {
        match &self.base.content {
            NodeContent::Value(NodeValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    fn validate(&self, value: &NodeValue) -> Result<()> {
        match value {
            NodeValue::Bool(_) => Ok(()),
            NodeValue::Int(i) => {
                // 0 and 1 are valid booleans
                if *i == 0 || *i == 1 {
                    Ok(())
                } else {
                    Err(ValidationError::new(format!("Cannot convert {} to bool", i)).into())
                }
            }
            NodeValue::String(s) => {
                let lower = s.to_lowercase();
                if ["true", "false", "yes", "no", "on", "off", "1", "0"].contains(&lower.as_str()) {
                    Ok(())
                } else {
                    Err(ValidationError::new(format!("Cannot convert '{}' to bool", s)).into())
                }
            }
            _ => Err(ValidationError::new("Cannot convert value to bool").into()),
        }
    }
}

impl Node for BooleanNode {
    fn node_type(&self) -> NodeType {
        NodeType::Value
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn metadata(&self) -> &Metadata {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.base.metadata
    }

    fn parent(&self) -> Option<Arc<RwLock<dyn Node>>> {
        self.base.parent.as_ref().and_then(|w| w.upgrade())
    }

    fn set_parent(&mut self, parent: Option<Weak<RwLock<dyn Node>>>) {
        self.base.parent = parent;
    }

    fn content(&self) -> &NodeContent {
        &self.base.content
    }

    fn set_content(&mut self, content: NodeContent) -> Result<()> {
        if let NodeContent::Value(ref v) = content {
            self.validate(v)?;
        }
        self.base.content = content;
        Ok(())
    }
}

/// A marker trait for value nodes
pub trait ValueNode: Node {
    /// Validate and convert a value
    fn validate_and_convert(&self, value: NodeValue) -> Result<NodeValue>;
}

impl ValueNode for AnyNode {
    fn validate_and_convert(&self, value: NodeValue) -> Result<NodeValue> {
        Ok(value)
    }
}

impl ValueNode for StringNode {
    fn validate_and_convert(&self, value: NodeValue) -> Result<NodeValue> {
        match value {
            NodeValue::String(s) => Ok(NodeValue::String(s)),
            NodeValue::Int(i) => Ok(NodeValue::String(i.to_string())),
            NodeValue::Float(f) => Ok(NodeValue::String(f.to_string())),
            NodeValue::Bool(b) => Ok(NodeValue::String(b.to_string())),
            _ => Err(ValidationError::new("Cannot convert value to string").into()),
        }
    }
}

impl ValueNode for IntegerNode {
    fn validate_and_convert(&self, value: NodeValue) -> Result<NodeValue> {
        match value {
            NodeValue::Int(i) => Ok(NodeValue::Int(i)),
            NodeValue::String(s) => {
                let i = s.parse::<i64>().map_err(|_| {
                    OmegaConfError::from(ValidationError::new(format!(
                        "Cannot convert '{}' to int",
                        s
                    )))
                })?;
                Ok(NodeValue::Int(i))
            }
            NodeValue::Float(f) => {
                if f == (f as i64) as f64 {
                    Ok(NodeValue::Int(f as i64))
                } else {
                    Err(ValidationError::new(format!("Cannot convert {} to int", f)).into())
                }
            }
            _ => Err(ValidationError::new("Cannot convert value to int").into()),
        }
    }
}

impl ValueNode for FloatNode {
    fn validate_and_convert(&self, value: NodeValue) -> Result<NodeValue> {
        match value {
            NodeValue::Float(f) => Ok(NodeValue::Float(f)),
            NodeValue::Int(i) => Ok(NodeValue::Float(i as f64)),
            NodeValue::String(s) => {
                let f = s.parse::<f64>().map_err(|_| {
                    OmegaConfError::from(ValidationError::new(format!(
                        "Cannot convert '{}' to float",
                        s
                    )))
                })?;
                Ok(NodeValue::Float(f))
            }
            _ => Err(ValidationError::new("Cannot convert value to float").into()),
        }
    }
}

impl ValueNode for BooleanNode {
    fn validate_and_convert(&self, value: NodeValue) -> Result<NodeValue> {
        match value {
            NodeValue::Bool(b) => Ok(NodeValue::Bool(b)),
            NodeValue::Int(i) => match i {
                0 => Ok(NodeValue::Bool(false)),
                1 => Ok(NodeValue::Bool(true)),
                _ => Err(ValidationError::new(format!("Cannot convert {} to bool", i)).into()),
            },
            NodeValue::String(s) => {
                let lower = s.to_lowercase();
                match lower.as_str() {
                    "true" | "yes" | "on" | "1" => Ok(NodeValue::Bool(true)),
                    "false" | "no" | "off" | "0" => Ok(NodeValue::Bool(false)),
                    _ => {
                        Err(ValidationError::new(format!("Cannot convert '{}' to bool", s)).into())
                    }
                }
            }
            _ => Err(ValidationError::new("Cannot convert value to bool").into()),
        }
    }
}

// Conversions from primitive types to NodeValue
impl From<bool> for NodeValue {
    fn from(b: bool) -> Self {
        NodeValue::Bool(b)
    }
}

impl From<i64> for NodeValue {
    fn from(i: i64) -> Self {
        NodeValue::Int(i)
    }
}

impl From<i32> for NodeValue {
    fn from(i: i32) -> Self {
        NodeValue::Int(i as i64)
    }
}

impl From<f64> for NodeValue {
    fn from(f: f64) -> Self {
        NodeValue::Float(f)
    }
}

impl From<f32> for NodeValue {
    fn from(f: f32) -> Self {
        NodeValue::Float(f as f64)
    }
}

impl From<String> for NodeValue {
    fn from(s: String) -> Self {
        NodeValue::String(s)
    }
}

impl From<&str> for NodeValue {
    fn from(s: &str) -> Self {
        NodeValue::String(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_node() {
        let node = AnyNode::with_value(42i64);
        assert!(!node.is_none());
        assert!(!node.is_missing());
        assert!(!node.is_interpolation());
    }

    #[test]
    fn test_string_node() {
        let node = StringNode::with_value("hello");
        assert_eq!(node.value(), Some("hello"));
    }

    #[test]
    fn test_integer_node() {
        let node = IntegerNode::with_value(42);
        assert_eq!(node.value(), Some(42));
    }

    #[test]
    fn test_float_node() {
        let node = FloatNode::with_value(3.14);
        assert_eq!(node.value(), Some(3.14));
    }

    #[test]
    fn test_boolean_node() {
        let node = BooleanNode::with_value(true);
        assert_eq!(node.value(), Some(true));
    }

    #[test]
    fn test_missing_node() {
        let node = AnyNode::missing();
        assert!(node.is_missing());
    }

    #[test]
    fn test_interpolation_node() {
        let node = AnyNode::interpolation("${foo.bar}");
        assert!(node.is_interpolation());
    }
}
