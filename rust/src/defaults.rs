// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Default element types for configuration composition

/// Result of resolving a default
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultDefault {
    /// Path to the config file
    pub config_path: Option<String>,
    /// Parent config path
    pub parent: Option<String>,
    /// Package for the config
    pub package: Option<String>,
    /// Whether this is a _self_ reference
    pub is_self: bool,
    /// Whether this is the primary config
    pub primary: bool,
    /// Override key for this default
    pub override_key: Option<String>,
}

impl Default for ResultDefault {
    fn default() -> Self {
        Self {
            config_path: None,
            parent: None,
            package: None,
            is_self: false,
            primary: false,
            override_key: None,
        }
    }
}

impl ResultDefault {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config_path(mut self, path: String) -> Self {
        self.config_path = Some(path);
        self
    }

    pub fn with_package(mut self, package: String) -> Self {
        self.package = Some(package);
        self
    }

    pub fn with_parent(mut self, parent: String) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn as_self(mut self) -> Self {
        self.is_self = true;
        self
    }

    pub fn as_primary(mut self) -> Self {
        self.primary = true;
        self
    }
}

/// Type of default (config or group)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefaultType {
    /// A config file default
    Config,
    /// A config group default
    Group,
    /// Virtual root node
    VirtualRoot,
}

/// Base information for an input default
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputDefaultBase {
    /// Package for this default
    pub package: Option<String>,
    /// Parent base directory
    pub parent_base_dir: Option<String>,
    /// Parent package
    pub parent_package: Option<String>,
    /// Package header from the config
    pub package_header: Option<String>,
    /// Whether this is the primary config
    pub primary: bool,
}

impl Default for InputDefaultBase {
    fn default() -> Self {
        Self {
            package: None,
            parent_base_dir: None,
            parent_package: None,
            package_header: None,
            primary: false,
        }
    }
}

impl InputDefaultBase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_parent(
        &mut self,
        parent_base_dir: Option<String>,
        parent_package: Option<String>,
    ) {
        if self.parent_package.is_none() || self.parent_package == parent_package {
            self.parent_package = parent_package;
        }
        if self.parent_base_dir.is_none() || self.parent_base_dir == parent_base_dir {
            self.parent_base_dir = parent_base_dir;
        }
    }

    /// Get the package, optionally falling back to package header
    pub fn get_package(&self, default_to_package_header: bool) -> Option<&str> {
        if self.package.is_none() && default_to_package_header {
            self.package_header.as_deref()
        } else {
            self.package.as_deref()
        }
    }
}

/// A config file default
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigDefault {
    /// Base default information
    pub base: InputDefaultBase,
    /// Path to the config (relative or absolute)
    pub path: Option<String>,
    /// Whether this default is optional
    pub optional: bool,
    /// Whether this default is deleted
    pub deleted: bool,
}

impl Default for ConfigDefault {
    fn default() -> Self {
        Self {
            base: InputDefaultBase::default(),
            path: None,
            optional: false,
            deleted: false,
        }
    }
}

impl ConfigDefault {
    pub fn new(path: String) -> Self {
        Self {
            path: Some(path),
            ..Self::default()
        }
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    pub fn is_self(&self) -> bool {
        self.path.as_deref() == Some("_self_")
    }

    /// Get the config name (basename)
    pub fn get_name(&self) -> Option<&str> {
        self.path.as_ref().map(|p| {
            if let Some(idx) = p.rfind('/') {
                &p[idx + 1..]
            } else {
                p.as_str()
            }
        })
    }

    /// Get the group path (parent directory)
    pub fn get_group_path(&self) -> String {
        let parent_base = self.base.parent_base_dir.as_deref().unwrap_or("");
        let path = self.path.as_deref().unwrap_or("");

        let (path, absolute) = if path.starts_with('/') {
            (&path[1..], true)
        } else {
            (path, false)
        };

        let group = if let Some(idx) = path.rfind('/') {
            &path[..idx]
        } else {
            ""
        };

        if absolute {
            group.to_string()
        } else if parent_base.is_empty() {
            group.to_string()
        } else if group.is_empty() {
            parent_base.to_string()
        } else {
            format!("{}/{}", parent_base, group)
        }
    }

    /// Get the full config path
    pub fn get_config_path(&self) -> String {
        let parent_base = self.base.parent_base_dir.as_deref().unwrap_or("");
        let path = self.path.as_deref().unwrap_or("");

        let (path, absolute) = if path.starts_with('/') {
            (&path[1..], true)
        } else {
            (path, false)
        };

        if absolute {
            path.to_string()
        } else if parent_base.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", parent_base, path)
        }
    }

