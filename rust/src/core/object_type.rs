// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Object type enumeration for config items.

/// Represents the type of a configuration object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ObjectType {
    /// Object not found
    NotFound = 0,
    /// A configuration file/node
    Config = 1,
    /// A configuration group (directory)
    Group = 2,
}

impl ObjectType {
    /// Check if the object was found
    pub fn is_found(&self) -> bool {
        !matches!(self, ObjectType::NotFound)
    }

    /// Check if the object is a config
    pub fn is_config(&self) -> bool {
        matches!(self, ObjectType::Config)
    }

    /// Check if the object is a group
    pub fn is_group(&self) -> bool {
        matches!(self, ObjectType::Group)
    }
}

impl Default for ObjectType {
    fn default() -> Self {
        ObjectType::NotFound
    }
}

impl std::fmt::Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectType::NotFound => write!(f, "NOT_FOUND"),
            ObjectType::Config => write!(f, "CONFIG"),
            ObjectType::Group => write!(f, "GROUP"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_type_values() {
        assert_eq!(ObjectType::NotFound as u8, 0);
        assert_eq!(ObjectType::Config as u8, 1);
        assert_eq!(ObjectType::Group as u8, 2);
    }

    #[test]
    fn test_is_found() {
        assert!(!ObjectType::NotFound.is_found());
        assert!(ObjectType::Config.is_found());
        assert!(ObjectType::Group.is_found());
    }

    #[test]
    fn test_is_config() {
        assert!(!ObjectType::NotFound.is_config());
        assert!(ObjectType::Config.is_config());
        assert!(!ObjectType::Group.is_config());
    }

    #[test]
    fn test_is_group() {
        assert!(!ObjectType::NotFound.is_group());
        assert!(!ObjectType::Config.is_group());
        assert!(ObjectType::Group.is_group());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", ObjectType::NotFound), "NOT_FOUND");
        assert_eq!(format!("{}", ObjectType::Config), "CONFIG");
        assert_eq!(format!("{}", ObjectType::Group), "GROUP");
    }

    #[test]
    fn test_default() {
        assert_eq!(ObjectType::default(), ObjectType::NotFound);
    }
}
