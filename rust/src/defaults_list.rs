// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Defaults list builder and processor
//!
//! Builds and flattens the defaults tree to get the final list of configs to load.

use std::collections::{HashMap, HashSet};

use crate::config::parser::ConfigLoadError;
use crate::config::value::{ConfigDict, ConfigValue};
use crate::defaults::{
    ConfigDefault, DefaultNodeContent, DefaultsTreeNode, GroupDefault, GroupValue, ResultDefault,
};

/// Config composition error
#[derive(Debug, Clone)]
pub struct ConfigCompositionError {
    pub message: String,
}

impl ConfigCompositionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ConfigCompositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ConfigCompositionError {}

impl From<ConfigCompositionError> for ConfigLoadError {
    fn from(e: ConfigCompositionError) -> Self {
        ConfigLoadError::new(e.message)
    }
}

/// Represents an override choice for a config group
#[derive(Clone, Debug)]
pub struct OverrideChoice {
    pub group: String,
    pub value: Option<String>,
    pub is_add: bool,
    pub is_delete: bool,
}

/// Metadata about an override
#[derive(Clone, Debug, Default)]
pub struct OverrideMetadata {
    /// True if this came from command line (external), false if from config file
    pub external_override: bool,
    /// Path of the config that contains this override (for internal overrides)
    pub containing_config_path: Option<String>,
    /// Whether this override has been applied
    pub used: bool,
    /// The relative key (for error messages)
    pub relative_key: Option<String>,
}

/// Deletion tracking
#[derive(Clone, Debug, Default)]
pub struct Deletion {
    /// The specific value to delete (None = delete any)
    pub name: Option<String>,
    /// Whether this deletion has been applied
    pub used: bool,
}

/// Tracks overrides during defaults list processing
#[derive(Clone, Debug, Default)]
pub struct Overrides {
    /// group -> selected config
    pub choices: HashMap<String, Option<String>>,
    /// Metadata for each override
    pub override_metadata: HashMap<String, OverrideMetadata>,
    /// Groups that have been deleted
    pub deletions: HashMap<String, Deletion>,
    /// Groups to append
    pub appends: Vec<GroupDefault>,
    /// Known choices (group -> option)
    pub known_choices: HashMap<String, Option<String>>,
    /// Known choices per group (for error messages)
    pub known_choices_per_group: HashMap<String, HashSet<String>>,
}

impl Overrides {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse override strings and build choices map
    pub fn from_overrides(overrides: &[String]) -> Self {
        let mut result = Self::new();

        for ovr in overrides {
            if let Some(eq_pos) = ovr.find('=') {
                let key = &ovr[..eq_pos];
                let value = &ovr[eq_pos + 1..];

                // Skip value overrides (contain dots) - only process group overrides
                if key.contains('.') {
                    continue;
                }

                if key.starts_with('~') {
                    // Deletion: ~db or ~db=mysql
                    let group = &key[1..];
                    let deletion = Deletion {
                        name: if value.is_empty() {
                            None
                        } else {
                            Some(value.to_string())
                        },
                        used: false,
                    };
                    result.deletions.insert(group.to_string(), deletion);
                } else if key.starts_with('+') {
                    // Addition: +db=mysql
                    let group = &key[1..];
                    result
                        .appends
                        .push(GroupDefault::new(group.to_string(), value.to_string()));
                } else {
                    // Regular override: db=mysql
                    result
                        .choices
                        .insert(key.to_string(), Some(value.to_string()));
                    result.override_metadata.insert(
                        key.to_string(),
                        OverrideMetadata {
                            external_override: true,
                            containing_config_path: None,
                            used: false,
                            relative_key: None,
                        },
                    );
                }
            } else if ovr.starts_with('~') {
                // Delete without value: ~db
                let group = &ovr[1..];
                result
                    .deletions
                    .insert(group.to_string(), Deletion::default());
            }
        }

        result
    }

    /// Check if a group has an override
    pub fn get_override(&self, group: &str) -> Option<&str> {
        self.choices.get(group).and_then(|v| v.as_deref())
    }

    /// Check if a group is deleted
    pub fn is_deleted(&self, group: &str) -> bool {
        self.deletions.contains_key(group)
    }

