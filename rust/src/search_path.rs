// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Config search path management for finding configuration files.
//!
//! This module implements the search path system that Hydra/Lerna uses to locate
//! configuration files from multiple sources (file system, packages, etc.).

use std::fmt;

/// A single element in the config search path
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchPathElement {
    /// Provider name (e.g., "hydra", "main", "plugin")
    pub provider: String,
    /// The actual path (e.g., "file://conf" or "pkg://myapp.conf")
    pub path: String,
}

impl SearchPathElement {
    /// Create a new search path element
    pub fn new(provider: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            path: path.into(),
        }
    }

    /// Get the scheme from the path (e.g., "file", "pkg")
    pub fn scheme(&self) -> Option<&str> {
        self.path.find("://").map(|idx| &self.path[..idx])
    }

    /// Get the path without the scheme
    pub fn path_without_scheme(&self) -> &str {
        if let Some(idx) = self.path.find("://") {
            &self.path[idx + 3..]
        } else {
            &self.path
        }
    }
}

impl fmt::Display for SearchPathElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "provider={}, path={}", self.provider, self.path)
    }
}

/// Query for matching search path elements
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchPathQuery {
    /// Optional provider to match
    pub provider: Option<String>,
    /// Optional path to match
    pub path: Option<String>,
}

impl SearchPathQuery {
    /// Create an empty query
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a query matching a provider
    pub fn by_provider(provider: impl Into<String>) -> Self {
        Self {
            provider: Some(provider.into()),
            path: None,
        }
    }

    /// Create a query matching a path
    pub fn by_path(path: impl Into<String>) -> Self {
        Self {
            provider: None,
            path: Some(path.into()),
        }
    }

    /// Create a query matching both provider and path
    pub fn by_both(provider: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            provider: Some(provider.into()),
            path: Some(path.into()),
        }
    }

    /// Check if this query matches an element
    pub fn matches(&self, element: &SearchPathElement) -> bool {
        let provider_match = self
            .provider
            .as_ref()
            .map(|p| p == &element.provider)
            .unwrap_or(true);
        let path_match = self
            .path
            .as_ref()
            .map(|p| p == &element.path)
            .unwrap_or(true);

        // At least one criteria must be specified
        if self.provider.is_none() && self.path.is_none() {
            return false;
        }

        provider_match && path_match
    }
}

/// The configuration search path, containing ordered elements to search
#[derive(Clone, Debug, Default)]
pub struct ConfigSearchPath {
    elements: Vec<SearchPathElement>,
}

impl ConfigSearchPath {
    /// Create an empty search path
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a search path from a list of elements
    pub fn from_elements(elements: Vec<SearchPathElement>) -> Self {
        Self { elements }
    }

    /// Get all elements in the search path
    pub fn get_path(&self) -> &[SearchPathElement] {
        &self.elements
    }

    /// Get mutable access to the elements
    pub fn get_path_mut(&mut self) -> &mut Vec<SearchPathElement> {
        &mut self.elements
    }

    /// Get the number of elements in the search path
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if the search path is empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Find the first element matching the query
    /// Returns the index if found, or -1 if not found
    pub fn find_first_match(&self, query: &SearchPathQuery) -> i32 {
        for (idx, element) in self.elements.iter().enumerate() {
            if query.matches(element) {
                return idx as i32;
            }
        }
        -1
    }

    /// Find the last element matching the query
    /// Returns the index if found, or -1 if not found
    pub fn find_last_match(&self, query: &SearchPathQuery) -> i32 {
        for (idx, element) in self.elements.iter().enumerate().rev() {
            if query.matches(element) {
                return idx as i32;
            }
        }
        -1
    }

    /// Append an element to the end of the search path
    pub fn append(&mut self, provider: impl Into<String>, path: impl Into<String>) {
        self.elements.push(SearchPathElement::new(provider, path));
    }

    /// Append an element after an anchor (if found), otherwise append at end
    pub fn append_after(
        &mut self,
        provider: impl Into<String>,
        path: impl Into<String>,
        anchor: &SearchPathQuery,
    ) {
        let element = SearchPathElement::new(provider, path);
        let idx = self.find_last_match(anchor);
        if idx >= 0 {
            self.elements.insert((idx + 1) as usize, element);
        } else {
            self.elements.push(element);
        }
    }

