// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Configuration loader implementation

use std::collections::HashMap;

use crate::config::parser::ConfigLoadError;
use crate::config::source::{ConfigResult, ConfigSource, FileConfigSource};
use crate::config::value::{ConfigDict, ConfigValue};
use crate::ObjectType;

/// A search path entry
#[derive(Clone, Debug)]
pub struct SearchPathEntry {
    pub provider: String,
    pub path: String,
}

impl SearchPathEntry {
    pub fn new(provider: &str, path: &str) -> Self {
        Self {
            provider: provider.to_string(),
            path: path.to_string(),
        }
    }
}

/// Configuration loader that manages sources and loads configs
pub struct ConfigLoader {
    sources: Vec<Box<dyn ConfigSource>>,
}

impl ConfigLoader {
    /// Create a new config loader with the given search paths
    pub fn new(search_paths: Vec<SearchPathEntry>) -> Self {
        let mut sources: Vec<Box<dyn ConfigSource>> = Vec::new();

        for entry in search_paths {
            let scheme = get_scheme(&entry.path);
            match scheme.as_str() {
                "file" => {
                    sources.push(Box::new(FileConfigSource::new(
                        &entry.provider,
                        &entry.path,
                    )));
                }
                // Add more schemes as needed (pkg, etc.)
                _ => {
                    // Default to file source
                    sources.push(Box::new(FileConfigSource::new(
                        &entry.provider,
                        &entry.path,
                    )));
                }
            }
        }

        Self { sources }
    }

    /// Create a loader from a single config directory
    pub fn from_config_dir(config_dir: &str) -> Self {
        let search_paths = vec![SearchPathEntry::new(
            "main",
            &format!("file://{}", config_dir),
        )];
        Self::new(search_paths)
    }

    /// Load a configuration by name with optional overrides
    pub fn load_config(
        &self,
        config_name: Option<&str>,
        overrides: &[String],
    ) -> Result<ConfigValue, ConfigLoadError> {
        // Separate default overrides from value overrides
        // Default overrides are like "db=postgres" (group=config selection)
        // Value overrides are like "db.port=3307" (dotted path to value)
        let (default_overrides, value_overrides): (Vec<_>, Vec<_>) =
            overrides.iter().partition(|o| self.is_default_override(o));

        // Build default override map: group -> config name
        let default_override_map = self.build_default_override_map(&default_overrides);

        // Start with empty config
        let mut merged_config = ConfigDict::new();

        // Load and merge defaults if we have a config name
        if let Some(name) = config_name {
            let primary = self.load_single_config(name)?;

            // Process defaults first
            if let ConfigValue::Dict(dict) = &primary.config {
                if let Some(ConfigValue::List(defaults)) = dict.get("defaults") {
                    // Apply default overrides to defaults list
                    let modified_defaults =
                        self.apply_default_overrides(defaults, &default_override_map);
                    self.process_defaults(&modified_defaults, &mut merged_config)?;
                }

                // Merge the primary config (excluding defaults)
                for (key, value) in dict.iter() {
                    if key != "defaults" {
                        merged_config.insert(key.to_string(), value.clone());
                    }
                }
            }
        }

        // Apply value overrides
        for override_str in &value_overrides {
            self.apply_override(&mut merged_config, override_str)?;
        }

        Ok(ConfigValue::Dict(merged_config))
    }

    /// Check if an override is a default override (group=config) vs value override (key.path=value)
    fn is_default_override(&self, override_str: &str) -> bool {
        if let Some(eq_pos) = override_str.find('=') {
            let key = &override_str[..eq_pos];
            // Default override: no dots in key, not a special prefix, and the value is a valid config name
            !key.contains('.') && !key.starts_with('+') && !key.starts_with('~')
        } else {
            false
        }
    }