    /// Check if a specific group/value pair is deleted
    pub fn is_deleted_with_value(&self, group: &str, value: Option<&str>) -> bool {
        if let Some(deletion) = self.deletions.get(group) {
            match (&deletion.name, value) {
                (None, _) => true, // Delete all
                (Some(del_name), Some(val)) => del_name == val,
                _ => false,
            }
        } else {
            false
        }
    }

    /// Mark an override as used
    pub fn mark_override_used(&mut self, key: &str) {
        if let Some(meta) = self.override_metadata.get_mut(key) {
            meta.used = true;
        }
    }

    /// Mark a deletion as used
    pub fn mark_deletion_used(&mut self, group: &str) {
        if let Some(deletion) = self.deletions.get_mut(group) {
            deletion.used = true;
        }
    }

    /// Record a known choice
    pub fn record_choice(&mut self, group: &str, value: Option<&str>) {
        self.known_choices
            .insert(group.to_string(), value.map(|s| s.to_string()));

        // Also track per-group choices for error messages
        self.known_choices_per_group
            .entry(group.split('@').next().unwrap_or(group).to_string())
            .or_default()
            .insert(group.to_string());
    }

    /// Add an internal override (from a config file)
    pub fn add_internal_override(
        &mut self,
        parent_config_path: &str,
        group: &str,
        value: &str,
        relative_key: Option<&str>,
    ) {
        if !self.choices.contains_key(group) {
            self.choices
                .insert(group.to_string(), Some(value.to_string()));
            self.override_metadata.insert(
                group.to_string(),
                OverrideMetadata {
                    external_override: false,
                    containing_config_path: Some(parent_config_path.to_string()),
                    used: false,
                    relative_key: relative_key.map(|s| s.to_string()),
                },
            );
        }
    }

    /// Ensure all overrides were used - returns error if any were not
    pub fn ensure_overrides_used(&self) -> Result<(), ConfigCompositionError> {
        for (key, meta) in &self.override_metadata {
            if !meta.used {
                let group = key.split('@').next().unwrap_or(key);
                let choices = self
                    .known_choices_per_group
                    .get(group)
                    .cloned()
                    .unwrap_or_default();

                let msg = if choices.len() > 1 {
                    let choice_list: Vec<String> = choices.iter().cloned().collect();
                    format!(
                        "Could not override '{}'. Did you mean to override one of {}?",
                        key,
                        choice_list.join(", ")
                    )
                } else if choices.len() == 1 {
                    format!(
                        "Could not override '{}'. Did you mean to override {}?",
                        key,
                        choices.iter().next().unwrap()
                    )
                } else {
                    format!(
                        "Could not override '{}'. No match in the defaults list.",
                        key
                    )
                };

                let mut full_msg = if let Some(ref path) = meta.containing_config_path {
                    format!("In '{}': {}", path, msg)
                } else {
                    msg
                };

                if meta.external_override {
                    if let Some(value) = self.choices.get(key).and_then(|v| v.as_ref()) {
                        full_msg.push_str(&format!(
                            "\nTo append to your default list use +{}={}",
                            key, value
                        ));
                    }
                }

                return Err(ConfigCompositionError::new(full_msg));
            }
        }
        Ok(())
    }

    /// Ensure all deletions were used - returns error if any were not
    pub fn ensure_deletions_used(&self) -> Result<(), ConfigCompositionError> {
        for (key, deletion) in &self.deletions {
            if !deletion.used {
                let desc = if let Some(ref name) = deletion.name {
                    format!("{}={}", key, name)
                } else {
                    key.clone()
                };
                return Err(ConfigCompositionError::new(format!(
                    "Could not delete '{}'. No match in the defaults list",
                    desc
                )));
            }
        }
        Ok(())
    }
}

/// Result of building a defaults list
#[derive(Clone, Debug)]
pub struct DefaultsListResult {
    /// Flattened list of defaults to load, in order
    pub defaults: Vec<ResultDefault>,
    /// The defaults tree
    pub tree: DefaultsTreeNode,
    /// Override tracking
    pub overrides: Overrides,
    /// Config overrides (key.path=value style)
    pub config_overrides: Vec<String>,
    /// Known choices from defaults processing
    pub known_choices: HashMap<String, Option<String>>,
}

