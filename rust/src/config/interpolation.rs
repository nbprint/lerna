// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Interpolation resolution for config values
//!
//! Resolves ${...} references in configuration values.

use std::collections::HashMap;
use std::env;

use crate::config::value::{ConfigDict, ConfigValue};

/// Error during interpolation resolution
#[derive(Debug, Clone)]
pub struct InterpolationError {
    pub message: String,
    pub key: Option<String>,
}

impl InterpolationError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            key: None,
        }
    }

    pub fn with_key(message: &str, key: &str) -> Self {
        Self {
            message: message.to_string(),
            key: Some(key.to_string()),
        }
    }
}

impl std::fmt::Display for InterpolationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(key) = &self.key {
            write!(f, "Interpolation error at '{}': {}", key, self.message)
        } else {
            write!(f, "Interpolation error: {}", self.message)
        }
    }
}

impl std::error::Error for InterpolationError {}

/// Resolver context for interpolation
pub struct ResolverContext<'a> {
    /// The root config for resolving references
    pub root: &'a ConfigDict,
    /// Custom resolvers (e.g., "oc.env" -> resolver function)
    pub resolvers: HashMap<String, Box<dyn Fn(&[&str]) -> Result<ConfigValue, InterpolationError>>>,
    /// Maximum recursion depth
    pub max_depth: usize,
}

impl<'a> ResolverContext<'a> {
    pub fn new(root: &'a ConfigDict) -> Self {
        let mut ctx = Self {
            root,
            resolvers: HashMap::new(),
            max_depth: 10,
        };
        ctx.register_default_resolvers();
        ctx
    }

    fn register_default_resolvers(&mut self) {
        // oc.env resolver: ${oc.env:VAR_NAME} or ${oc.env:VAR_NAME,default}
        self.resolvers.insert(
            "oc.env".to_string(),
            Box::new(|args: &[&str]| {
                if args.is_empty() {
                    return Err(InterpolationError::new(
                        "oc.env requires at least one argument",
                    ));
                }
                let var_name = args[0];
                match env::var(var_name) {
                    Ok(value) => Ok(ConfigValue::String(value)),
                    Err(_) => {
                        if args.len() > 1 {
                            // Use default value
                            Ok(ConfigValue::String(args[1].to_string()))
                        } else {
                            Err(InterpolationError::new(&format!(
                                "Environment variable '{}' not found",
                                var_name
                            )))
                        }
                    }
                }
            }),
        );

        // oc.decode resolver: ${oc.decode:string_value} - Converts string to its typed representation
        self.resolvers.insert(
            "oc.decode".to_string(),
            Box::new(|args: &[&str]| {
                if args.is_empty() {
                    return Err(InterpolationError::new("oc.decode requires an argument"));
                }
                let value = args[0].trim();
                // Try to parse as different types
                if value == "null" || value == "~" {
                    return Ok(ConfigValue::Null);
                }
                if value == "true" {
                    return Ok(ConfigValue::Bool(true));
                }
                if value == "false" {
                    return Ok(ConfigValue::Bool(false));
                }
                if let Ok(i) = value.parse::<i64>() {
                    return Ok(ConfigValue::Int(i));
                }
                if let Ok(f) = value.parse::<f64>() {
                    return Ok(ConfigValue::Float(f));
                }
                // Default to string
                Ok(ConfigValue::String(value.to_string()))
            }),
        );

        // oc.mandatory resolver: ${oc.mandatory:key,message} - Throws if value is missing
        self.resolvers.insert(
            "oc.mandatory".to_string(),
            Box::new(|args: &[&str]| {
                // This resolver is a placeholder - actual implementation
                // requires context about the value being checked
                if args.is_empty() {
                    return Err(InterpolationError::new(
                        "oc.mandatory requires at least one argument",
                    ));
                }
                // The first argument should be the key to check
                // In a real implementation, we'd need to check if that key exists
                Err(InterpolationError::new(&format!(
                    "Mandatory value {} is missing",
                    args[0]
                )))
            }),
        );
    }
}

/// Resolve all interpolations in a config value
pub fn resolve(
    value: &ConfigValue,
    ctx: &ResolverContext,
) -> Result<ConfigValue, InterpolationError> {
    resolve_with_depth(value, ctx, 0)
}

