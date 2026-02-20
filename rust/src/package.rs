// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Package path resolution for config composition
//!
//! Handles the @package directive and package path computation for config composition.

/// Package resolution context
#[derive(Clone, Debug, Default)]
pub struct PackageResolver {
    /// Current config group path (e.g., "db/mysql")
    config_group: String,
    /// Package override from @package directive
    package_override: Option<String>,
    /// Header package (first @package directive in file)
    header_package: Option<String>,
}

impl PackageResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config_group(mut self, group: &str) -> Self {
        self.config_group = group.to_string();
        self
    }

    pub fn with_package_override(mut self, package: &str) -> Self {
        self.package_override = Some(package.to_string());
        self
    }

    pub fn with_header_package(mut self, package: &str) -> Self {
        self.header_package = Some(package.to_string());
        self
    }

    /// Resolve the final package path for a config
    pub fn resolve(&self) -> String {
        // Package override takes precedence
        if let Some(ref pkg) = self.package_override {
            return self.resolve_special_package(pkg);
        }

        // Then header package
        if let Some(ref pkg) = self.header_package {
            return self.resolve_special_package(pkg);
        }

        // Default to config group
        self.config_group.clone()
    }

    /// Resolve special package values like _global_, _group_, _name_
    fn resolve_special_package(&self, package: &str) -> String {
        match package {
            "_global_" => String::new(),
            "_group_" => self.config_group.clone(),
            "_name_" => self.get_config_name(),
            pkg => {
                // Handle relative package with _group_
                if pkg.starts_with("_group_.") {
                    let suffix = &pkg[8..];
                    if self.config_group.is_empty() {
                        suffix.to_string()
                    } else {
                        format!("{}.{}", self.config_group, suffix)
                    }
                } else {
                    pkg.to_string()
                }
            }
        }
    }

    /// Get just the config name from the group path
    fn get_config_name(&self) -> String {
        if let Some(pos) = self.config_group.rfind('/') {
            self.config_group[pos + 1..].to_string()
        } else {
            self.config_group.clone()
        }
    }
}

/// Parse @package directive from config header
pub fn parse_package_header(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with('#') {
            // Check for @package in comment
            if let Some(pkg_start) = trimmed.find("@package") {
                let rest = &trimmed[pkg_start + 8..].trim();
                // Get package value (ends at whitespace or end of line)
                let pkg_value = rest.split_whitespace().next()?;
                return Some(pkg_value.to_string());
            }
            continue;
        }

        // Once we hit non-comment content, stop
        if !trimmed.is_empty() {
            break;
        }
    }
    None
}

/// Compute the target path for a config value given package and key path
pub fn compute_target_path(package: &str, key_path: &str) -> String {
    if package.is_empty() {
        key_path.to_string()
    } else if key_path.is_empty() {
        package.to_string()
    } else {
        format!("{}.{}", package, key_path)
    }
}

/// Split a target path into components
pub fn split_path(path: &str) -> Vec<String> {
    if path.is_empty() {
        Vec::new()
    } else {
        path.split('.').map(String::from).collect()
    }
}

/// Join path components
pub fn join_path(components: &[String]) -> String {
    components.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_resolver_default() {
        let resolver = PackageResolver::new().with_config_group("db/mysql");
        assert_eq!(resolver.resolve(), "db/mysql");
    }

    #[test]
    fn test_package_resolver_override() {
        let resolver = PackageResolver::new()
            .with_config_group("db/mysql")
            .with_package_override("database");
        assert_eq!(resolver.resolve(), "database");
    }

    #[test]
    fn test_package_resolver_global() {
        let resolver = PackageResolver::new()
            .with_config_group("db/mysql")
            .with_package_override("_global_");
        assert_eq!(resolver.resolve(), "");
    }

    #[test]
    fn test_package_resolver_group() {
        let resolver = PackageResolver::new()
            .with_config_group("db/mysql")
            .with_package_override("_group_");
        assert_eq!(resolver.resolve(), "db/mysql");
    }

    #[test]
    fn test_package_resolver_name() {
        let resolver = PackageResolver::new()
            .with_config_group("db/mysql")
            .with_package_override("_name_");
        assert_eq!(resolver.resolve(), "mysql");
    }

    #[test]
    fn test_package_resolver_relative() {
        let resolver = PackageResolver::new()
            .with_config_group("db")
            .with_package_override("_group_.connection");
        assert_eq!(resolver.resolve(), "db.connection");
    }

    #[test]
    fn test_parse_package_header() {
        let content = "# @package _global_\ndb:\n  host: localhost";
        assert_eq!(parse_package_header(content), Some("_global_".to_string()));
    }

    #[test]
    fn test_parse_package_header_none() {
        let content = "db:\n  host: localhost";
        assert_eq!(parse_package_header(content), None);
    }

    #[test]
    fn test_compute_target_path() {
        assert_eq!(compute_target_path("db", "host"), "db.host");
        assert_eq!(compute_target_path("", "host"), "host");
        assert_eq!(compute_target_path("db", ""), "db");
    }

    #[test]
    fn test_split_path() {
        assert_eq!(split_path("db.host.port"), vec!["db", "host", "port"]);
        assert_eq!(split_path(""), Vec::<String>::new());
    }
}
