// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Configuration source interface and implementations

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::config::parser::{extract_header, load_yaml_file, ConfigLoadError};
use crate::config::value::ConfigValue;
use crate::ObjectType;

/// Result of loading a config file
#[derive(Clone, Debug)]
pub struct ConfigResult {
    /// Provider name (e.g., "file", "pkg")
    pub provider: String,
    /// Path to the config
    pub path: String,
    /// The loaded configuration
    pub config: ConfigValue,
    /// Header information (e.g., @package)
    pub header: HashMap<String, String>,
    /// Whether this is from the schema source
    pub is_schema_source: bool,
}

/// Trait for configuration sources
pub trait ConfigSource: Send + Sync {
    /// Get the scheme for this source (e.g., "file", "pkg")
    fn scheme(&self) -> &str;

    /// Get the provider name
    fn provider(&self) -> &str;

    /// Get the base path
    fn path(&self) -> &str;

    /// Check if this source is available
    fn available(&self) -> bool;

    /// Load a config by path
    fn load_config(&self, config_path: &str) -> Result<ConfigResult, ConfigLoadError>;

    /// Check if a path is a group (directory)
    fn is_group(&self, config_path: &str) -> bool;

    /// Check if a path is a config file
    fn is_config(&self, config_path: &str) -> bool;

    /// Check if a path exists
    fn exists(&self, config_path: &str) -> bool {
        self.is_group(config_path) || self.is_config(config_path)
    }

    /// List items in a config path
    fn list(&self, config_path: &str, results_filter: Option<ObjectType>) -> Vec<String>;
}

/// File-based configuration source
pub struct FileConfigSource {
    provider_name: String,
    base_path: PathBuf,
}

impl FileConfigSource {
    pub fn new(provider: &str, path: &str) -> Self {
        // Strip scheme if present
        let clean_path = if let Some(idx) = path.find("://") {
            &path[idx + 3..]
        } else {
            path
        };

        Self {
            provider_name: provider.to_string(),
            base_path: PathBuf::from(clean_path),
        }
    }

    fn normalize_config_path(&self, config_path: &str) -> PathBuf {
        let mut path = config_path.to_string();

        // Add .yaml extension if missing
        if !path.ends_with(".yaml") && !path.ends_with(".yml") {
            path.push_str(".yaml");
        }

        self.base_path.join(&path)
    }

    fn full_path(&self, config_path: &str) -> PathBuf {
        self.base_path.join(config_path)
    }
}

impl ConfigSource for FileConfigSource {
    fn scheme(&self) -> &str {
        "file"
    }

    fn provider(&self) -> &str {
        &self.provider_name
    }

    fn path(&self) -> &str {
        self.base_path.to_str().unwrap_or("")
    }

    fn available(&self) -> bool {
        self.is_group("")
    }

    fn load_config(&self, config_path: &str) -> Result<ConfigResult, ConfigLoadError> {
        let full_path = self.normalize_config_path(config_path);

        // Read header first
        let content = fs::read_to_string(&full_path).map_err(|e| {
            ConfigLoadError::with_path(
                format!("Failed to read: {}", e),
                full_path.to_string_lossy(),
            )
        })?;

        let header = extract_header(&content);
        let config = load_yaml_file(&full_path)?;

        Ok(ConfigResult {
            provider: self.provider_name.clone(),
            path: format!("{}://{}", self.scheme(), self.base_path.display()),
            config,
            header,
            is_schema_source: false,
        })
    }

    fn is_group(&self, config_path: &str) -> bool {
        let full_path = self.full_path(config_path);
        full_path.is_dir()
    }

    fn is_config(&self, config_path: &str) -> bool {
        let full_path = self.normalize_config_path(config_path);
        full_path.is_file()
    }

    fn list(&self, config_path: &str, results_filter: Option<ObjectType>) -> Vec<String> {
        let full_path = self.full_path(config_path);
        let mut items = Vec::new();

        if let Ok(entries) = fs::read_dir(&full_path) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();

                // Skip pycache and __init__.py
                if file_name == "__pycache__" || file_name == "__init__.py" {
                    continue;
                }

                let _file_path = if config_path.is_empty() {
                    file_name.clone()
                } else {
                    format!("{}/{}", config_path, file_name)
                };
                let is_group = entry.path().is_dir();
                let is_config = entry.path().is_file()
                    && (file_name.ends_with(".yaml") || file_name.ends_with(".yml"));

                let include = match results_filter {
                    None => is_group || is_config,
                    Some(ObjectType::Group) => is_group,
                    Some(ObjectType::Config) => is_config,
                    Some(ObjectType::NotFound) => false,
                };

                if include {
                    // Remove .yaml extension for config files
                    let name = if is_config && !is_group {
                        file_name
                            .trim_end_matches(".yaml")
                            .trim_end_matches(".yml")
                            .to_string()
                    } else {
                        file_name
                    };
                    items.push(name);
                }
            }
        }

        items.sort();
        items.dedup();
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_config(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_file_source_load() {
        let temp_dir = TempDir::new().unwrap();
        create_test_config(
            temp_dir.path(),
            "config.yaml",
            "db:\n  host: localhost\n  port: 3306\n",
        );

        let source = FileConfigSource::new("test", temp_dir.path().to_str().unwrap());

        assert!(source.available());
        assert!(source.is_config("config"));

        let result = source.load_config("config").unwrap();
        let dict = result.config.as_dict().unwrap();
        let db = dict.get("db").unwrap().as_dict().unwrap();
        assert_eq!(db.get("host").unwrap().as_str(), Some("localhost"));
    }

    #[test]
    fn test_file_source_list() {
        let temp_dir = TempDir::new().unwrap();
        create_test_config(temp_dir.path(), "a.yaml", "value: 1\n");
        create_test_config(temp_dir.path(), "b.yaml", "value: 2\n");
        fs::create_dir(temp_dir.path().join("group")).unwrap();

        let source = FileConfigSource::new("test", temp_dir.path().to_str().unwrap());

        let all = source.list("", None);
        assert!(all.contains(&"a".to_string()));
        assert!(all.contains(&"b".to_string()));
        assert!(all.contains(&"group".to_string()));

        let configs = source.list("", Some(ObjectType::Config));
        assert!(configs.contains(&"a".to_string()));
        assert!(!configs.contains(&"group".to_string()));

        let groups = source.list("", Some(ObjectType::Group));
        assert!(groups.contains(&"group".to_string()));
        assert!(!groups.contains(&"a".to_string()));
    }
}
