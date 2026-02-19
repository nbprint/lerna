// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyO3 bindings for config search path management

use pyo3::prelude::*;
use lerna::search_path::{
    ConfigSearchPath as RustConfigSearchPath,
    SearchPathElement as RustSearchPathElement,
    SearchPathQuery as RustSearchPathQuery,
};

/// A single element in the config search path
#[pyclass(name = "SearchPathElement")]
#[derive(Clone)]
pub struct PySearchPathElement {
    inner: RustSearchPathElement,
}

#[pymethods]
impl PySearchPathElement {
    #[new]
    fn new(provider: String, path: String) -> Self {
        Self {
            inner: RustSearchPathElement::new(provider, path),
        }
    }

    /// Get the provider name
    #[getter]
    fn provider(&self) -> String {
        self.inner.provider.clone()
    }

    /// Set the provider name
    #[setter]
    fn set_provider(&mut self, value: String) {
        self.inner.provider = value;
    }

    /// Get the path
    #[getter]
    fn path(&self) -> String {
        self.inner.path.clone()
    }

    /// Set the path
    #[setter]
    fn set_path(&mut self, value: String) {
        self.inner.path = value;
    }

    /// Get the scheme from the path (e.g., "file", "pkg")
    fn scheme(&self) -> Option<String> {
        self.inner.scheme().map(|s| s.to_string())
    }

    /// Get the path without the scheme
    fn path_without_scheme(&self) -> String {
        self.inner.path_without_scheme().to_string()
    }

    fn __repr__(&self) -> String {
        format!("SearchPathElement(provider={}, path={})", self.inner.provider, self.inner.path)
    }

    fn __str__(&self) -> String {
        format!("provider={}, path={}", self.inner.provider, self.inner.path)
    }
}

/// Query for matching search path elements
#[pyclass(name = "SearchPathQuery")]
#[derive(Clone)]
pub struct PySearchPathQuery {
    inner: RustSearchPathQuery,
}

#[pymethods]
impl PySearchPathQuery {
    #[new]
    #[pyo3(signature = (provider=None, path=None))]
    fn new(provider: Option<String>, path: Option<String>) -> Self {
        Self {
            inner: RustSearchPathQuery {
                provider,
                path,
            },
        }
    }

    /// Create a query matching a provider
    #[staticmethod]
    fn by_provider(provider: String) -> Self {
        Self {
            inner: RustSearchPathQuery::by_provider(provider),
        }
    }

    /// Create a query matching a path
    #[staticmethod]
    fn by_path(path: String) -> Self {
        Self {
            inner: RustSearchPathQuery::by_path(path),
        }
    }

    /// Create a query matching both provider and path
    #[staticmethod]
    fn by_both(provider: String, path: String) -> Self {
        Self {
            inner: RustSearchPathQuery::by_both(provider, path),
        }
    }

    #[getter]
    fn provider(&self) -> Option<String> {
        self.inner.provider.clone()
    }

    #[setter]
    fn set_provider(&mut self, value: Option<String>) {
        self.inner.provider = value;
    }

    #[getter]
    fn path(&self) -> Option<String> {
        self.inner.path.clone()
    }

    #[setter]
    fn set_path(&mut self, value: Option<String>) {
        self.inner.path = value;
    }

    /// Check if this query matches an element
    fn matches(&self, element: &PySearchPathElement) -> bool {
        self.inner.matches(&element.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "SearchPathQuery(provider={:?}, path={:?})",
            self.inner.provider,
            self.inner.path
        )
    }
}

/// The configuration search path
#[pyclass(name = "RustConfigSearchPath")]
#[derive(Clone)]
pub struct PyConfigSearchPath {
    inner: RustConfigSearchPath,
}

#[pymethods]
impl PyConfigSearchPath {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustConfigSearchPath::new(),
        }
    }

    /// Create from a list of (provider, path) tuples
    #[staticmethod]
    fn from_tuples(tuples: Vec<(String, String)>) -> Self {
        let elements: Vec<RustSearchPathElement> = tuples
            .into_iter()
            .map(|(p, path)| RustSearchPathElement::new(p, path))
            .collect();
        Self {
            inner: RustConfigSearchPath::from_elements(elements),
        }
    }

    /// Get the number of elements
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get all elements as a list
    fn get_path(&self) -> Vec<PySearchPathElement> {
        self.inner
            .get_path()
            .iter()
            .map(|e| PySearchPathElement { inner: e.clone() })
            .collect()
    }

    /// Get element at index
    fn get(&self, index: usize) -> Option<PySearchPathElement> {
        self.inner.get(index).map(|e| PySearchPathElement { inner: e.clone() })
    }

    /// Find the first element matching the query
    fn find_first_match(&self, query: &PySearchPathQuery) -> i32 {
        self.inner.find_first_match(&query.inner)
    }

    /// Find the last element matching the query
    fn find_last_match(&self, query: &PySearchPathQuery) -> i32 {
        self.inner.find_last_match(&query.inner)
    }

    /// Append an element to the end
    fn append(&mut self, provider: String, path: String) {
        self.inner.append(provider, path);
    }

    /// Append an element after an anchor (if found)
    fn append_after(&mut self, provider: String, path: String, anchor: &PySearchPathQuery) {
        self.inner.append_after(provider, path, &anchor.inner);
    }

    /// Prepend an element to the start
    fn prepend(&mut self, provider: String, path: String) {
        self.inner.prepend(provider, path);
    }

    /// Prepend an element before an anchor (if found)
    fn prepend_before(&mut self, provider: String, path: String, anchor: &PySearchPathQuery) {
        self.inner.prepend_before(provider, path, &anchor.inner);
    }

    /// Remove all elements matching the query
    fn remove(&mut self, query: &PySearchPathQuery) -> usize {
        self.inner.remove(&query.inner)
    }

    /// Clear all elements
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Check if any element matches the query
    fn contains(&self, query: &PySearchPathQuery) -> bool {
        self.inner.contains(&query.inner)
    }

    fn __repr__(&self) -> String {
        format!("RustConfigSearchPath(len={})", self.inner.len())
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }
}

/// Register search path types with the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySearchPathElement>()?;
    m.add_class::<PySearchPathQuery>()?;
    m.add_class::<PyConfigSearchPath>()?;
    Ok(())
}