/// Builds and processes defaults lists
pub struct DefaultsListBuilder<'a> {
    /// Function to load a config by path
    config_loader: Box<dyn Fn(&str) -> Result<ConfigDict, ConfigLoadError> + 'a>,
    /// Function to check if a config exists
    config_exists: Box<dyn Fn(&str) -> bool + 'a>,
    /// Function to check if a group exists
    group_exists: Box<dyn Fn(&str) -> bool + 'a>,
    /// Override tracking
    overrides: Overrides,
    /// Seen config paths (for cycle detection)
    seen_paths: HashSet<String>,
}

impl<'a> DefaultsListBuilder<'a> {
    pub fn new<L, E, G>(
        config_loader: L,
        config_exists: E,
        group_exists: G,
        overrides: &[String],
    ) -> Self
    where
        L: Fn(&str) -> Result<ConfigDict, ConfigLoadError> + 'a,
        E: Fn(&str) -> bool + 'a,
        G: Fn(&str) -> bool + 'a,
    {
        Self {
            config_loader: Box::new(config_loader),
            config_exists: Box::new(config_exists),
            group_exists: Box::new(group_exists),
            overrides: Overrides::from_overrides(overrides),
            seen_paths: HashSet::new(),
        }
    }

    /// Build the defaults list from a config name
    pub fn build(
        mut self,
        config_name: Option<&str>,
    ) -> Result<DefaultsListResult, ConfigLoadError> {
        // Separate config overrides from group overrides
        let config_overrides: Vec<String> = self
            .overrides
            .choices
            .keys()
            .filter(|k| k.contains('.'))
            .cloned()
            .collect();

        // Build the tree starting from virtual root
        let mut root = DefaultsTreeNode::virtual_root();

        if let Some(name) = config_name {
            let mut primary = ConfigDefault {
                path: Some(name.to_string()),
                ..ConfigDefault::default()
            };
            primary.base.primary = true;
            let primary_node = self.build_tree_from_config(primary, true)?;
            root.children = Some(vec![primary_node]);
        }

        // Append any +group=value overrides
        if !self.overrides.appends.is_empty() {
            let mut children = root.children.take().unwrap_or_default();
            for gd in &self.overrides.appends {
                children.push(DefaultsTreeNode::group(gd.clone()));
            }
            root.children = Some(children);
        }

        // Flatten the tree
        let defaults = self.flatten_tree(&root)?;

        // Validate that all overrides were used
        self.overrides.ensure_overrides_used()?;

        // Validate that all deletions were used
        self.overrides.ensure_deletions_used()?;

        // Collect known choices from overrides before moving
        let known_choices = self.overrides.known_choices.clone();

        Ok(DefaultsListResult {
            defaults,
            tree: root,
            overrides: self.overrides,
            config_overrides,
            known_choices,
        })
    }

    /// Build tree for a config default
    fn build_tree_from_config(
        &mut self,
        config: ConfigDefault,
        _is_primary: bool,
    ) -> Result<DefaultsTreeNode, ConfigLoadError> {
        if config.is_self() {
            return Ok(DefaultsTreeNode::config(config));
        }

        let config_path = config.get_config_path();

        // Cycle detection
        if self.seen_paths.contains(&config_path) {
            return Err(ConfigLoadError::new(format!(
                "Circular dependency detected: {}",
                config_path
            )));
        }
        self.seen_paths.insert(config_path.clone());

        // Load the config to get its defaults
        let cfg = (self.config_loader)(&config_path)?;

        // Extract defaults list from config
        let defaults_list = cfg
            .get("defaults")
            .and_then(|v| v.as_list())
            .cloned()
            .unwrap_or_default();

        let mut children = Vec::new();
        let mut found_self = false;

        for default_val in &defaults_list {
            match self.parse_default_value(default_val, &config)? {
                ParsedDefault::SelfRef => {
                    found_self = true;
                    children.push(DefaultsTreeNode::config(ConfigDefault::new(
                        "_self_".to_string(),
                    )));
                }
                ParsedDefault::Config(cd) => {
                    let child = self.build_tree_from_config(cd, false)?;
                    children.push(child);
                }
                ParsedDefault::Group(mut gd) => {
                    // Apply override if exists
                    if let Some(override_val) = self.overrides.get_override(&gd.group) {
                        gd.value = GroupValue::Single(override_val.to_string());
                        gd.config_name_overridden = true;
                        // Mark this override as used
                        self.overrides.mark_override_used(&gd.group);
                    }

                    // Skip if deleted
                    if self
                        .overrides
                        .is_deleted_with_value(&gd.group, gd.value.as_single())
                    {
                        gd.deleted = true;
                        // Mark this deletion as used
                        self.overrides.mark_deletion_used(&gd.group);
                    }

                    // Record the choice
                    self.overrides
                        .record_choice(&gd.group, gd.value.as_single());

                    if !gd.deleted {
                        let child = self.build_tree_from_group(gd)?;
                        children.push(child);
                    }
                }
            }
        }

        // If no _self_ was found, add implicit _self_ at the end
        if !found_self {
            children.push(DefaultsTreeNode::config(ConfigDefault::new(
                "_self_".to_string(),
            )));
        }

        self.seen_paths.remove(&config_path);

        let mut node = DefaultsTreeNode::config(config);
        if !children.is_empty() {
            node.children = Some(children);
        }

        Ok(node)
    }