    /// Prepend an element to the start of the search path
    pub fn prepend(&mut self, provider: impl Into<String>, path: impl Into<String>) {
        self.elements
            .insert(0, SearchPathElement::new(provider, path));
    }

    /// Prepend an element before an anchor (if found), otherwise prepend at start
    pub fn prepend_before(
        &mut self,
        provider: impl Into<String>,
        path: impl Into<String>,
        anchor: &SearchPathQuery,
    ) {
        let element = SearchPathElement::new(provider, path);
        let idx = self.find_first_match(anchor);
        if idx > 0 {
            self.elements.insert(idx as usize, element);
        } else {
            self.elements.insert(0, element);
        }
    }

    /// Remove all elements matching the query
    pub fn remove(&mut self, query: &SearchPathQuery) -> usize {
        let len_before = self.elements.len();
        self.elements.retain(|e| !query.matches(e));
        len_before - self.elements.len()
    }

    /// Clear all elements
    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// Check if any element matches the query
    pub fn contains(&self, query: &SearchPathQuery) -> bool {
        self.find_first_match(query) >= 0
    }

    /// Get element at index
    pub fn get(&self, index: usize) -> Option<&SearchPathElement> {
        self.elements.get(index)
    }

    /// Iterate over all elements
    pub fn iter(&self) -> impl Iterator<Item = &SearchPathElement> {
        self.elements.iter()
    }
}

impl IntoIterator for ConfigSearchPath {
    type Item = SearchPathElement;
    type IntoIter = std::vec::IntoIter<SearchPathElement>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}

impl<'a> IntoIterator for &'a ConfigSearchPath {
    type Item = &'a SearchPathElement;
    type IntoIter = std::slice::Iter<'a, SearchPathElement>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}

impl fmt::Display for ConfigSearchPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (idx, element) in self.elements.iter().enumerate() {
            if idx > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", element)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_path_element_new() {
        let elem = SearchPathElement::new("hydra", "file://conf");
        assert_eq!(elem.provider, "hydra");
        assert_eq!(elem.path, "file://conf");
    }

    #[test]
    fn test_search_path_element_scheme() {
        let elem = SearchPathElement::new("hydra", "file://conf");
        assert_eq!(elem.scheme(), Some("file"));

        let elem2 = SearchPathElement::new("hydra", "pkg://myapp.conf");
        assert_eq!(elem2.scheme(), Some("pkg"));

        let elem3 = SearchPathElement::new("hydra", "conf");
        assert_eq!(elem3.scheme(), None);
    }

    #[test]
    fn test_search_path_element_path_without_scheme() {
        let elem = SearchPathElement::new("hydra", "file://conf");
        assert_eq!(elem.path_without_scheme(), "conf");

        let elem2 = SearchPathElement::new("hydra", "pkg://myapp.conf");
        assert_eq!(elem2.path_without_scheme(), "myapp.conf");

        let elem3 = SearchPathElement::new("hydra", "conf");
        assert_eq!(elem3.path_without_scheme(), "conf");
    }

    #[test]
    fn test_search_path_query_matches() {
        let elem = SearchPathElement::new("hydra", "file://conf");

        // Match by provider
        let query = SearchPathQuery::by_provider("hydra");
        assert!(query.matches(&elem));

        // Match by path
        let query = SearchPathQuery::by_path("file://conf");
        assert!(query.matches(&elem));

        // Match by both
        let query = SearchPathQuery::by_both("hydra", "file://conf");
        assert!(query.matches(&elem));

        // No match - wrong provider
        let query = SearchPathQuery::by_provider("other");
        assert!(!query.matches(&elem));

        // No match - wrong path
        let query = SearchPathQuery::by_path("pkg://conf");
        assert!(!query.matches(&elem));

        // Empty query matches nothing
        let query = SearchPathQuery::new();
        assert!(!query.matches(&elem));
    }