fn resolve_with_depth(
    value: &ConfigValue,
    ctx: &ResolverContext,
    depth: usize,
) -> Result<ConfigValue, InterpolationError> {
    if depth > ctx.max_depth {
        return Err(InterpolationError::new(
            "Maximum interpolation depth exceeded",
        ));
    }

    match value {
        ConfigValue::Interpolation(expr) => {
            // Handle both "${expr}" and "expr" formats
            let inner_expr = if expr.starts_with("${") && expr.ends_with("}") {
                &expr[2..expr.len() - 1]
            } else {
                expr.as_str()
            };

            // Check if this is a multi-interpolation string (e.g., "jdbc:${db.driver}://...")
            if inner_expr.contains("${") {
                // This is actually a string with multiple interpolations
                return resolve_string_interpolations(expr, ctx, depth);
            }

            let resolved = resolve_interpolation(inner_expr, ctx, depth)?;
            // Recursively resolve in case the result contains more interpolations
            resolve_with_depth(&resolved, ctx, depth + 1)
        }
        ConfigValue::String(s) => {
            // Check if string contains interpolation markers
            if s.contains("${") {
                resolve_string_interpolations(s, ctx, depth)
            } else {
                Ok(value.clone())
            }
        }
        ConfigValue::Dict(dict) => {
            let mut new_dict = ConfigDict::new();
            for (k, v) in dict.iter() {
                new_dict.insert(k.to_string(), resolve_with_depth(v, ctx, depth)?);
            }
            Ok(ConfigValue::Dict(new_dict))
        }
        ConfigValue::List(list) => {
            let new_list: Result<Vec<_>, _> = list
                .iter()
                .map(|v| resolve_with_depth(v, ctx, depth))
                .collect();
            Ok(ConfigValue::List(new_list?))
        }
        // Other values pass through unchanged
        _ => Ok(value.clone()),
    }
}

/// Resolve a single interpolation expression
fn resolve_interpolation(
    expr: &str,
    ctx: &ResolverContext,
    depth: usize,
) -> Result<ConfigValue, InterpolationError> {
    // Check for resolver syntax: resolver_name:arg1,arg2,...
    if let Some(colon_pos) = expr.find(':') {
        let resolver_name = &expr[..colon_pos];
        let args_str = &expr[colon_pos + 1..];

        // Split args by comma (simple split, doesn't handle nested commas)
        let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).collect();

        if let Some(resolver) = ctx.resolvers.get(resolver_name) {
            return resolver(&args);
        }
        // If no resolver found, try as a path lookup
    }

    // Simple path lookup: ${db.host}
    let value = lookup_path(expr, ctx.root)?;

    // Recursively resolve the looked-up value in case it contains interpolations
    resolve_with_depth(&value, ctx, depth + 1)
}

/// Resolve interpolations embedded in a string
fn resolve_string_interpolations(
    s: &str,
    ctx: &ResolverContext,
    _depth: usize,
) -> Result<ConfigValue, InterpolationError> {
    // Check if the string is exactly a single interpolation: ${...}
    let trimmed = s.trim();
    if trimmed.starts_with("${") && trimmed.ends_with("}") {
        // Count braces to see if it's a single interpolation
        let inner = &trimmed[2..trimmed.len() - 1];
        let mut brace_count = 0;
        let mut is_single = true;
        for c in inner.chars() {
            if c == '{' {
                brace_count += 1;
            } else if c == '}' {
                if brace_count == 0 {
                    // Found closing brace before end - not a single interpolation
                    is_single = false;
                    break;
                }
                brace_count -= 1;
            }
        }
        if is_single && brace_count == 0 {
            // It's a single interpolation, preserve the type
            return resolve_interpolation(inner, ctx, 0);
        }
    }

    // Multiple interpolations or mixed content - build a string
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'

            // Extract interpolation expression
            let mut expr = String::new();
            let mut brace_depth = 1;

            while let Some(c) = chars.next() {
                if c == '{' {
                    brace_depth += 1;
                    expr.push(c);
                } else if c == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        break;
                    }
                    expr.push(c);
                } else {
                    expr.push(c);
                }
            }

            let resolved = resolve_interpolation(&expr, ctx, 0)?;

            match &resolved {
                ConfigValue::String(s) => result.push_str(s),
                ConfigValue::Int(i) => result.push_str(&i.to_string()),
                ConfigValue::Float(f) => result.push_str(&f.to_string()),
                ConfigValue::Bool(b) => result.push_str(&b.to_string()),
                ConfigValue::Null => result.push_str("null"),
                _ => result.push_str(&format!("{:?}", resolved)),
            }
        } else {
            result.push(c);
        }
    }

    Ok(ConfigValue::String(result))
}

