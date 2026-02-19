// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Interpolation resolution engine
//!
//! Resolves interpolations like ${key}, ${oc.env:VAR}, ${oc.decode:...}

use crate::config::{ConfigDict, ConfigValue};
use std::collections::HashMap;
use std::env;

#[derive(Clone, Debug, PartialEq)]
pub enum InterpolationType {
    /// Simple key reference: ${key}
    Key(String),
    /// Nested key reference: ${parent.child}
    NestedKey(Vec<String>),
    /// Environment variable: ${oc.env:VAR} or ${oc.env:VAR,default}
    Env(String, Option<String>),
    /// Decode string: ${oc.decode:...}
    Decode(String),
    /// Create object: ${oc.create:...}
    Create(String),
    /// Selection: ${oc.select:key,dict}
    Select(String, String),
    /// Literal escape: $${...} -> ${...}
    EscapedLiteral(String),
    /// Not an interpolation
    Literal(String),
}

/// Parse an interpolation string into components
pub fn parse_interpolation(s: &str) -> Result<InterpolationType, String> {
    let s = s.trim();

    // Check for escaped literal: $${...} -> ${...}
    if s.starts_with("$${") && s.ends_with('}') {
        let inner = &s[3..s.len() - 1]; // Skip $${ and }
        return Ok(InterpolationType::EscapedLiteral(inner.to_string()));
    }

    // Must start with ${ and end with }
    if !s.starts_with("${") || !s.ends_with('}') {
        return Ok(InterpolationType::Literal(s.to_string()));
    }

    let inner = &s[2..s.len() - 1];

    // Check for oc.* resolvers
    if inner.starts_with("oc.env:") {
        let rest = &inner[7..];
        if let Some(comma_pos) = rest.find(',') {
            let var = rest[..comma_pos].trim().to_string();
            let default = rest[comma_pos + 1..].trim().to_string();
            return Ok(InterpolationType::Env(var, Some(default)));
        }
        return Ok(InterpolationType::Env(rest.trim().to_string(), None));
    }

    if inner.starts_with("oc.decode:") {
        return Ok(InterpolationType::Decode(inner[10..].to_string()));
    }

    if inner.starts_with("oc.create:") {
        return Ok(InterpolationType::Create(inner[10..].to_string()));
    }

    if inner.starts_with("oc.select:") {
        let rest = &inner[10..];
        if let Some(comma_pos) = rest.find(',') {
            let key = rest[..comma_pos].trim().to_string();
            let dict = rest[comma_pos + 1..].trim().to_string();
            return Ok(InterpolationType::Select(key, dict));
        }
        return Err(format!("Invalid oc.select syntax: {}", s));
    }

    // Check for nested key
    if inner.contains('.') {
        let parts: Vec<String> = inner.split('.').map(String::from).collect();
        return Ok(InterpolationType::NestedKey(parts));
    }

    // Simple key reference
    Ok(InterpolationType::Key(inner.to_string()))
}

