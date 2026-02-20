// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! ConfigStore - A singleton store for structured configurations.
//!
//! This module provides a Rust implementation of Hydra's ConfigStore,
//! which stores structured configs that can be composed with file-based configs.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::config::value::ConfigDict;
use crate::ObjectType;

/// A config node stored in the ConfigStore
#[derive(Debug, Clone)]
pub struct ConfigNode {
    /// The name of the config (e.g., "mysql.yaml")
    pub name: String,
    /// The config data
    pub node: ConfigDict,
    /// The config group (e.g., "db")
    pub group: Option<String>,
    /// The package path (e.g., "db")
    pub package: Option<String>,
    /// The provider name (e.g., "my_app")
    pub provider: Option<String>,
}

impl ConfigNode {
    pub fn new(
        name: String,
        node: ConfigDict,
        group: Option<String>,
        package: Option<String>,
        provider: Option<String>,
    ) -> Self {
        Self {
            name,
            node,
            group,
            package,
            provider,
        }
    }
}

/// Entry in the repository - either a group (directory) or a config (file)
#[derive(Debug, Clone)]
pub enum RepoEntry {
    /// A group containing subgroups and configs
    Group(HashMap<String, RepoEntry>),
    /// A config node
    Config(ConfigNode),
}

/// A singleton ConfigStore for structured configurations
///
/// This is a thread-safe store that mirrors the Python ConfigStore.
/// It uses RwLock for safe concurrent access.
#[derive(Debug)]
pub struct ConfigStore {
    /// The repository tree
    repo: RwLock<HashMap<String, RepoEntry>>,
}

impl ConfigStore {
    /// Create a new ConfigStore
    pub fn new() -> Self {
        Self {
            repo: RwLock::new(HashMap::new()),
        }
    }

    /// Store a config node
    ///
    /// # Arguments
    /// * `name` - Config name (without .yaml extension)
    /// * `node` - The config data
    /// * `group` - Optional group path (e.g., "db" or "hydra/launcher")
    /// * `package` - Optional package path (e.g., "db")
    /// * `provider` - Optional provider name
    pub fn store(
        &self,
        name: &str,
        node: ConfigDict,
        group: Option<&str>,
        package: Option<&str>,
        provider: Option<&str>,
    ) {
        let mut repo = self.repo.write().unwrap();

        // Navigate to the correct location
        let mut cur: &mut HashMap<String, RepoEntry> = &mut *repo;

        if let Some(group_path) = group {
            for part in group_path.split('/') {
                if part.is_empty() {
                    continue;
                }
                // Ensure the group exists
                if !cur.contains_key(part) {
                    cur.insert(part.to_string(), RepoEntry::Group(HashMap::new()));
                }
                // Navigate into the group
                if let RepoEntry::Group(ref mut inner) = cur.get_mut(part).unwrap() {
                    cur = inner;
                } else {
                    // This shouldn't happen - a config where we expected a group
                    return;
                }
            }
        }

        // Add .yaml suffix if not present
        let full_name = if name.ends_with(".yaml") {
            name.to_string()
        } else {
            format!("{}.yaml", name)
        };

        // Store the config node
        let config_node = ConfigNode::new(
            full_name.clone(),
            node,
            group.map(|s| s.to_string()),
            package.map(|s| s.to_string()),
            provider.map(|s| s.to_string()),
        );
        cur.insert(full_name, RepoEntry::Config(config_node));
    }

    /// Load a config by path
    ///
    /// # Arguments
    /// * `config_path` - The config path (e.g., "db/mysql" or "config")
    ///
    /// # Returns
    /// The config node if found
    pub fn load(&self, config_path: &str) -> Option<ConfigNode> {
        let repo = self.repo.read().unwrap();
        self.load_from_repo(&repo, config_path)
    }

    fn load_from_repo(
        &self,
        repo: &HashMap<String, RepoEntry>,
        config_path: &str,
    ) -> Option<ConfigNode> {
        let path = if config_path.ends_with(".yaml") {
            config_path.to_string()
        } else {
            format!("{}.yaml", config_path)
        };

        // Split into group path and name
        if let Some(idx) = path.rfind('/') {
            let group_path = &path[..idx];
            let name = &path[idx + 1..];

            // Navigate to the group
            let mut cur = repo;
            for part in group_path.split('/') {
                if part.is_empty() {
                    continue;
                }
                match cur.get(part) {
                    Some(RepoEntry::Group(inner)) => cur = inner,
                    _ => return None,
                }
            }

            // Get the config
            match cur.get(name) {
                Some(RepoEntry::Config(node)) => Some(node.clone()),
                _ => None,
            }
        } else {
            // No group path, config at root
            match repo.get(&path) {
                Some(RepoEntry::Config(node)) => Some(node.clone()),
                _ => None,
            }
        }
    }