/// Lookup a dotted path in the config
fn lookup_path(path: &str, root: &ConfigDict) -> Result<ConfigValue, InterpolationError> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = ConfigValue::Dict(root.clone());

    for part in parts {
        match current {
            ConfigValue::Dict(dict) => match dict.get(part) {
                Some(value) => current = value.clone(),
                None => {
                    return Err(InterpolationError::with_key(
                        &format!("Key '{}' not found", part),
                        path,
                    ));
                }
            },
            _ => {
                return Err(InterpolationError::with_key(
                    "Cannot traverse non-dict value",
                    path,
                ));
            }
        }
    }

    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> ConfigDict {
        let mut db = ConfigDict::new();
        db.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );
        db.insert("port".to_string(), ConfigValue::Int(3306));

        let mut root = ConfigDict::new();
        root.insert("db".to_string(), ConfigValue::Dict(db));
        root.insert("name".to_string(), ConfigValue::String("myapp".to_string()));
        root
    }

    #[test]
    fn test_simple_lookup() {
        let root = make_config();
        let ctx = ResolverContext::new(&root);

        let result = resolve_interpolation("db.host", &ctx, 0).unwrap();
        assert_eq!(result, ConfigValue::String("localhost".to_string()));

        let result = resolve_interpolation("db.port", &ctx, 0).unwrap();
        assert_eq!(result, ConfigValue::Int(3306));
    }

    #[test]
    fn test_string_interpolation() {
        let root = make_config();
        let ctx = ResolverContext::new(&root);

        let value = ConfigValue::String("host is ${db.host}".to_string());
        let result = resolve(&value, &ctx).unwrap();
        assert_eq!(result, ConfigValue::String("host is localhost".to_string()));
    }

    #[test]
    fn test_full_string_interpolation() {
        let root = make_config();
        let ctx = ResolverContext::new(&root);

        // When the entire string is an interpolation, preserve the type
        let value = ConfigValue::String("${db.port}".to_string());
        let result = resolve(&value, &ctx).unwrap();
        assert_eq!(result, ConfigValue::Int(3306));
    }

    #[test]
    fn test_interpolation_value() {
        let root = make_config();
        let ctx = ResolverContext::new(&root);

        let value = ConfigValue::Interpolation("name".to_string());
        let result = resolve(&value, &ctx).unwrap();
        assert_eq!(result, ConfigValue::String("myapp".to_string()));
    }

    #[test]
    fn test_env_resolver() {
        env::set_var("TEST_VAR_12345", "test_value");

        let root = ConfigDict::new();
        let ctx = ResolverContext::new(&root);

        let result = resolve_interpolation("oc.env:TEST_VAR_12345", &ctx, 0).unwrap();
        assert_eq!(result, ConfigValue::String("test_value".to_string()));

        env::remove_var("TEST_VAR_12345");
    }

    #[test]
    fn test_env_resolver_with_default() {
        let root = ConfigDict::new();
        let ctx = ResolverContext::new(&root);

        let result =
            resolve_interpolation("oc.env:NONEXISTENT_VAR_12345,default_val", &ctx, 0).unwrap();
        assert_eq!(result, ConfigValue::String("default_val".to_string()));
    }

    #[test]
    fn test_dict_resolution() {
        let mut inner = ConfigDict::new();
        inner.insert(
            "ref".to_string(),
            ConfigValue::Interpolation("name".to_string()),
        );

        let mut root = ConfigDict::new();
        root.insert("name".to_string(), ConfigValue::String("value".to_string()));
        root.insert("inner".to_string(), ConfigValue::Dict(inner));

        let ctx = ResolverContext::new(&root);
        let result = resolve(&ConfigValue::Dict(root.clone()), &ctx).unwrap();

        if let ConfigValue::Dict(dict) = result {
            if let Some(ConfigValue::Dict(inner)) = dict.get("inner") {
                assert_eq!(
                    inner.get("ref"),
                    Some(&ConfigValue::String("value".to_string()))
                );
            } else {
                panic!("Expected inner dict");
            }
        } else {
            panic!("Expected dict");
        }
    }

    #[test]
    fn test_missing_key_error() {
        let root = make_config();
        let ctx = ResolverContext::new(&root);

        let result = resolve_interpolation("nonexistent.key", &ctx, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_resolver_bool() {
        let root = ConfigDict::new();
        let ctx = ResolverContext::new(&root);

        let result = resolve_interpolation("oc.decode:true", &ctx, 0).unwrap();
        assert_eq!(result, ConfigValue::Bool(true));

        let result = resolve_interpolation("oc.decode:false", &ctx, 0).unwrap();
        assert_eq!(result, ConfigValue::Bool(false));
    }

    #[test]
    fn test_decode_resolver_int() {
        let root = ConfigDict::new();
        let ctx = ResolverContext::new(&root);

        let result = resolve_interpolation("oc.decode:42", &ctx, 0).unwrap();
        assert_eq!(result, ConfigValue::Int(42));

        let result = resolve_interpolation("oc.decode:-123", &ctx, 0).unwrap();
        assert_eq!(result, ConfigValue::Int(-123));
    }

    #[test]
    fn test_decode_resolver_float() {
        let root = ConfigDict::new();
        let ctx = ResolverContext::new(&root);

        let result = resolve_interpolation("oc.decode:3.14", &ctx, 0).unwrap();
        assert_eq!(result, ConfigValue::Float(3.14));
    }

    #[test]
    fn test_decode_resolver_null() {
        let root = ConfigDict::new();
        let ctx = ResolverContext::new(&root);

        let result = resolve_interpolation("oc.decode:null", &ctx, 0).unwrap();
        assert_eq!(result, ConfigValue::Null);
    }

    #[test]
    fn test_decode_resolver_string() {
        let root = ConfigDict::new();
        let ctx = ResolverContext::new(&root);

        let result = resolve_interpolation("oc.decode:hello", &ctx, 0).unwrap();
        assert_eq!(result, ConfigValue::String("hello".to_string()));
    }
}
