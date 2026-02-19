// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Validation module for configuration values
//!
//! Provides validation for structured configs and type checking.

use std::collections::HashMap;

use crate::config::value::{ConfigDict, ConfigValue};

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
}

impl ValidationError {
    pub fn new(path: &str, message: &str) -> Self {
        Self {
            path: path.to_string(),
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Type specification for validation
#[derive(Debug, Clone, PartialEq)]
pub enum TypeSpec {
    Any,
    Null,
    Bool,
    Int,
    Float,
    String,
    List(Box<TypeSpec>),
    Dict(Box<TypeSpec>),
    Optional(Box<TypeSpec>),
    Union(Vec<TypeSpec>),
}

impl TypeSpec {
    /// Parse a type specification from a string
    /// Examples: "int", "str", "List[int]", "Optional[str]", "Dict[str, int]"
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();

        // Simple types
        match s.to_lowercase().as_str() {
            "any" => return Some(TypeSpec::Any),
            "none" | "null" => return Some(TypeSpec::Null),
            "bool" | "boolean" => return Some(TypeSpec::Bool),
            "int" | "integer" => return Some(TypeSpec::Int),
            "float" | "number" => return Some(TypeSpec::Float),
            "str" | "string" => return Some(TypeSpec::String),
            _ => {}
        }

        // Generic types: List[T], Optional[T], Dict[K,V], Union[A,B]
        if let Some(inner) = s.strip_prefix("List[").and_then(|s| s.strip_suffix(']')) {
            return TypeSpec::parse(inner).map(|t| TypeSpec::List(Box::new(t)));
        }

        if let Some(inner) = s
            .strip_prefix("Optional[")
            .and_then(|s| s.strip_suffix(']'))
        {
            return TypeSpec::parse(inner).map(|t| TypeSpec::Optional(Box::new(t)));
        }

        if let Some(inner) = s.strip_prefix("Union[").and_then(|s| s.strip_suffix(']')) {
            let types: Vec<_> = inner
                .split(',')
                .filter_map(|t| TypeSpec::parse(t.trim()))
                .collect();
            if !types.is_empty() {
                return Some(TypeSpec::Union(types));
            }
        }

        if let Some(inner) = s.strip_prefix("Dict[").and_then(|s| s.strip_suffix(']')) {
            // Dict[str, T] - we only care about value type
            if let Some(comma) = inner.find(',') {
                let value_type = &inner[comma + 1..];
                return TypeSpec::parse(value_type.trim()).map(|t| TypeSpec::Dict(Box::new(t)));
            }
        }

        None
    }

    /// Check if a value matches this type specification
    pub fn matches(&self, value: &ConfigValue) -> bool {
        match (self, value) {
            (TypeSpec::Any, _) => true,
            (TypeSpec::Null, ConfigValue::Null) => true,
            (TypeSpec::Bool, ConfigValue::Bool(_)) => true,
            (TypeSpec::Int, ConfigValue::Int(_)) => true,
            (TypeSpec::Float, ConfigValue::Float(_)) => true,
            (TypeSpec::Float, ConfigValue::Int(_)) => true, // int is valid for float
            (TypeSpec::String, ConfigValue::String(_)) => true,
            (TypeSpec::String, ConfigValue::Interpolation(_)) => true, // interpolations resolve to strings
            (TypeSpec::List(inner), ConfigValue::List(items)) => {
                items.iter().all(|item| inner.matches(item))
            }
            (TypeSpec::Dict(inner), ConfigValue::Dict(dict)) => {
                dict.values().all(|v| inner.matches(v))
            }
            (TypeSpec::Optional(_inner), ConfigValue::Null) => true,
            (TypeSpec::Optional(inner), value) => inner.matches(value),
            (TypeSpec::Union(types), value) => types.iter().any(|t| t.matches(value)),
            _ => false,
        }
    }
}

/// Schema for structured config validation
#[derive(Debug, Clone)]
pub struct ConfigSchema {
    /// Field name -> (type_spec, required, default)
    pub fields: HashMap<String, (TypeSpec, bool, Option<ConfigValue>)>,
}

impl ConfigSchema {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Add a required field
    pub fn required(mut self, name: &str, type_spec: TypeSpec) -> Self {
        self.fields
            .insert(name.to_string(), (type_spec, true, None));
        self
    }

    /// Add an optional field with a default
    pub fn optional(mut self, name: &str, type_spec: TypeSpec, default: ConfigValue) -> Self {
        self.fields
            .insert(name.to_string(), (type_spec, false, Some(default)));
        self
    }

    /// Validate a config dict against this schema
    pub fn validate(&self, config: &ConfigDict) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Check required fields exist
        for (name, (type_spec, required, _default)) in &self.fields {
            if *required && config.get(name).is_none() {
                errors.push(ValidationError::new(name, "Missing required field"));
                continue;
            }

            if let Some(value) = config.get(name) {
                if !type_spec.matches(value) {
                    errors.push(ValidationError::new(
                        name,
                        &format!("Type mismatch: expected {:?}", type_spec),
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Apply defaults to a config dict
    pub fn apply_defaults(&self, config: &mut ConfigDict) {
        for (name, (_type_spec, _required, default)) in &self.fields {
            if config.get(name).is_none() {
                if let Some(default_value) = default {
                    config.insert(name.clone(), default_value.clone());
                }
            }
        }
    }
}

impl Default for ConfigSchema {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_spec_parse() {
        assert_eq!(TypeSpec::parse("int"), Some(TypeSpec::Int));
        assert_eq!(TypeSpec::parse("str"), Some(TypeSpec::String));
        assert_eq!(TypeSpec::parse("bool"), Some(TypeSpec::Bool));
        assert_eq!(TypeSpec::parse("float"), Some(TypeSpec::Float));
        assert_eq!(TypeSpec::parse("Any"), Some(TypeSpec::Any));
    }

    #[test]
    fn test_type_spec_parse_generic() {
        assert_eq!(
            TypeSpec::parse("List[int]"),
            Some(TypeSpec::List(Box::new(TypeSpec::Int)))
        );
        assert_eq!(
            TypeSpec::parse("Optional[str]"),
            Some(TypeSpec::Optional(Box::new(TypeSpec::String)))
        );
    }

    #[test]
    fn test_type_spec_matches() {
        assert!(TypeSpec::Int.matches(&ConfigValue::Int(42)));
        assert!(!TypeSpec::Int.matches(&ConfigValue::String("42".to_string())));
        assert!(TypeSpec::Float.matches(&ConfigValue::Int(42))); // int is valid for float
        assert!(TypeSpec::Optional(Box::new(TypeSpec::Int)).matches(&ConfigValue::Null));
        assert!(TypeSpec::Optional(Box::new(TypeSpec::Int)).matches(&ConfigValue::Int(42)));
    }

    #[test]
    fn test_schema_validation() {
        let schema = ConfigSchema::new()
            .required("name", TypeSpec::String)
            .required("port", TypeSpec::Int)
            .optional(
                "host",
                TypeSpec::String,
                ConfigValue::String("localhost".to_string()),
            );

        let mut config = ConfigDict::new();
        config.insert("name".to_string(), ConfigValue::String("test".to_string()));
        config.insert("port".to_string(), ConfigValue::Int(8080));

        assert!(schema.validate(&config).is_ok());

        // Missing required field
        let mut bad_config = ConfigDict::new();
        bad_config.insert("name".to_string(), ConfigValue::String("test".to_string()));
        assert!(schema.validate(&bad_config).is_err());
    }

    #[test]
    fn test_schema_apply_defaults() {
        let schema = ConfigSchema::new()
            .optional(
                "host",
                TypeSpec::String,
                ConfigValue::String("localhost".to_string()),
            )
            .optional("port", TypeSpec::Int, ConfigValue::Int(8080));

        let mut config = ConfigDict::new();
        schema.apply_defaults(&mut config);

        assert_eq!(config.get("host").unwrap().as_str(), Some("localhost"));
        assert_eq!(config.get("port").unwrap().as_int(), Some(8080));
    }
}
