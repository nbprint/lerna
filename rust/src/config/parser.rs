// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! YAML configuration parser

use std::fs;
use std::path::Path;

use crate::config::value::{ConfigDict, ConfigValue};

/// Error type for config loading
#[derive(Debug, Clone)]
pub struct ConfigLoadError {
    pub message: String,
    pub path: Option<String>,
}

impl ConfigLoadError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: None,
        }
    }

    pub fn with_path(message: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: Some(path.into()),
        }
    }
}

impl std::fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "{}: {}", path, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ConfigLoadError {}

/// Parse a YAML string into a ConfigValue
pub fn parse_yaml(content: &str) -> Result<ConfigValue, ConfigLoadError> {
    // Use serde_yaml for parsing
    let yaml: serde_yaml::Value = serde_yaml::from_str(content)
        .map_err(|e| ConfigLoadError::new(format!("YAML parse error: {}", e)))?;

    Ok(yaml_to_config_value(&yaml))
}

/// Load a YAML file and parse it
pub fn load_yaml_file(path: &Path) -> Result<ConfigValue, ConfigLoadError> {
    let path_str = path.to_string_lossy().to_string();

    if !path.exists() {
        return Err(ConfigLoadError::with_path("Config not found", &path_str));
    }

    let content = fs::read_to_string(path).map_err(|e| {
        ConfigLoadError::with_path(format!("Failed to read file: {}", e), &path_str)
    })?;

    parse_yaml(&content).map_err(|mut e| {
        e.path = Some(path_str);
        e
    })
}

/// Convert serde_yaml::Value to ConfigValue
fn yaml_to_config_value(yaml: &serde_yaml::Value) -> ConfigValue {
    match yaml {
        serde_yaml::Value::Null => ConfigValue::Null,
        serde_yaml::Value::Bool(b) => ConfigValue::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ConfigValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                ConfigValue::Float(f)
            } else {
                ConfigValue::Null
            }
        }
        serde_yaml::Value::String(s) => {
            // Handle special values
            if s == "???" {
                ConfigValue::Missing
            } else if s.contains("${") && s.contains('}') {
                ConfigValue::Interpolation(s.clone())
            } else {
                ConfigValue::String(s.clone())
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            let values: Vec<ConfigValue> = seq.iter().map(yaml_to_config_value).collect();
            ConfigValue::List(values)
        }
        serde_yaml::Value::Mapping(map) => {
            let mut dict = ConfigDict::new();
            for (key, value) in map {
                if let serde_yaml::Value::String(k) = key {
                    dict.insert(k.clone(), yaml_to_config_value(value));
                }
            }
            ConfigValue::Dict(dict)
        }
    }
}

/// Extract the defaults list from a config
pub fn extract_defaults(config: &ConfigValue) -> Option<Vec<ConfigValue>> {
    if let ConfigValue::Dict(dict) = config {
        if let Some(ConfigValue::List(defaults)) = dict.get("defaults") {
            return Some(defaults.clone());
        }
    }
    None
}

/// Extract the package header from YAML content
pub fn extract_header(content: &str) -> std::collections::HashMap<String, String> {
    let mut header = std::collections::HashMap::new();

    // Look for @key value directives in comments at the start
    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Process comment lines
        if trimmed.starts_with('#') {
            let comment = trimmed.trim_start_matches('#').trim();

            // Check for @key pattern
            if comment.starts_with('@') {
                // Split on whitespace to get key and value
                let parts: Vec<&str> = comment.splitn(2, char::is_whitespace).collect();
                if parts.len() >= 2 {
                    let key = parts[0].trim_start_matches('@').trim();
                    let value = parts[1].trim();
                    if !key.is_empty() && !value.is_empty() {
                        header.insert(key.to_string(), value.to_string());
                    }
                } else if parts.len() == 1 {
                    // Handle @package:value or @package without value
                    let part = parts[0].trim_start_matches('@');
                    if let Some(idx) = part.find(':') {
                        let key = &part[..idx];
                        let value = part[idx + 1..].trim();
                        if !key.is_empty() && !value.is_empty() {
                            header.insert(key.to_string(), value.to_string());
                        }
                    }
                }
            }
        } else if !trimmed.starts_with("---") {
            // Stop at first non-comment, non-empty, non-yaml-separator line
            break;
        }
    }

    header
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_yaml() {
        let yaml = r#"
name: test
value: 42
enabled: true
ratio: 3.14
"#;
        let config = parse_yaml(yaml).unwrap();
        let dict = config.as_dict().unwrap();

        assert_eq!(dict.get("name").unwrap().as_str(), Some("test"));
        assert_eq!(dict.get("value").unwrap().as_int(), Some(42));
        assert_eq!(dict.get("enabled").unwrap().as_bool(), Some(true));
        assert_eq!(dict.get("ratio").unwrap().as_float(), Some(3.14));
    }

    #[test]
    fn test_parse_nested_yaml() {
        let yaml = r#"
db:
  host: localhost
  port: 3306
"#;
        let config = parse_yaml(yaml).unwrap();
        let dict = config.as_dict().unwrap();

        let db = dict.get("db").unwrap().as_dict().unwrap();
        assert_eq!(db.get("host").unwrap().as_str(), Some("localhost"));
        assert_eq!(db.get("port").unwrap().as_int(), Some(3306));
    }

    #[test]
    fn test_parse_list() {
        let yaml = r#"
items:
  - one
  - two
  - three
"#;
        let config = parse_yaml(yaml).unwrap();
        let dict = config.as_dict().unwrap();

        let items = dict.get("items").unwrap().as_list().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].as_str(), Some("one"));
    }

    #[test]
    fn test_parse_interpolation() {
        let yaml = r#"
db_host: ${db.host}
"#;
        let config = parse_yaml(yaml).unwrap();
        let dict = config.as_dict().unwrap();

        assert!(dict.get("db_host").unwrap().is_interpolation());
    }

    #[test]
    fn test_parse_missing() {
        let yaml = r#"
required: ???
"#;
        let config = parse_yaml(yaml).unwrap();
        let dict = config.as_dict().unwrap();

        assert!(dict.get("required").unwrap().is_missing());
    }

    #[test]
    fn test_extract_defaults() {
        let yaml = r#"
defaults:
  - db: mysql
  - server: apache
name: myapp
"#;
        let config = parse_yaml(yaml).unwrap();
        let defaults = extract_defaults(&config).unwrap();

        assert_eq!(defaults.len(), 2);
    }

    #[test]
    fn test_extract_header() {
        let yaml = "# @package db\nhost: localhost\n";
        let header = extract_header(yaml);

        assert_eq!(header.get("package"), Some(&"db".to_string()));
    }

    #[test]
    fn test_extract_header_multiple() {
        let yaml = "# @package _global_\n# @mode strict\nhost: localhost\n";
        let header = extract_header(yaml);

        assert_eq!(header.get("package"), Some(&"_global_".to_string()));
        assert_eq!(header.get("mode"), Some(&"strict".to_string()));
    }

    #[test]
    fn test_extract_header_with_empty_lines() {
        let yaml = "\n# @package db\n\nhost: localhost\n";
        let header = extract_header(yaml);

        assert_eq!(header.get("package"), Some(&"db".to_string()));
    }

    #[test]
    fn test_extract_header_stops_at_content() {
        let yaml = "# @package db\nhost: localhost\n# @ignored comment\n";
        let header = extract_header(yaml);

        // Should only have package, not the comment after content
        assert_eq!(header.len(), 1);
        assert_eq!(header.get("package"), Some(&"db".to_string()));
    }
}