    /// Get the type of a path (group or config)
    pub fn get_type(&self, path: &str) -> ObjectType {
        let repo = self.repo.read().unwrap();

        if path.is_empty() {
            // Root is always a group if it has content
            if repo.is_empty() {
                return ObjectType::NotFound;
            }
            return ObjectType::Group;
        }

        let mut cur: &HashMap<String, RepoEntry> = &repo;
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();

        for (i, part) in parts.iter().enumerate() {
            // Check with .yaml suffix if this is the last part
            let key = if i == parts.len() - 1 {
                // Could be config or group
                if cur.contains_key(*part) {
                    part.to_string()
                } else if cur.contains_key(&format!("{}.yaml", part)) {
                    format!("{}.yaml", part)
                } else {
                    return ObjectType::NotFound;
                }
            } else {
                part.to_string()
            };

            match cur.get(&key) {
                Some(RepoEntry::Group(inner)) => {
                    if i == parts.len() - 1 {
                        return ObjectType::Group;
                    }
                    cur = inner;
                }
                Some(RepoEntry::Config(_)) => {
                    if i == parts.len() - 1 {
                        return ObjectType::Config;
                    }
                    return ObjectType::NotFound; // Can't navigate into a config
                }
                None => return ObjectType::NotFound,
            }
        }

        ObjectType::Group
    }

    /// List items in a path
    pub fn list(&self, path: &str) -> Option<Vec<String>> {
        let repo = self.repo.read().unwrap();

        if path.is_empty() {
            // List root
            let mut items: Vec<String> = repo.keys().cloned().collect();
            items.sort();
            return Some(items);
        }

        let mut cur: &HashMap<String, RepoEntry> = &repo;
        for part in path.split('/') {
            if part.is_empty() {
                continue;
            }
            match cur.get(part) {
                Some(RepoEntry::Group(inner)) => cur = inner,
                _ => return None,
            }
        }

        let mut items: Vec<String> = cur.keys().cloned().collect();
        items.sort();
        Some(items)
    }

    /// Check if a config exists
    pub fn config_exists(&self, config_path: &str) -> bool {
        self.get_type(config_path) == ObjectType::Config
    }

    /// Check if a group exists
    pub fn group_exists(&self, group_path: &str) -> bool {
        self.get_type(group_path) == ObjectType::Group
    }

    /// Clear all stored configs
    pub fn clear(&self) {
        let mut repo = self.repo.write().unwrap();
        repo.clear();
    }
}

impl Default for ConfigStore {
    fn default() -> Self {
        Self::new()
    }
}

// Global singleton instance using OnceLock
use std::sync::OnceLock;

static INSTANCE: OnceLock<Arc<ConfigStore>> = OnceLock::new();

/// Get the global ConfigStore instance
pub fn instance() -> Arc<ConfigStore> {
    Arc::clone(INSTANCE.get_or_init(|| Arc::new(ConfigStore::new())))
}

/// Clear and reset the global ConfigStore instance
/// This is primarily for testing purposes
pub fn reset_instance() {
    // Note: OnceLock doesn't support reset, so we just clear the existing instance
    if let Some(store) = INSTANCE.get() {
        store.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::value::ConfigValue;

    fn make_test_dict() -> ConfigDict {
        let mut dict = ConfigDict::new();
        dict.insert(
            "driver".to_string(),
            ConfigValue::String("mysql".to_string()),
        );
        dict.insert("port".to_string(), ConfigValue::Int(3306));
        dict
    }

    #[test]
    fn test_store_and_load_simple() {
        let store = ConfigStore::new();
        let node = make_test_dict();

        store.store("config", node.clone(), None, None, None);

        let loaded = store.load("config").unwrap();
        assert_eq!(loaded.name, "config.yaml");
    }

    #[test]
    fn test_store_with_group() {
        let store = ConfigStore::new();
        let node = make_test_dict();

        store.store("mysql", node.clone(), Some("db"), Some("db"), Some("test"));

        let loaded = store.load("db/mysql").unwrap();
        assert_eq!(loaded.name, "mysql.yaml");
        assert_eq!(loaded.group, Some("db".to_string()));
        assert_eq!(loaded.package, Some("db".to_string()));
    }

    #[test]
    fn test_get_type() {
        let store = ConfigStore::new();
        let node = make_test_dict();

        store.store("mysql", node.clone(), Some("db"), None, None);

        assert_eq!(store.get_type("db"), ObjectType::Group);
        assert_eq!(store.get_type("db/mysql"), ObjectType::Config);
        assert_eq!(store.get_type("nonexistent"), ObjectType::NotFound);
    }

    #[test]
    fn test_list() {
        let store = ConfigStore::new();
        let node = make_test_dict();

        store.store("mysql", node.clone(), Some("db"), None, None);
        store.store("postgres", node.clone(), Some("db"), None, None);

        let items = store.list("db").unwrap();
        assert_eq!(items, vec!["mysql.yaml", "postgres.yaml"]);
    }
}