    /// Build tree for a group default
    fn build_tree_from_group(
        &mut self,
        group: GroupDefault,
    ) -> Result<DefaultsTreeNode, ConfigLoadError> {
        // For single value, load that config
        if let Some(value) = group.value.as_single() {
            if value == "???" {
                // Missing value - error will be handled later
                return Ok(DefaultsTreeNode::group(group));
            }

            let config_path = format!("{}/{}", group.group, value);

            // Check if config exists
            if !(self.config_exists)(&config_path) {
                if group.optional {
                    return Ok(DefaultsTreeNode::group(group));
                }
                return Err(ConfigLoadError::with_path("Config not found", &config_path));
            }

            // Load the config
            let cfg = (self.config_loader)(&config_path)?;

            // Check for nested defaults
            let defaults_list = cfg
                .get("defaults")
                .and_then(|v| v.as_list())
                .cloned()
                .unwrap_or_default();

            let mut children = Vec::new();
            for default_val in &defaults_list {
                let fake_parent = ConfigDefault {
                    path: Some(config_path.clone()),
                    base: crate::defaults::InputDefaultBase {
                        parent_base_dir: Some(group.group.clone()),
                        ..Default::default()
                    },
                    ..Default::default()
                };

                match self.parse_default_value(default_val, &fake_parent)? {
                    ParsedDefault::SelfRef => {
                        children.push(DefaultsTreeNode::config(ConfigDefault::new(
                            "_self_".to_string(),
                        )));
                    }
                    ParsedDefault::Config(cd) => {
                        let child = self.build_tree_from_config(cd, false)?;
                        children.push(child);
                    }
                    ParsedDefault::Group(gd) => {
                        let child = self.build_tree_from_group(gd)?;
                        children.push(child);
                    }
                }
            }

            let mut node = DefaultsTreeNode::group(group);
            if !children.is_empty() {
                node.children = Some(children);
            }
            return Ok(node);
        }

        // Multiple values - just return the node, expansion happens elsewhere
        Ok(DefaultsTreeNode::group(group))
    }

    /// Parse a default value from config
    fn parse_default_value(
        &self,
        value: &ConfigValue,
        parent: &ConfigDefault,
    ) -> Result<ParsedDefault, ConfigLoadError> {
        match value {
            ConfigValue::String(s) => {
                if s == "_self_" {
                    Ok(ParsedDefault::SelfRef)
                } else {
                    // Plain string default is a config path
                    let mut cd = ConfigDefault::new(s.clone());
                    cd.base.parent_base_dir = parent.base.parent_base_dir.clone();
                    Ok(ParsedDefault::Config(cd))
                }
            }
            ConfigValue::Dict(dict) => {
                // Dict can be group selection or config with options
                // Check for "optional" key
                let optional = dict
                    .get("optional")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Check for "override" key
                let is_override = dict
                    .get("override")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Find the actual group/config key (not optional/override)
                for (key, val) in dict.iter() {
                    if key == "optional" || key == "override" || key == "package" {
                        continue;
                    }

                    // This is a group selection: {group: value}
                    let value_str = match val {
                        ConfigValue::String(s) => s.clone(),
                        ConfigValue::Null => continue, // Skip null
                        _ => continue,
                    };

                    // Determine if this is a group (directory) or config
                    let full_path = if parent.base.parent_base_dir.is_some() {
                        format!("{}/{}", parent.base.parent_base_dir.as_ref().unwrap(), key)
                    } else {
                        key.to_string()
                    };

                    if (self.group_exists)(&full_path) {
                        let mut gd = GroupDefault::new(full_path, value_str);
                        gd.optional = optional;
                        gd.is_override = is_override;

                        // Get package if specified
                        if let Some(ConfigValue::String(pkg)) = dict.get("package") {
                            gd.base.package = Some(pkg.clone());
                        }

                        return Ok(ParsedDefault::Group(gd));
                    } else {
                        // Treat as config path
                        let path = format!("{}/{}", key, value_str);
                        let mut cd = ConfigDefault::new(path);
                        cd.optional = optional;
                        cd.base.parent_base_dir = parent.base.parent_base_dir.clone();
                        return Ok(ParsedDefault::Config(cd));
                    }
                }

                Err(ConfigLoadError::new("Invalid default entry"))
            }
            _ => Err(ConfigLoadError::new(format!(
                "Invalid default type: {:?}",
                value
            ))),
        }
    }

