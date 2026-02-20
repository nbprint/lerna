// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Configuration repository implementation
//!
//! This module provides a centralized interface for loading and managing
//! configuration sources, mirroring the Python IConfigRepository interface.

use std::collections::HashMap;

use crate::config::parser::ConfigLoadError;
use crate::config::source::{ConfigResult, ConfigSource, FileConfigSource};
use crate::config::value::{ConfigDict, ConfigValue};
use crate::ObjectType;

/// Search path element for config loading
#[derive(Clone, Debug)]
pub struct SearchPathElement {
    pub provider: String,
    pub path: String,
}

impl SearchPathElement {
    pub fn new(provider: &str, path: &str) -> Self {
        Self {
            provider: provider.to_string(),
            path: path.to_string(),
        }
    }
}

/// Get the scheme from a path (e.g., "file" from "file:///path")
pub fn get_scheme(path: &str) -> String {
    if let Some(idx) = path.find("://") {
        path[..idx].to_string()
    } else {
        "file".to_string()
    }
}

/// Configuration repository for managing and loading configs
pub struct ConfigRepository {
    sources: Vec<Box<dyn ConfigSource>>,
}

impl ConfigRepository {
    /// Create a new repository from search path elements
    pub fn new(search_paths: &[SearchPathElement]) -> Self {
        let sources = search_paths
            .iter()
            .map(|sp| Self::create_source(sp))
            .collect();

        Self { sources }
    }

    /// Create a config source from a search path element
    fn create_source(element: &SearchPathElement) -> Box<dyn ConfigSource> {
        let scheme = get_scheme(&element.path);
        match scheme.as_str() {
            "file" => Box::new(FileConfigSource::new(&element.provider, &element.path)),
            "pkg" => {
                // For pkg:// sources, we would need to resolve Python package paths
                // For now, skip or use file source
                Box::new(FileConfigSource::new(&element.provider, &element.path))
            }
            _ => Box::new(FileConfigSource::new(&element.provider, &element.path)),
        }
    }

    /// Get all sources
    pub fn get_sources(&self) -> &[Box<dyn ConfigSource>] {
        &self.sources
    }