/// Find all interpolations in a string
pub fn find_interpolations(s: &str) -> Vec<(usize, usize, String)> {
    let mut results = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Check for ${ or $${
        if chars[i] == '$' && i + 1 < len && chars[i + 1] == '{' {
            let start = i;
            let mut depth = 1;
            let mut j = i + 2;

            // Skip escaped literals
            if start > 0 && chars[start - 1] == '$' {
                i += 1;
                continue;
            }

            while j < len && depth > 0 {
                if chars[j] == '{' {
                    depth += 1;
                } else if chars[j] == '}' {
                    depth -= 1;
                }
                j += 1;
            }

            if depth == 0 {
                let interp: String = chars[start..j].iter().collect();
                results.push((start, j, interp));
                i = j;
                continue;
            }
        }
        i += 1;
    }

    results
}

/// Resolution context holding current config and environment
#[derive(Clone)]
pub struct ResolutionContext {
    /// The root config being resolved
    config: ConfigDict,
    /// Environment variable overrides for testing
    env_overrides: HashMap<String, String>,
}

impl ResolutionContext {
    pub fn new(config: ConfigDict) -> Self {
        Self {
            config,
            env_overrides: HashMap::new(),
        }
    }

    pub fn with_env_override(mut self, key: &str, value: &str) -> Self {
        self.env_overrides
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Resolve a key path in the config
    fn resolve_key(&self, key: &str) -> Option<ConfigValue> {
        let parts: Vec<&str> = key.split('.').collect();
        self.resolve_key_parts(&parts)
    }

    fn resolve_key_parts(&self, parts: &[&str]) -> Option<ConfigValue> {
        if parts.is_empty() {
            return None;
        }

        let mut current = ConfigValue::Dict(self.config.clone());

        for part in parts {
            match current {
                ConfigValue::Dict(ref dict) => {
                    current = dict.get(*part)?.clone();
                }
                _ => return None,
            }
        }

        Some(current)
    }

    /// Get environment variable
    fn get_env(&self, var: &str) -> Option<String> {
        if let Some(val) = self.env_overrides.get(var) {
            return Some(val.clone());
        }
        env::var(var).ok()
    }
}

/// Resolve a single interpolation
pub fn resolve_interpolation(
    interp: &InterpolationType,
    ctx: &ResolutionContext,
) -> Result<ConfigValue, String> {
    match interp {
        InterpolationType::Key(key) => ctx
            .resolve_key(key)
            .ok_or_else(|| format!("Key not found: {}", key)),
        InterpolationType::NestedKey(parts) => {
            let parts_ref: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
            ctx.resolve_key_parts(&parts_ref)
                .ok_or_else(|| format!("Key not found: {}", parts.join(".")))
        }
        InterpolationType::Env(var, default) => match ctx.get_env(var) {
            Some(val) => Ok(ConfigValue::String(val)),
            None => match default {
                Some(d) => Ok(ConfigValue::String(d.clone())),
                None => Err(format!("Environment variable not found: {}", var)),
            },
        },
        InterpolationType::Decode(expr) => {
            // Simple decode - just return as string for now
            // Full decode would need expression parsing
            Ok(ConfigValue::String(expr.clone()))
        }
        InterpolationType::Create(_) => Err("oc.create requires runtime instantiation".to_string()),
        InterpolationType::Select(key, _dict_ref) => {
            // Need to resolve the dict reference first
            Err(format!("oc.select requires full resolution: {}", key))
        }
        InterpolationType::EscapedLiteral(inner) => {
            Ok(ConfigValue::String(format!("${{{}}}", inner)))
        }
        InterpolationType::Literal(s) => Ok(ConfigValue::String(s.clone())),
    }
}

/// Resolve all interpolations in a string
pub fn resolve_string(s: &str, ctx: &ResolutionContext) -> Result<String, String> {
    let interpolations = find_interpolations(s);

    if interpolations.is_empty() {
        return Ok(s.to_string());
    }

    let mut result = s.to_string();

    // Process in reverse order to maintain positions
    for (start, end, interp_str) in interpolations.into_iter().rev() {
        let interp_type = parse_interpolation(&interp_str)?;
        let resolved = resolve_interpolation(&interp_type, ctx)?;

        let replacement = match resolved {
            ConfigValue::String(s) => s,
            ConfigValue::Int(i) => i.to_string(),
            ConfigValue::Float(f) => f.to_string(),
            ConfigValue::Bool(b) => b.to_string(),
            _ => return Err(format!("Cannot interpolate complex type: {:?}", resolved)),
        };

        result.replace_range(start..end, &replacement);
    }

    Ok(result)
}

/// Resolve all interpolations in a config value recursively
pub fn resolve_value(value: ConfigValue, ctx: &ResolutionContext) -> Result<ConfigValue, String> {
    match value {
        ConfigValue::String(s) => {
            // Check if entire string is a single interpolation
            let s_trimmed = s.trim();
            if s_trimmed.starts_with("${")
                && s_trimmed.ends_with('}')
                && find_interpolations(&s).len() == 1
            {
                // Return the actual type from resolution
                let interp_type = parse_interpolation(&s)?;
                return resolve_interpolation(&interp_type, ctx);
            }
            // Otherwise resolve as string
            Ok(ConfigValue::String(resolve_string(&s, ctx)?))
        }
        ConfigValue::List(items) => {
            let resolved: Result<Vec<ConfigValue>, String> =
                items.into_iter().map(|v| resolve_value(v, ctx)).collect();
            Ok(ConfigValue::List(resolved?))
        }
        ConfigValue::Dict(dict) => {
            let mut resolved = ConfigDict::new();
            for (key, val) in dict.iter() {
                resolved.insert(key.to_string(), resolve_value(val.clone(), ctx)?);
            }
            Ok(ConfigValue::Dict(resolved))
        }
        // Primitives pass through unchanged
        other => Ok(other),
    }
}

/// Fully resolve a config dictionary
pub fn resolve_config(config: ConfigDict) -> Result<ConfigDict, String> {
    let ctx = ResolutionContext::new(config.clone());

    match resolve_value(ConfigValue::Dict(config), &ctx)? {
        ConfigValue::Dict(d) => Ok(d),
        _ => Err("Config resolution did not return a dict".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_interpolation_simple() {
        let result = parse_interpolation("${key}").unwrap();
        assert_eq!(result, InterpolationType::Key("key".to_string()));
    }

    #[test]
    fn test_parse_interpolation_nested() {
        let result = parse_interpolation("${db.host}").unwrap();
        assert_eq!(
            result,
            InterpolationType::NestedKey(vec!["db".to_string(), "host".to_string()])
        );
    }

    #[test]
    fn test_parse_interpolation_env() {
        let result = parse_interpolation("${oc.env:HOME}").unwrap();
        assert_eq!(result, InterpolationType::Env("HOME".to_string(), None));
    }

    #[test]
    fn test_parse_interpolation_env_default() {
        let result = parse_interpolation("${oc.env:MISSING,default_val}").unwrap();
        assert_eq!(
            result,
            InterpolationType::Env("MISSING".to_string(), Some("default_val".to_string()))
        );
    }

    #[test]
    fn test_find_interpolations() {
        let s = "host=${db.host}, port=${db.port}";
        let interps = find_interpolations(s);
        assert_eq!(interps.len(), 2);
        assert_eq!(interps[0].2, "${db.host}");
        assert_eq!(interps[1].2, "${db.port}");
    }

    #[test]
    fn test_resolve_simple_key() {
        let mut config = ConfigDict::new();
        config.insert("name".to_string(), ConfigValue::String("test".to_string()));

        let ctx = ResolutionContext::new(config);
        let interp = InterpolationType::Key("name".to_string());
        let result = resolve_interpolation(&interp, &ctx).unwrap();

        assert_eq!(result, ConfigValue::String("test".to_string()));
    }

    #[test]
    fn test_resolve_nested_key() {
        let mut db = ConfigDict::new();
        db.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );

        let mut config = ConfigDict::new();
        config.insert("db".to_string(), ConfigValue::Dict(db));

        let ctx = ResolutionContext::new(config);
        let interp = InterpolationType::NestedKey(vec!["db".to_string(), "host".to_string()]);
        let result = resolve_interpolation(&interp, &ctx).unwrap();

        assert_eq!(result, ConfigValue::String("localhost".to_string()));
    }

    #[test]
    fn test_resolve_string() {
        let mut config = ConfigDict::new();
        config.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );
        config.insert("port".to_string(), ConfigValue::Int(3306));

        let ctx = ResolutionContext::new(config);
        let result = resolve_string("mysql://${host}:${port}", &ctx).unwrap();

        assert_eq!(result, "mysql://localhost:3306");
    }

    #[test]
    fn test_resolve_env() {
        let config = ConfigDict::new();
        let ctx = ResolutionContext::new(config).with_env_override("TEST_VAR", "test_value");

        let interp = InterpolationType::Env("TEST_VAR".to_string(), None);
        let result = resolve_interpolation(&interp, &ctx).unwrap();

        assert_eq!(result, ConfigValue::String("test_value".to_string()));
    }

    #[test]
    fn test_resolve_env_default() {
        let config = ConfigDict::new();
        let ctx = ResolutionContext::new(config);

        let interp = InterpolationType::Env(
            "NONEXISTENT_VAR_12345".to_string(),
            Some("default".to_string()),
        );
        let result = resolve_interpolation(&interp, &ctx).unwrap();

        assert_eq!(result, ConfigValue::String("default".to_string()));
    }

    #[test]
    fn test_resolve_config() {
        let mut db = ConfigDict::new();
        db.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );
        db.insert("port".to_string(), ConfigValue::Int(3306));

        let mut config = ConfigDict::new();
        config.insert("db".to_string(), ConfigValue::Dict(db));
        config.insert(
            "url".to_string(),
            ConfigValue::String("mysql://${db.host}:${db.port}".to_string()),
        );

        let resolved = resolve_config(config).unwrap();

        assert_eq!(
            resolved.get("url").unwrap(),
            &ConfigValue::String("mysql://localhost:3306".to_string())
        );
    }

    #[test]
    fn test_escaped_literal() {
        let result = parse_interpolation("$${escaped}").unwrap();
        assert_eq!(
            result,
            InterpolationType::EscapedLiteral("escaped".to_string())
        );
    }
}