    /// Get the default package based on group path
    pub fn get_default_package(&self) -> String {
        self.get_group_path().replace("/", ".")
    }
}

/// A config group default
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupDefault {
    /// Base default information
    pub base: InputDefaultBase,
    /// The config group name
    pub group: String,
    /// The selected option value(s)
    pub value: GroupValue,
    /// Whether this default is optional
    pub optional: bool,
    /// Whether this default is deleted
    pub deleted: bool,
    /// Whether this is an override
    pub is_override: bool,
    /// Whether this was added externally
    pub external_append: bool,
    /// Whether the config name was overridden
    pub config_name_overridden: bool,
}

/// Value for a group default (single or multiple)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GroupValue {
    Single(String),
    Multiple(Vec<String>),
}

impl GroupValue {
    pub fn as_single(&self) -> Option<&str> {
        match self {
            GroupValue::Single(s) => Some(s),
            GroupValue::Multiple(_) => None,
        }
    }

    pub fn as_multiple(&self) -> Option<&Vec<String>> {
        match self {
            GroupValue::Multiple(v) => Some(v),
            GroupValue::Single(_) => None,
        }
    }

    pub fn is_missing(&self) -> bool {
        match self {
            GroupValue::Single(s) => s == "???",
            GroupValue::Multiple(_) => false,
        }
    }
}

impl Default for GroupDefault {
    fn default() -> Self {
        Self {
            base: InputDefaultBase::default(),
            group: String::new(),
            value: GroupValue::Single(String::new()),
            optional: false,
            deleted: false,
            is_override: false,
            external_append: false,
            config_name_overridden: false,
        }
    }
}

impl GroupDefault {
    pub fn new(group: String, value: String) -> Self {
        Self {
            group,
            value: GroupValue::Single(value),
            ..Self::default()
        }
    }

    pub fn new_multi(group: String, values: Vec<String>) -> Self {
        Self {
            group,
            value: GroupValue::Multiple(values),
            ..Self::default()
        }
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    pub fn as_override(mut self) -> Self {
        self.is_override = true;
        self
    }

    /// Get the group path
    pub fn get_group_path(&self) -> String {
        let parent_base = self.base.parent_base_dir.as_deref().unwrap_or("");

        if self.group.starts_with('/') {
            self.group[1..].to_string()
        } else if parent_base.is_empty() {
            self.group.clone()
        } else {
            format!("{}/{}", parent_base, self.group)
        }
    }

    /// Get the config path for a specific value
    pub fn get_config_path(&self, value: &str) -> String {
        let group_path = self.get_group_path();
        if group_path.is_empty() {
            value.to_string()
        } else {
            format!("{}/{}", group_path, value)
        }
    }

    /// Get the default package based on group path
    pub fn get_default_package(&self) -> String {
        self.get_group_path().replace("/", ".")
    }

    /// Get the override key
    pub fn get_override_key(&self) -> String {
        let default_pkg = self.get_default_package();
        let final_pkg = self.get_final_package(false);
        let key = self.get_group_path();

        if default_pkg != final_pkg {
            let pkg = if final_pkg.is_empty() {
                "_global_"
            } else {
                &final_pkg
            };
            format!("{}@{}", key, pkg)
        } else {
            key
        }
    }

    /// Get the final package
    pub fn get_final_package(&self, default_to_package_header: bool) -> String {
        let parent_package = self.base.parent_package.as_deref().unwrap_or("");
        let package = self.base.get_package(default_to_package_header);

        let pkg = match package {
            Some(p) => p.to_string(),
            None => self.get_group_path().replace("/", "."),
        };

        if parent_package.is_empty() {
            pkg
        } else if pkg.is_empty() {
            parent_package.to_string()
        } else {
            format!("{}.{}", parent_package, pkg)
        }
    }

    pub fn is_missing(&self) -> bool {
        self.value.is_missing()
    }
}

/// A node in the defaults tree
#[derive(Clone, Debug)]
pub struct DefaultsTreeNode {
    /// The node content (can be any type of default)
    pub node: DefaultNodeContent,
    /// Child nodes
    pub children: Option<Vec<DefaultsTreeNode>>,
}

/// Content of a defaults tree node
#[derive(Clone, Debug)]
pub enum DefaultNodeContent {
    VirtualRoot,
    Config(ConfigDefault),
    Group(GroupDefault),
}

impl DefaultsTreeNode {
    pub fn virtual_root() -> Self {
        Self {
            node: DefaultNodeContent::VirtualRoot,
            children: None,
        }
    }

