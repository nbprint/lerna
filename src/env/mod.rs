// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyO3 bindings for environment variable handling

use lerna::env::{
    find_env_refs, get_all_env, get_many_env, is_env_set, parse_env_ref, resolve_env_string,
    EnvResolver as RustEnvResolver,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;

/// Environment variable resolver with caching and default value support
#[pyclass(name = "EnvResolver")]
#[derive(Clone)]
pub struct PyEnvResolver {
    inner: RustEnvResolver,
}

#[pymethods]
impl PyEnvResolver {
    #[new]
    #[pyo3(signature = (use_cache=true, overrides=None))]
    fn new(use_cache: bool, overrides: Option<HashMap<String, String>>) -> Self {
        let mut resolver = if use_cache {
            RustEnvResolver::new()
        } else {
            RustEnvResolver::without_cache()
        };

        if let Some(ovr) = overrides {
            for (k, v) in ovr {
                resolver.set_override(k, v);
            }
        }

        Self { inner: resolver }
    }

    /// Get an environment variable value
    /// Returns None if not found
    fn get(&mut self, key: &str) -> Option<String> {
        self.inner.get(key).ok().map(|s| s.to_string())
    }

    /// Get an environment variable with a default value
    fn get_or_default(&mut self, key: &str, default: &str) -> String {
        self.inner.get_or_default(key, default)
    }

    /// Get an environment variable, raising an error if not found
    fn get_required(&mut self, key: &str) -> PyResult<String> {
        self.inner
            .get_required(key)
            .map_err(|e| PyValueError::new_err(e))
    }

    /// Clear the cache
    fn clear_cache(&mut self) {
        self.inner.clear_cache();
    }

    /// Add an override
    fn set_override(&mut self, key: String, value: String) {
        self.inner.set_override(key, value);
    }

    /// Resolve all environment variable references in a string
    fn resolve_string(&mut self, s: &str) -> PyResult<String> {
        resolve_env_string(s, &mut self.inner).map_err(|e| PyValueError::new_err(e))
    }

    /// Enable/disable caching
    fn enable_caching(&mut self, enabled: bool) {
        self.inner.enable_caching(enabled);
    }

    fn __repr__(&self) -> String {
        "EnvResolver()".to_string()
    }
}

/// Parse an environment variable reference like ${oc.env:VAR} or ${oc.env:VAR,default}
/// Returns (var_name, default_value) or None if not a valid reference
#[pyfunction]
#[pyo3(name = "parse_env_reference")]
fn py_parse_env_ref(s: &str) -> Option<(String, Option<String>)> {
    parse_env_ref(s)
}

/// Find all environment variable references in a string
/// Returns list of (start, end, var_name, default_value) tuples
#[pyfunction]
#[pyo3(name = "find_env_references")]
fn py_find_env_refs(s: &str) -> Vec<(usize, usize, String, Option<String>)> {
    find_env_refs(s)
}

/// Resolve all environment variable references in a string
#[pyfunction]
#[pyo3(name = "resolve_env_string")]
#[pyo3(signature = (s, overrides=None))]
fn py_resolve_env_string(s: &str, overrides: Option<HashMap<String, String>>) -> PyResult<String> {
    let mut resolver = if let Some(ovr) = overrides {
        RustEnvResolver::with_overrides(ovr)
    } else {
        RustEnvResolver::new()
    };

    resolve_env_string(s, &mut resolver).map_err(|e| PyValueError::new_err(e))
}

/// Get all environment variables as a dict
#[pyfunction]
#[pyo3(name = "get_all_environment")]
fn py_get_all_env() -> HashMap<String, String> {
    get_all_env()
}

/// Check if an environment variable is set
#[pyfunction]
#[pyo3(name = "is_env_set")]
fn py_is_env_set(key: &str) -> bool {
    is_env_set(key)
}

/// Get multiple environment variables at once
#[pyfunction]
#[pyo3(name = "get_many_env")]
fn py_get_many_env(keys: Vec<String>) -> HashMap<String, Option<String>> {
    let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
    get_many_env(&key_refs)
}

/// Get an environment variable with optional default
#[pyfunction]
#[pyo3(name = "get_env")]
#[pyo3(signature = (key, default=None))]
fn py_get_env(key: &str, default: Option<String>) -> Option<String> {
    match std::env::var(key) {
        Ok(v) => Some(v),
        Err(_) => default,
    }
}

/// Register environment functions as a submodule
pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "env")?;
    m.add_class::<PyEnvResolver>()?;
    m.add_function(wrap_pyfunction!(py_parse_env_ref, &m)?)?;
    m.add_function(wrap_pyfunction!(py_find_env_refs, &m)?)?;
    m.add_function(wrap_pyfunction!(py_resolve_env_string, &m)?)?;
    m.add_function(wrap_pyfunction!(py_get_all_env, &m)?)?;
    m.add_function(wrap_pyfunction!(py_is_env_set, &m)?)?;
    m.add_function(wrap_pyfunction!(py_get_many_env, &m)?)?;
    m.add_function(wrap_pyfunction!(py_get_env, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