    #[test]
    fn test_config_search_path_append() {
        let mut sp = ConfigSearchPath::new();
        sp.append("hydra", "file://conf1");
        sp.append("main", "file://conf2");

        assert_eq!(sp.len(), 2);
        assert_eq!(sp.get(0).unwrap().provider, "hydra");
        assert_eq!(sp.get(1).unwrap().provider, "main");
    }

    #[test]
    fn test_config_search_path_prepend() {
        let mut sp = ConfigSearchPath::new();
        sp.append("hydra", "file://conf1");
        sp.prepend("main", "file://conf2");

        assert_eq!(sp.len(), 2);
        assert_eq!(sp.get(0).unwrap().provider, "main");
        assert_eq!(sp.get(1).unwrap().provider, "hydra");
    }

    #[test]
    fn test_config_search_path_find() {
        let mut sp = ConfigSearchPath::new();
        sp.append("hydra", "file://conf1");
        sp.append("main", "file://conf2");
        sp.append("hydra", "file://conf3");

        // Find first hydra
        let query = SearchPathQuery::by_provider("hydra");
        assert_eq!(sp.find_first_match(&query), 0);

        // Find last hydra
        assert_eq!(sp.find_last_match(&query), 2);

        // Find by path
        let query = SearchPathQuery::by_path("file://conf2");
        assert_eq!(sp.find_first_match(&query), 1);

        // Not found
        let query = SearchPathQuery::by_provider("other");
        assert_eq!(sp.find_first_match(&query), -1);
    }

    #[test]
    fn test_config_search_path_append_after() {
        let mut sp = ConfigSearchPath::new();
        sp.append("hydra", "file://conf1");
        sp.append("main", "file://conf2");

        // Append after hydra
        let anchor = SearchPathQuery::by_provider("hydra");
        sp.append_after("plugin", "file://plugin_conf", &anchor);

        assert_eq!(sp.len(), 3);
        assert_eq!(sp.get(0).unwrap().provider, "hydra");
        assert_eq!(sp.get(1).unwrap().provider, "plugin");
        assert_eq!(sp.get(2).unwrap().provider, "main");
    }

    #[test]
    fn test_config_search_path_prepend_before() {
        let mut sp = ConfigSearchPath::new();
        sp.append("hydra", "file://conf1");
        sp.append("main", "file://conf2");

        // Prepend before main
        let anchor = SearchPathQuery::by_provider("main");
        sp.prepend_before("plugin", "file://plugin_conf", &anchor);

        assert_eq!(sp.len(), 3);
        assert_eq!(sp.get(0).unwrap().provider, "hydra");
        assert_eq!(sp.get(1).unwrap().provider, "plugin");
        assert_eq!(sp.get(2).unwrap().provider, "main");
    }

    #[test]
    fn test_config_search_path_remove() {
        let mut sp = ConfigSearchPath::new();
        sp.append("hydra", "file://conf1");
        sp.append("main", "file://conf2");
        sp.append("hydra", "file://conf3");

        // Remove all hydra elements
        let query = SearchPathQuery::by_provider("hydra");
        let removed = sp.remove(&query);

        assert_eq!(removed, 2);
        assert_eq!(sp.len(), 1);
        assert_eq!(sp.get(0).unwrap().provider, "main");
    }

    #[test]
    fn test_config_search_path_contains() {
        let mut sp = ConfigSearchPath::new();
        sp.append("hydra", "file://conf1");

        assert!(sp.contains(&SearchPathQuery::by_provider("hydra")));
        assert!(!sp.contains(&SearchPathQuery::by_provider("other")));
    }

    #[test]
    fn test_config_search_path_display() {
        let mut sp = ConfigSearchPath::new();
        sp.append("hydra", "file://conf");

        let s = format!("{}", sp);
        assert!(s.contains("provider=hydra"));
        assert!(s.contains("path=file://conf"));
    }

    #[test]
    fn test_config_search_path_iter() {
        let mut sp = ConfigSearchPath::new();
        sp.append("hydra", "file://conf1");
        sp.append("main", "file://conf2");

        let providers: Vec<_> = sp.iter().map(|e| e.provider.as_str()).collect();
        assert_eq!(providers, vec!["hydra", "main"]);
    }
}