    pub fn config(default: ConfigDefault) -> Self {
        Self {
            node: DefaultNodeContent::Config(default),
            children: None,
        }
    }

    pub fn group(default: GroupDefault) -> Self {
        Self {
            node: DefaultNodeContent::Group(default),
            children: None,
        }
    }

    pub fn with_children(mut self, children: Vec<DefaultsTreeNode>) -> Self {
        self.children = Some(children);
        self
    }

    pub fn is_virtual_root(&self) -> bool {
        matches!(self.node, DefaultNodeContent::VirtualRoot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_default() {
        let rd = ResultDefault::new()
            .with_config_path("db/mysql".to_string())
            .with_package("db".to_string())
            .as_primary();

        assert_eq!(rd.config_path, Some("db/mysql".to_string()));
        assert_eq!(rd.package, Some("db".to_string()));
        assert!(rd.primary);
        assert!(!rd.is_self);
    }

    #[test]
    fn test_config_default_is_self() {
        let cd = ConfigDefault::new("_self_".to_string());
        assert!(cd.is_self());

        let cd = ConfigDefault::new("db/mysql".to_string());
        assert!(!cd.is_self());
    }

    #[test]
    fn test_config_default_get_name() {
        let cd = ConfigDefault::new("db/mysql".to_string());
        assert_eq!(cd.get_name(), Some("mysql"));

        let cd = ConfigDefault::new("config".to_string());
        assert_eq!(cd.get_name(), Some("config"));
    }

    #[test]
    fn test_config_default_group_path() {
        let mut cd = ConfigDefault::new("db/mysql".to_string());
        cd.base.parent_base_dir = Some("".to_string());
        assert_eq!(cd.get_group_path(), "db");

        let mut cd = ConfigDefault::new("mysql".to_string());
        cd.base.parent_base_dir = Some("db".to_string());
        assert_eq!(cd.get_group_path(), "db");
    }

    #[test]
    fn test_group_default() {
        let gd = GroupDefault::new("db".to_string(), "mysql".to_string());
        assert_eq!(gd.group, "db");
        assert_eq!(gd.value.as_single(), Some("mysql"));
    }

    #[test]
    fn test_group_default_multi() {
        let gd = GroupDefault::new_multi(
            "db".to_string(),
            vec!["mysql".to_string(), "postgres".to_string()],
        );
        assert_eq!(
            gd.value.as_multiple(),
            Some(&vec!["mysql".to_string(), "postgres".to_string()])
        );
    }

    #[test]
    fn test_group_value_is_missing() {
        let v = GroupValue::Single("???".to_string());
        assert!(v.is_missing());

        let v = GroupValue::Single("mysql".to_string());
        assert!(!v.is_missing());
    }

    #[test]
    fn test_defaults_tree_node() {
        let root = DefaultsTreeNode::virtual_root().with_children(vec![
            DefaultsTreeNode::config(ConfigDefault::new("config".to_string())),
            DefaultsTreeNode::group(GroupDefault::new("db".to_string(), "mysql".to_string())),
        ]);

        assert!(root.is_virtual_root());
        assert!(root.children.is_some());
        assert_eq!(root.children.as_ref().unwrap().len(), 2);
    }
}