    /// Build a map of group -> config name from default overrides
    fn build_default_override_map(&self, overrides: &[&String]) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for o in overrides {
            if let Some(eq_pos) = o.find('=') {
                let group = o[..eq_pos].to_string();
                let config = o[eq_pos + 1..].to_string();
                map.insert(group, config);
            }
        }
        map
    }

    /// Apply default overrides to modify the defaults list
    fn apply_default_overrides(
        &self,
        defaults: &[ConfigValue],
        override_map: &HashMap<String, String>,
    ) -> Vec<ConfigValue> {
        defaults
            .iter()
            .map(|default| {
                match default {
                    ConfigValue::Dict(dict) => {
                        // Check if any group in this default has an override
                        let mut new_dict = ConfigDict::new();
                        for (group, value) in dict.iter() {
                            if let Some(new_config) = override_map.get(group) {
                                // Override the config selection
                                new_dict.insert(
                                    group.to_string(),
                                    ConfigValue::String(new_config.to_string()),
                                );
                            } else {
                                new_dict.insert(group.to_string(), value.clone());
                            }
                        }
                        ConfigValue::Dict(new_dict)
                    }
                    _ => default.clone(),
                }
            })
            .collect()
    }

    /// Process defaults list and merge configs
    fn process_defaults(
        &self,
        defaults: &[ConfigValue],
        merged_config: &mut ConfigDict,
    ) -> Result<(), ConfigLoadError> {
        for default in defaults {
            match default {
                // String default: just a config name
                ConfigValue::String(name) => {
                    if name != "_self_" {
                        let result = self.load_single_config(name)?;
                        if let ConfigValue::Dict(dict) = &result.config {
                            self.merge_config(merged_config, dict, None);
                        }
                    }
                }
                // Dict default: group/config selection
                ConfigValue::Dict(dict) => {
                    // Handle each key-value pair as group=config
                    for (group, value) in dict.iter() {
                        if group == "optional" {
                            continue; // Skip optional marker
                        }

                        let config_name = match value {
                            ConfigValue::String(s) => s.clone(),
                            ConfigValue::Null => continue, // null means skip
                            _ => continue,
                        };

                        // Load group/config
                        let config_path = format!("{}/{}", group, config_name);
                        match self.load_single_config(&config_path) {
                            Ok(result) => {
                                if let ConfigValue::Dict(cfg_dict) = &result.config {
                                    // Determine package (where to merge)
                                    let package = result
                                        .header
                                        .get("package")
                                        .map(|s| s.as_str())
                                        .unwrap_or(group);

                                    self.merge_config(merged_config, cfg_dict, Some(package));
                                }
                            }
                            Err(e) => {
                                // Check if optional
                                let is_optional = dict
                                    .get("optional")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);

                                if !is_optional {
                                    return Err(e);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Merge a config dict into the target, optionally at a package path
    fn merge_config(&self, target: &mut ConfigDict, source: &ConfigDict, package: Option<&str>) {
        if let Some(pkg) = package {
            if pkg == "_global_" || pkg.is_empty() {
                // Merge at root
                target.merge(source);
            } else {
                // Merge at package path
                let parts: Vec<&str> = pkg.split('.').collect();
                self.merge_at_path(target, source, &parts);
            }
        } else {
            target.merge(source);
        }
    }

    /// Merge source at a nested path in target
    fn merge_at_path(&self, target: &mut ConfigDict, source: &ConfigDict, path: &[&str]) {
        if path.is_empty() {
            target.merge(source);
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
                self.merge_at_path(nested_dict, source, remaining);
            }
        }
    }

    /// Load a single config file from the sources
    fn load_single_config(&self, config_path: &str) -> Result<ConfigResult, ConfigLoadError> {
        for source in &self.sources {
            if source.is_config(config_path) {
                return source.load_config(config_path);
            }
        }

        Err(ConfigLoadError::with_path(
            "Config not found in any source",
            config_path,
        ))
    }

    /// Apply a single override to the config
    fn apply_override(
        &self,
        config: &mut ConfigDict,
        override_str: &str,
    ) -> Result<(), ConfigLoadError> {
        // Parse override: key=value
        if let Some(eq_pos) = override_str.find('=') {
            let key = &override_str[..eq_pos];
            let value_str = &override_str[eq_pos + 1..];

            // Handle deletion (starts with ~)
            if key.starts_with('~') {
                let actual_key = &key[1..];
                self.delete_at_path(config, actual_key);
                return Ok(());
            }

            // Handle addition (starts with +)
            let (actual_key, is_add) = if key.starts_with('+') {
                (&key[1..], true)
            } else {
                (key, false)
            };

            // Parse the value
            let value = self.parse_override_value(value_str);

            // Set the value at the path
            self.set_at_path(config, actual_key, value, is_add);
        }

        Ok(())
    }

    /// Parse an override value string
    fn parse_override_value(&self, value_str: &str) -> ConfigValue {
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

        // List [a, b, c]
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let inner = &trimmed[1..trimmed.len() - 1];
            let items: Vec<ConfigValue> = inner
                .split(',')
                .map(|s| self.parse_override_value(s.trim()))
                .collect();
            return ConfigValue::List(items);
        }

        // String (strip quotes if present)
        let unquoted = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            &trimmed[1..trimmed.len() - 1]
        } else {
            trimmed
        };

        ConfigValue::from(unquoted)
    }

    /// Set a value at a dotted path
    fn set_at_path(&self, config: &mut ConfigDict, path: &str, value: ConfigValue, create: bool) {
        let parts: Vec<&str> = path.split('.').collect();
        self.set_at_path_parts(config, &parts, value, create);
    }

    fn set_at_path_parts(
        &self,
        config: &mut ConfigDict,
        parts: &[&str],
        value: ConfigValue,
        create: bool,
    ) {
        if parts.is_empty() {
            return;
        }

        if parts.len() == 1 {
            config.insert(parts[0].to_string(), value);
            return;
        }

        let key = parts[0];
        let remaining = &parts[1..];

        // Ensure nested dict exists
        if !config.contains_key(key) {
            if create {
                config.insert(key.to_string(), ConfigValue::Dict(ConfigDict::new()));
            } else {
                return;
            }
        }

        if let Some(nested) = config.get_mut(key) {
            if let Some(nested_dict) = nested.as_dict_mut() {
                self.set_at_path_parts(nested_dict, remaining, value, create);
            }
        }
    }

    /// Delete a value at a dotted path
    fn delete_at_path(&self, config: &mut ConfigDict, path: &str) {
        let parts: Vec<&str> = path.split('.').collect();
        self.delete_at_path_parts(config, &parts);
    }

    fn delete_at_path_parts(&self, config: &mut ConfigDict, parts: &[&str]) {
        if parts.is_empty() {
            return;
        }

        if parts.len() == 1 {
            config.remove(parts[0]);
            return;
        }

        let key = parts[0];
        let remaining = &parts[1..];

        if let Some(nested) = config.get_mut(key) {
            if let Some(nested_dict) = nested.as_dict_mut() {
                self.delete_at_path_parts(nested_dict, remaining);
            }
        }
    }

    /// Check if a config exists
    pub fn config_exists(&self, config_path: &str) -> bool {
        self.sources.iter().any(|s| s.is_config(config_path))
    }

    /// Check if a group exists
    pub fn group_exists(&self, group_path: &str) -> bool {
        self.sources.iter().any(|s| s.is_group(group_path))
    }

    /// List configs in a group
    pub fn list_group(&self, group_path: &str) -> Vec<String> {
        let mut items = Vec::new();
        for source in &self.sources {
            items.extend(source.list(group_path, Some(ObjectType::Config)));
        }
        items.sort();
        items.dedup();
        items
    }

    /// List groups in a path
    pub fn list_groups(&self, parent_path: &str) -> Vec<String> {
        let mut items = Vec::new();
        for source in &self.sources {
            items.extend(source.list(parent_path, Some(ObjectType::Group)));
        }
        items.sort();
        items.dedup();
        items
    }

    /// Get the sources
    pub fn sources(&self) -> &[Box<dyn ConfigSource>] {
        &self.sources
    }
}

/// Get the scheme from a path
fn get_scheme(path: &str) -> String {
    if let Some(idx) = path.find("://") {
        path[..idx].to_string()
    } else {
        "file".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_config_file(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_load_simple_config() {
        let temp_dir = TempDir::new().unwrap();
        create_config_file(
            temp_dir.path(),
            "config.yaml",
            "db:\n  host: localhost\n  port: 3306\n",
        );

        let loader = ConfigLoader::from_config_dir(temp_dir.path().to_str().unwrap());
        let config = loader.load_config(Some("config"), &[]).unwrap();

        let dict = config.as_dict().unwrap();
        let db = dict.get("db").unwrap().as_dict().unwrap();
        assert_eq!(db.get("host").unwrap().as_str(), Some("localhost"));
        assert_eq!(db.get("port").unwrap().as_int(), Some(3306));
    }

    #[test]
    fn test_load_with_override() {
        let temp_dir = TempDir::new().unwrap();
        create_config_file(
            temp_dir.path(),
            "config.yaml",
            "db:\n  host: localhost\n  port: 3306\n",
        );

        let loader = ConfigLoader::from_config_dir(temp_dir.path().to_str().unwrap());
        let config = loader
            .load_config(
                Some("config"),
                &["db.host=remotehost".to_string(), "db.port=5432".to_string()],
            )
            .unwrap();

        let dict = config.as_dict().unwrap();
        let db = dict.get("db").unwrap().as_dict().unwrap();
        assert_eq!(db.get("host").unwrap().as_str(), Some("remotehost"));
        assert_eq!(db.get("port").unwrap().as_int(), Some(5432));
    }

    #[test]
    fn test_load_with_defaults() {
        let temp_dir = TempDir::new().unwrap();

        // Create db config group
        create_config_file(
            temp_dir.path(),
            "db/mysql.yaml",
            "# @package db\ndriver: mysql\nport: 3306\n",
        );

        // Create main config with defaults
        create_config_file(
            temp_dir.path(),
            "config.yaml",
            "defaults:\n  - db: mysql\n\napp_name: myapp\n",
        );

        let loader = ConfigLoader::from_config_dir(temp_dir.path().to_str().unwrap());
        let config = loader.load_config(Some("config"), &[]).unwrap();

        let dict = config.as_dict().unwrap();
        assert_eq!(dict.get("app_name").unwrap().as_str(), Some("myapp"));

        let db = dict.get("db").unwrap().as_dict().unwrap();
        assert_eq!(db.get("driver").unwrap().as_str(), Some("mysql"));
        assert_eq!(db.get("port").unwrap().as_int(), Some(3306));
    }

    #[test]
    fn test_config_exists() {
        let temp_dir = TempDir::new().unwrap();
        create_config_file(temp_dir.path(), "config.yaml", "value: 1\n");

        let loader = ConfigLoader::from_config_dir(temp_dir.path().to_str().unwrap());

        assert!(loader.config_exists("config"));
        assert!(!loader.config_exists("nonexistent"));
    }

    #[test]
    fn test_list_group() {
        let temp_dir = TempDir::new().unwrap();
        create_config_file(temp_dir.path(), "db/mysql.yaml", "driver: mysql\n");
        create_config_file(temp_dir.path(), "db/postgres.yaml", "driver: postgres\n");

        let loader = ConfigLoader::from_config_dir(temp_dir.path().to_str().unwrap());
        let items = loader.list_group("db");

        assert!(items.contains(&"mysql".to_string()));
        assert!(items.contains(&"postgres".to_string()));
    }
}

/// A caching wrapper around ConfigLoader
/// Caches loaded configs to avoid re-reading from disk
pub struct CachingConfigLoader {
    loader: ConfigLoader,
    cache: std::cell::RefCell<HashMap<String, ConfigResult>>,
}

impl CachingConfigLoader {
    /// Create a new caching loader wrapping an existing loader
    pub fn new(loader: ConfigLoader) -> Self {
        Self {
            loader,
            cache: std::cell::RefCell::new(HashMap::new()),
        }
    }

    /// Create a caching loader from a config directory
    pub fn from_config_dir(config_dir: &str) -> Self {
        Self::new(ConfigLoader::from_config_dir(config_dir))
    }

    /// Load a single config with caching
    pub fn load_single_config(&self, config_path: &str) -> Result<ConfigResult, ConfigLoadError> {
        let cache_key = format!("config_path={}", config_path);

        // Check cache first
        if let Some(cached) = self.cache.borrow().get(&cache_key) {
            return Ok(cached.clone());
        }

        // Load and cache
        let result = self.loader.load_single_config(config_path)?;
        self.cache.borrow_mut().insert(cache_key, result.clone());
        Ok(result)
    }

    /// Load a full config with overrides (not cached since overrides differ)
    pub fn load_config(
        &self,
        config_name: Option<&str>,
        overrides: &[String],
    ) -> Result<ConfigValue, ConfigLoadError> {
        self.loader.load_config(config_name, overrides)
    }

    /// Check if a config exists
    pub fn config_exists(&self, config_path: &str) -> bool {
        self.loader.config_exists(config_path)
    }

    /// Check if a group exists
    pub fn group_exists(&self, group_path: &str) -> bool {
        self.loader.group_exists(group_path)
    }

    /// List configs in a group
    pub fn list_group(&self, group_path: &str) -> Vec<String> {
        self.loader.list_group(group_path)
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }

    /// Get cache statistics
    pub fn cache_size(&self) -> usize {
        self.cache.borrow().len()
    }
}