    /// Load a config by path
    pub fn load_config(&self, config_path: &str) -> Result<Option<ConfigResult>, ConfigLoadError> {
        for source in &self.sources {
            if source.is_config(config_path) {
                let result = source.load_config(config_path)?;
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    /// Check if a group (directory) exists
    pub fn group_exists(&self, config_path: &str) -> bool {
        self.sources.iter().any(|s| s.is_group(config_path))
    }

    /// Check if a config file exists
    pub fn config_exists(&self, config_path: &str) -> bool {
        self.sources.iter().any(|s| s.is_config(config_path))
    }

    /// Get available options for a config group
    pub fn get_group_options(
        &self,
        group_name: &str,
        results_filter: Option<ObjectType>,
    ) -> Vec<String> {
        let mut options: Vec<String> = Vec::new();

        for source in &self.sources {
            if source.is_group(group_name) {
                let items = source.list(group_name, results_filter);
                options.extend(items);
            }
        }

        // Remove duplicates and sort
        options.sort();
        options.dedup();
        options
    }

    /// Find the source that contains a config or group
    pub fn find_source(
        &self,
        config_path: &str,
        object_type: ObjectType,
    ) -> Option<&dyn ConfigSource> {
        for source in &self.sources {
            match object_type {
                ObjectType::Config => {
                    if source.is_config(config_path) {
                        return Some(source.as_ref());
                    }
                }
                ObjectType::Group => {
                    if source.is_group(config_path) {
                        return Some(source.as_ref());
                    }
                }
                ObjectType::NotFound => {}
            }
        }
        None
    }
}

/// Caching wrapper for ConfigRepository
pub struct CachingConfigRepository {
    delegate: ConfigRepository,
    cache: HashMap<String, Option<ConfigResult>>,
}

impl CachingConfigRepository {
    pub fn new(delegate: ConfigRepository) -> Self {
        Self {
            delegate,
            cache: HashMap::new(),
        }
    }

    /// Load a config, using cache if available
    pub fn load_config(
        &mut self,
        config_path: &str,
    ) -> Result<Option<ConfigResult>, ConfigLoadError> {
        let cache_key = format!("config_path={}", config_path);

        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let result = self.delegate.load_config(config_path)?;
        self.cache.insert(cache_key, result.clone());
        Ok(result)
    }

    /// Check if a group exists
    pub fn group_exists(&self, config_path: &str) -> bool {
        self.delegate.group_exists(config_path)
    }

    /// Check if a config exists
    pub fn config_exists(&self, config_path: &str) -> bool {
        self.delegate.config_exists(config_path)
    }

    /// Get group options
    pub fn get_group_options(
        &self,
        group_name: &str,
        results_filter: Option<ObjectType>,
    ) -> Vec<String> {
        self.delegate.get_group_options(group_name, results_filter)
    }

    /// Get sources
    pub fn get_sources(&self) -> &[Box<dyn ConfigSource>] {
        self.delegate.get_sources()
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Load and compose a full configuration with defaults processing
    /// This uses the DefaultsListBuilder to properly process the defaults tree
    pub fn load_and_compose(
        &mut self,
        config_name: Option<&str>,
        overrides: &[String],
    ) -> Result<ComposedConfig, ConfigLoadError> {
        use crate::defaults_list::DefaultsListBuilder;

        // Build closures for the DefaultsListBuilder
        let repo = &self.delegate;

        // Config loader closure (not caching - DefaultsListBuilder handles tree building)
        let load_config = |config_path: &str| -> Result<ConfigDict, ConfigLoadError> {
            match repo.load_config(config_path)? {
                Some(result) => {
                    if let ConfigValue::Dict(dict) = result.config {
                        Ok(dict)
                    } else {
                        Ok(ConfigDict::new())
                    }
                }
                None => Err(ConfigLoadError::with_path("Config not found", config_path)),
            }
        };

        let config_exists = |config_path: &str| -> bool { repo.config_exists(config_path) };

        let group_exists = |config_path: &str| -> bool { repo.group_exists(config_path) };

        let overrides_vec: Vec<String> = overrides.to_vec();

        // Build the defaults list
        let builder =
            DefaultsListBuilder::new(load_config, config_exists, group_exists, &overrides_vec);

        let defaults_result = builder.build(config_name)?;

        // Compose the final config by merging all defaults
        let mut merged = ConfigDict::new();

        for result_default in &defaults_result.defaults {
            if result_default.is_self {
                continue; // _self_ is handled inline
            }

            if let Some(ref config_path) = result_default.config_path {
                match repo.load_config(config_path)? {
                    Some(config_result) => {
                        if let ConfigValue::Dict(ref dict) = config_result.config {
                            // Filter out the "defaults" key when merging
                            let mut filtered = ConfigDict::new();
                            for (k, v) in dict.iter() {
                                if k != "defaults" {
                                    filtered.insert(k.to_string(), v.clone());
                                }
                            }

                            merge_at_package(
                                &mut merged,
                                &filtered,
                                result_default.package.as_deref(),
                            );
                        }
                    }
                    None => {
                        // Config not found - this should have been caught earlier
                    }
                }
            }
        }

        // Apply config overrides (key.path=value style)
        for ovr in &defaults_result.config_overrides {
            apply_override_to_dict(&mut merged, ovr)?;
        }

        // Also apply overrides from the original list (for value overrides)
        for ovr in overrides {
            if ovr.contains('.') && !ovr.starts_with('+') && !ovr.starts_with('~') {
                apply_override_to_dict(&mut merged, ovr)?;
            }
        }

        Ok(ComposedConfig {
            config: merged,
            defaults_result,
        })
    }
}

/// Result of config composition
#[derive(Debug)]
pub struct ComposedConfig {
    /// The fully merged configuration
    pub config: ConfigDict,
    /// The defaults list result for debugging/inspection
    pub defaults_result: crate::defaults_list::DefaultsListResult,
}

/// Merge source dict at a package path into target
fn merge_at_package(target: &mut ConfigDict, source: &ConfigDict, package: Option<&str>) {
    use crate::config::value::merge_dicts;

    match package {
        None | Some("") | Some("_global_") => {
            merge_dicts(target, source);
        }
        Some(pkg) => {
            // Navigate/create nested path and merge there
            let parts: Vec<&str> = pkg.split('.').collect();
            merge_at_path(target, source, &parts);
        }
    }
}

/// Merge source at a nested path in target
fn merge_at_path(target: &mut ConfigDict, source: &ConfigDict, path: &[&str]) {
    use crate::config::value::merge_dicts;

    if path.is_empty() {
        merge_dicts(target, source);
        return;
    }

    let key = path[0];
    let remaining = &path[1..];

    // Ensure the nested dict exists
    if !target.contains_key(key) {
        target.insert(key.to_string(), ConfigValue::Dict(ConfigDict::new()));
    }

    if let Some(nested) = target.get_mut(key) {
        if let Some(nested_dict) = nested.as_dict_mut() {
            merge_at_path(nested_dict, source, remaining);
        }
    }
}

/// Apply a key=value override to a dict
fn apply_override_to_dict(
    config: &mut ConfigDict,
    override_str: &str,
) -> Result<(), ConfigLoadError> {
    if let Some(eq_pos) = override_str.find('=') {
        let key = &override_str[..eq_pos];
        let value_str = &override_str[eq_pos + 1..];

        // Handle deletion (starts with ~)
        if key.starts_with('~') {
            let actual_key = &key[1..];
            delete_at_path(config, actual_key);
            return Ok(());
        }

        // Handle addition (starts with +)
        let actual_key = if key.starts_with('+') { &key[1..] } else { key };

        // Parse the value
        let value = parse_override_value(value_str);

        // Set the value at the path
        set_at_path(config, actual_key, value);
    }

    Ok(())
}

/// Parse an override value string
fn parse_override_value(value_str: &str) -> ConfigValue {
    let trimmed = value_str.trim();

    // Boolean
    if trimmed == "true" {
        return ConfigValue::Bool(true);
    }
    if trimmed == "false" {
        return ConfigValue::Bool(false);
    }

    // Null
    if trimmed == "null" || trimmed == "~" {
        return ConfigValue::Null;
    }

    // Integer
    if let Ok(i) = trimmed.parse::<i64>() {
        return ConfigValue::Int(i);
    }

    // Float
    if let Ok(f) = trimmed.parse::<f64>() {
        return ConfigValue::Float(f);
    }

    // String (strip quotes if present)
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        return ConfigValue::String(trimmed[1..trimmed.len() - 1].to_string());
    }

    ConfigValue::String(trimmed.to_string())
}

/// Set a value at a dotted path
fn set_at_path(config: &mut ConfigDict, path: &str, value: ConfigValue) {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        config.insert(parts[0].to_string(), value);
        return;
    }

    // Navigate to the parent dict
    let key = parts[0];
    if !config.contains_key(key) {
        config.insert(key.to_string(), ConfigValue::Dict(ConfigDict::new()));
    }

    if let Some(nested) = config.get_mut(key) {
        if let Some(nested_dict) = nested.as_dict_mut() {
            let remaining = parts[1..].join(".");
            set_at_path(nested_dict, &remaining, value);
        }
    }
}

/// Delete a value at a dotted path
fn delete_at_path(config: &mut ConfigDict, path: &str) {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        config.remove(parts[0]);
        return;
    }

    // Navigate to the parent dict
    let key = parts[0];
    if let Some(nested) = config.get_mut(key) {
        if let Some(nested_dict) = nested.as_dict_mut() {
            let remaining = parts[1..].join(".");
            delete_at_path(nested_dict, &remaining);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_configs(temp_dir: &TempDir) {
        // Create config.yaml
        let mut file = fs::File::create(temp_dir.path().join("config.yaml")).unwrap();
        file.write_all(b"db:\n  host: localhost\n  port: 3306\n")
            .unwrap();

        // Create db/ group with mysql.yaml and postgres.yaml
        fs::create_dir(temp_dir.path().join("db")).unwrap();
        let mut mysql = fs::File::create(temp_dir.path().join("db/mysql.yaml")).unwrap();
        mysql.write_all(b"driver: mysql\nport: 3306\n").unwrap();

        let mut postgres = fs::File::create(temp_dir.path().join("db/postgres.yaml")).unwrap();
        postgres
            .write_all(b"driver: postgres\nport: 5432\n")
            .unwrap();
    }

    #[test]
    fn test_repository_load_config() {
        let temp_dir = TempDir::new().unwrap();
        setup_test_configs(&temp_dir);

        let search_path = vec![SearchPathElement::new(
            "main",
            temp_dir.path().to_str().unwrap(),
        )];
        let repo = ConfigRepository::new(&search_path);

        // Load main config
        let result = repo.load_config("config").unwrap();
        assert!(result.is_some());

        // Load group config
        let result = repo.load_config("db/mysql").unwrap();
        assert!(result.is_some());
        let config = result.unwrap().config;
        if let ConfigValue::Dict(dict) = config {
            assert_eq!(dict.get("driver").unwrap().as_str(), Some("mysql"));
        } else {
            panic!("Expected dict config");
        }
    }

    #[test]
    fn test_repository_group_exists() {
        let temp_dir = TempDir::new().unwrap();
        setup_test_configs(&temp_dir);

        let search_path = vec![SearchPathElement::new(
            "main",
            temp_dir.path().to_str().unwrap(),
        )];
        let repo = ConfigRepository::new(&search_path);

        assert!(repo.group_exists("db"));
        assert!(!repo.group_exists("server")); // doesn't exist
    }

    #[test]
    fn test_repository_config_exists() {
        let temp_dir = TempDir::new().unwrap();
        setup_test_configs(&temp_dir);

        let search_path = vec![SearchPathElement::new(
            "main",
            temp_dir.path().to_str().unwrap(),
        )];
        let repo = ConfigRepository::new(&search_path);

        assert!(repo.config_exists("config"));
        assert!(repo.config_exists("db/mysql"));
        assert!(!repo.config_exists("db/oracle")); // doesn't exist
    }

    #[test]
    fn test_repository_get_group_options() {
        let temp_dir = TempDir::new().unwrap();
        setup_test_configs(&temp_dir);

        let search_path = vec![SearchPathElement::new(
            "main",
            temp_dir.path().to_str().unwrap(),
        )];
        let repo = ConfigRepository::new(&search_path);

        let options = repo.get_group_options("db", Some(ObjectType::Config));
        assert!(options.contains(&"mysql".to_string()));
        assert!(options.contains(&"postgres".to_string()));
    }

    #[test]
    fn test_caching_repository() {
        let temp_dir = TempDir::new().unwrap();
        setup_test_configs(&temp_dir);

        let search_path = vec![SearchPathElement::new(
            "main",
            temp_dir.path().to_str().unwrap(),
        )];
        let repo = ConfigRepository::new(&search_path);
        let mut caching_repo = CachingConfigRepository::new(repo);

        // First load should cache
        let result1 = caching_repo.load_config("config").unwrap();
        assert!(result1.is_some());

        // Second load should use cache
        let result2 = caching_repo.load_config("config").unwrap();
        assert!(result2.is_some());

        // Clear cache
        caching_repo.clear_cache();
    }
}
