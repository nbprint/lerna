// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Flag management for OmegaConf nodes

use std::collections::HashMap;

/// Flags that can be set on OmegaConf nodes
#[derive(Debug, Clone, Default)]
pub struct Flags {
    flags: HashMap<String, bool>,
}

impl Flags {
    /// Create a new empty flags container
    pub fn new() -> Self {
        Self::default()
    }

    /// Create flags with initial values
    pub fn with_flags(flags: HashMap<String, bool>) -> Self {
        Self { flags }
    }

    /// Get a flag value
    pub fn get(&self, name: &str) -> Option<bool> {
        self.flags.get(name).copied()
    }

    /// Set a flag value
    pub fn set(&mut self, name: &str, value: Option<bool>) {
        if let Some(v) = value {
            self.flags.insert(name.to_string(), v);
        } else {
            self.flags.remove(name);
        }
    }

    /// Check if a flag is set to true
    pub fn is_set(&self, name: &str) -> bool {
        self.get(name).unwrap_or(false)
    }

    /// Merge flags from another Flags instance
    pub fn merge(&mut self, other: &Flags) {
        for (name, value) in &other.flags {
            self.flags.insert(name.clone(), *value);
        }
    }

    /// Clone the underlying flags map
    pub fn to_map(&self) -> HashMap<String, bool> {
        self.flags.clone()
    }
}

/// Standard flag names
pub mod flag_names {
    pub const STRUCT: &str = "struct";
    pub const READONLY: &str = "readonly";
    pub const ALLOW_OBJECTS: &str = "allow_objects";
    pub const NO_DEEPCOPY_SET_NODES: &str = "no_deepcopy_set_nodes";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flags_basic() {
        let mut flags = Flags::new();
        assert_eq!(flags.get("readonly"), None);

        flags.set("readonly", Some(true));
        assert_eq!(flags.get("readonly"), Some(true));
        assert!(flags.is_set("readonly"));

        flags.set("readonly", Some(false));
        assert_eq!(flags.get("readonly"), Some(false));
        assert!(!flags.is_set("readonly"));

        flags.set("readonly", None);
        assert_eq!(flags.get("readonly"), None);
    }

    #[test]
    fn test_flags_merge() {
        let mut flags1 = Flags::new();
        flags1.set("readonly", Some(true));

        let mut flags2 = Flags::new();
        flags2.set("struct", Some(true));
        flags2.set("readonly", Some(false));

        flags1.merge(&flags2);

        assert_eq!(flags1.get("readonly"), Some(false));
        assert_eq!(flags1.get("struct"), Some(true));
    }
}
