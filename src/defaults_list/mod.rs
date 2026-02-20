// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Python bindings for defaults list builder

use pyo3::prelude::*;
use pyo3::types::PyDict;

use lerna::defaults::GroupDefault;
use lerna::defaults_list::{DefaultsListBuilder, DefaultsListResult, Overrides};

/// Python wrapper for Overrides
#[pyclass(name = "RustOverrides")]
#[derive(Clone)]
pub struct PyOverrides {
    inner: Overrides,
}

#[pymethods]
impl PyOverrides {
    #[new]
    fn new() -> Self {
        Self {
            inner: Overrides::default(),
        }
    }

    /// Create from list of override strings
    #[staticmethod]
    fn from_strings(overrides: Vec<String>) -> Self {
        Self {
            inner: Overrides::from_overrides(&overrides),
        }
    }

    /// Get the choices dictionary
    fn get_choices(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.choices {
            match v {
                Some(val) => dict.set_item(k, val)?,
                None => dict.set_item(k, py.None())?,
            }
        }
        Ok(dict.into())
    }

    /// Get deletions set
    fn get_deletions(&self) -> Vec<String> {
        self.inner.deletions.keys().cloned().collect()
    }

    /// Get appends as list of (group, value) tuples
    fn get_appends(&self) -> Vec<(String, String)> {
        self.inner
            .appends
            .iter()
            .filter_map(|gd| {
                gd.value
                    .as_single()
                    .map(|v| (gd.group.clone(), v.to_string()))
            })
            .collect()
    }

    /// Check if a group is overridden
    fn is_overridden(&self, group: &str) -> bool {
        self.inner.choices.contains_key(group)
    }

    /// Check if a group is deleted
    fn is_deleted(&self, group: &str) -> bool {
        self.inner.deletions.contains_key(group)
    }

    /// Get the override value for a group
    fn get_choice(&self, group: &str) -> Option<String> {
        self.inner.choices.get(group).and_then(|v| v.clone())
    }

    /// Add a choice override
    fn add_choice(&mut self, group: String, value: Option<String>) {
        self.inner.choices.insert(group, value);
    }

    /// Add a deletion
    fn add_deletion(&mut self, group: String) {
        self.inner
            .deletions
            .insert(group, lerna::defaults_list::Deletion::default());
    }

    /// Add an append
    fn add_append(&mut self, group: String, value: String) {
        self.inner.appends.push(GroupDefault::new(group, value));
    }

    fn __repr__(&self) -> String {
        format!(
            "RustOverrides(choices={:?}, deletions={:?}, appends={})",
            self.inner.choices,
            self.inner.deletions,
            self.inner.appends.len()
        )
    }
}

/// Python wrapper for DefaultsListResult
#[pyclass(name = "RustDefaultsListResult")]
pub struct PyDefaultsListResult {
    inner: DefaultsListResult,
}

#[pymethods]
impl PyDefaultsListResult {
    /// Get all result defaults as list of dicts
    fn get_defaults(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        let mut results = Vec::new();
        for rd in &self.inner.defaults {
            let dict = PyDict::new(py);
            match &rd.config_path {
                Some(p) => dict.set_item("config_path", p)?,
                None => dict.set_item("config_path", py.None())?,
            }
            match &rd.parent {
                Some(p) => dict.set_item("parent", p)?,
                None => dict.set_item("parent", py.None())?,
            }
            match &rd.package {
                Some(p) => dict.set_item("package", p)?,
                None => dict.set_item("package", py.None())?,
            }
            dict.set_item("is_self", rd.is_self)?;
            dict.set_item("primary", rd.primary)?;
            match &rd.override_key {
                Some(k) => dict.set_item("override_key", k)?,
                None => dict.set_item("override_key", py.None())?,
            }
            results.push(dict.into());
        }
        Ok(results)
    }

    /// Get config overrides as list of strings
    fn get_config_overrides(&self) -> Vec<String> {
        self.inner.config_overrides.clone()
    }

    /// Get known choices as dict
    fn get_known_choices(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.known_choices {
            match v {
                Some(val) => dict.set_item(k, val)?,
                None => dict.set_item(k, py.None())?,
            }
        }
        Ok(dict.into())
    }

    /// Get count of defaults
    fn __len__(&self) -> usize {
        self.inner.defaults.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "RustDefaultsListResult({} defaults)",
            self.inner.defaults.len()
        )
    }
}

/// Build defaults list from config path
#[pyfunction]
#[pyo3(signature = (config_path, overrides, config_loader))]
pub fn build_defaults_list(
    py: Python<'_>,
    config_path: Option<&str>,
    overrides: Vec<String>,
    config_loader: Py<PyAny>,
) -> PyResult<PyDefaultsListResult> {
    // Create closures for the builder
    let loader = config_loader.clone_ref(py);
    let loader2 = config_loader.clone_ref(py);
    let loader3 = config_loader.clone_ref(py);

    let config_loader_fn = move |path: &str| -> Result<
        lerna::config::value::ConfigDict,
        lerna::config::parser::ConfigLoadError,
    > {
        Python::attach(|py| {
            let result = loader.call_method1(py, "load_config", (path,));
            match result {
                Ok(_obj) => {
                    // Return empty dict as placeholder
                    // Real implementation would convert Python dict to ConfigDict
                    Ok(lerna::config::value::ConfigDict::new())
                }
                Err(_) => Err(lerna::config::parser::ConfigLoadError::with_path(
                    "Config not found",
                    path,
                )),
            }
        })
    };

    let config_exists_fn = move |path: &str| -> bool {
        Python::attach(|py| {
            let result = loader2.call_method1(py, "config_exists", (path,));
            match result {
                Ok(obj) => obj.extract::<bool>(py).unwrap_or(false),
                Err(_) => false,
            }
        })
    };

    let group_exists_fn = move |group: &str| -> bool {
        Python::attach(|py| {
            let result = loader3.call_method1(py, "group_exists", (group,));
            match result {
                Ok(obj) => obj.extract::<bool>(py).unwrap_or(false),
                Err(_) => false,
            }
        })
    };

    let builder = DefaultsListBuilder::new(
        config_loader_fn,
        config_exists_fn,
        group_exists_fn,
        &overrides,
    );

    let result = builder
        .build(config_path)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    Ok(PyDefaultsListResult { inner: result })
}

/// Parse overrides into choices, deletions, and appends
#[pyfunction]
pub fn parse_overrides(overrides: Vec<String>) -> PyOverrides {
    PyOverrides::from_strings(overrides)
}

/// Register the module
pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "defaults_list")?;
    m.add_class::<PyOverrides>()?;
    m.add_class::<PyDefaultsListResult>()?;
    m.add_function(wrap_pyfunction!(build_defaults_list, &m)?)?;
    m.add_function(wrap_pyfunction!(parse_overrides, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