    /// Flatten the defaults tree to a list
    fn flatten_tree(&self, node: &DefaultsTreeNode) -> Result<Vec<ResultDefault>, ConfigLoadError> {
        let mut result = Vec::new();
        self.flatten_node(node, &mut result, None)?;
        Ok(result)
    }

    fn flatten_node(
        &self,
        node: &DefaultsTreeNode,
        result: &mut Vec<ResultDefault>,
        parent_path: Option<&str>,
    ) -> Result<(), ConfigLoadError> {
        // Process children first (depth-first)
        if let Some(children) = &node.children {
            for child in children {
                let current_path = match &node.node {
                    DefaultNodeContent::Config(cd) => cd.path.as_deref(),
                    DefaultNodeContent::Group(gd) => Some(gd.group.as_str()),
                    DefaultNodeContent::VirtualRoot => None,
                };
                self.flatten_node(child, result, current_path)?;
            }
        }

        // Add this node's result
        match &node.node {
            DefaultNodeContent::Config(cd) => {
                if !cd.is_self() && !cd.deleted {
                    let rd = ResultDefault {
                        config_path: Some(cd.get_config_path()),
                        parent: parent_path.map(|s| s.to_string()),
                        package: cd
                            .base
                            .package
                            .clone()
                            .or_else(|| Some(cd.get_default_package())),
                        is_self: false,
                        primary: cd.base.primary,
                        override_key: None,
                    };
                    result.push(rd);
                } else if cd.is_self() {
                    let rd = ResultDefault {
                        config_path: parent_path.map(|s| s.to_string()),
                        parent: None,
                        package: None,
                        is_self: true,
                        primary: false,
                        override_key: None,
                    };
                    result.push(rd);
                }
            }
            DefaultNodeContent::Group(gd) => {
                if !gd.deleted {
                    if let Some(value) = gd.value.as_single() {
                        let config_path = format!("{}/{}", gd.group, value);
                        let rd = ResultDefault {
                            config_path: Some(config_path),
                            parent: parent_path.map(|s| s.to_string()),
                            package: gd
                                .base
                                .package
                                .clone()
                                .or_else(|| Some(gd.group.replace("/", "."))),
                            is_self: false,
                            primary: gd.base.primary,
                            override_key: Some(gd.group.clone()),
                        };
                        result.push(rd);
                    }
                }
            }
            DefaultNodeContent::VirtualRoot => {
                // Virtual root doesn't produce a result
            }
        }

        Ok(())
    }
}

/// Parsed default value
enum ParsedDefault {
    SelfRef,
    Config(ConfigDefault),
    Group(GroupDefault),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overrides_from_strings() {
        let ovrs = Overrides::from_overrides(&[
            "db=mysql".to_string(),
            "server=prod".to_string(),
            "~cache".to_string(),
            "+logger=file".to_string(),
        ]);

        assert_eq!(ovrs.get_override("db"), Some("mysql"));
        assert_eq!(ovrs.get_override("server"), Some("prod"));
        assert!(ovrs.is_deleted("cache"));
        assert_eq!(ovrs.appends.len(), 1);
        assert_eq!(ovrs.appends[0].group, "logger");
    }

    #[test]
    fn test_overrides_value_override_skipped() {
        let ovrs = Overrides::from_overrides(&[
            "db.port=3306".to_string(), // This should be skipped (value override)
            "db=mysql".to_string(),     // This should be processed (group override)
        ]);

        assert_eq!(ovrs.get_override("db"), Some("mysql"));
        assert!(ovrs.get_override("db.port").is_none());
    }
}
