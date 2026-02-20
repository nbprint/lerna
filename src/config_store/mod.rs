// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyO3 bindings for ConfigStore

use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use lerna::config::value::{ConfigDict, ConfigValue};
use lerna::config_store::{self, ConfigNode as RustConfigNode};

/// Convert ConfigValue to a Python object
fn config_value_to_py(py: Python, value: &ConfigValue) -> PyResult<Py<PyAny>> {
    match value {
        ConfigValue::Null => Ok(py.None()),
        ConfigValue::Bool(b) => Ok((*b).into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::Int(i) => Ok((*i).into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::Float(f) => Ok((*f).into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::String(s) => Ok(s.as_str().into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::Interpolation(s) => {
            Ok(s.as_str().into_pyobject(py)?.to_owned().into_any().unbind())
        }
        ConfigValue::Missing => Ok("???".into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::List(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(config_value_to_py(py, item)?)?;
            }
            Ok(list.into_any().unbind())
        }
        ConfigValue::Dict(dict) => config_dict_to_py(py, dict),
    }
}

/// Convert ConfigDict to a Python dict
fn config_dict_to_py(py: Python, dict: &ConfigDict) -> PyResult<Py<PyAny>> {
    let py_dict = PyDict::new(py);
    for (key, value) in dict.iter() {
        py_dict.set_item(key, config_value_to_py(py, value)?)?;
    }
    Ok(py_dict.into_any().unbind())
}

/// Convert a Python object to ConfigValue
fn py_to_config_value(py: Python, obj: &Bound<'_, PyAny>) -> PyResult<ConfigValue> {
    if obj.is_none() {
        Ok(ConfigValue::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(ConfigValue::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(ConfigValue::Int(i))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(ConfigValue::Float(f))
    } else if let Ok(s) = obj.extract::<String>() {
        if s == "???" {
            Ok(ConfigValue::Missing)
        } else {
            Ok(ConfigValue::String(s))
        }
    } else if let Ok(list) = obj.cast::<PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(py_to_config_value(py, &item)?);
        }
        Ok(ConfigValue::List(items))
    } else if let Ok(dict) = obj.cast::<PyDict>() {
        let mut config_dict = ConfigDict::new();
        for (key, value) in dict.iter() {
            if let Ok(k) = key.extract::<String>() {
                config_dict.insert(k, py_to_config_value(py, &value)?);
            }
        }
        Ok(ConfigValue::Dict(config_dict))
    } else {
        // Fallback to string representation
        Ok(ConfigValue::String(obj.str()?.to_string()))
    }
}

/// Python wrapper for a ConfigNode
#[pyclass(name = "RustConfigNode")]
#[derive(Clone)]
pub struct PyConfigNode {
    name: String,
    node: ConfigDict,
    group: Option<String>,
    package: Option<String>,
    provider: Option<String>,
}

#[pymethods]
impl PyConfigNode {
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    #[getter]
    fn node(&self, py: Python) -> PyResult<Py<PyAny>> {
        config_dict_to_py(py, &self.node)
    }

    #[getter]
    fn group(&self) -> Option<&str> {
        self.group.as_deref()
    }

    #[getter]
    fn package(&self) -> Option<&str> {
        self.package.as_deref()
    }

    #[getter]
    fn provider(&self) -> Option<&str> {
        self.provider.as_deref()
    }
}

impl From<RustConfigNode> for PyConfigNode {
    fn from(node: RustConfigNode) -> Self {
        Self {
            name: node.name,
            node: node.node,
            group: node.group,
            package: node.package,
            provider: node.provider,
        }
    }
}

/// Python wrapper for the ConfigStore singleton
///
/// This class provides access to the global Rust ConfigStore instance.
/// It mirrors the Python ConfigStore API but uses Rust for storage.
#[pyclass(name = "RustConfigStore")]
pub struct PyConfigStore;

#[pymethods]
impl PyConfigStore {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Store a config node
    ///
    /// Args:
    ///     name: Config name
    ///     node: Config data (dict)
    ///     group: Optional group path (e.g., "db" or "hydra/launcher")
    ///     package: Optional package path
    ///     provider: Optional provider name
    #[pyo3(signature = (name, node, group=None, package=None, provider=None))]
    fn store(
        &self,
        py: Python,
        name: &str,
        node: &Bound<'_, PyAny>,
        group: Option<&str>,
        package: Option<&str>,
        provider: Option<&str>,
    ) -> PyResult<()> {
        let config_dict = match py_to_config_value(py, node)? {
            ConfigValue::Dict(d) => d,
            _ => {
                // If not a dict, wrap it
                let mut d = ConfigDict::new();
                d.insert("_value_".to_string(), py_to_config_value(py, node)?);
                d
            }
        };

        let store = config_store::instance();
        store.store(name, config_dict, group, package, provider);
        Ok(())
    }

    /// Load a config by path
    ///
    /// Args:
    ///     config_path: The config path (e.g., "db/mysql" or "config")
    ///
    /// Returns:
    ///     RustConfigNode if found
    ///
    /// Raises:
    ///     KeyError: If config not found
    fn load(&self, config_path: &str) -> PyResult<PyConfigNode> {
        let store = config_store::instance();
        match store.load(config_path) {
            Some(node) => Ok(node.into()),
            None => Err(PyKeyError::new_err(format!(
                "Structured config not found: {}",
                config_path
            ))),
        }
    }

    /// Get the type of a path
    ///
    /// Args:
    ///     path: The path to check
    ///
    /// Returns:
    ///     "CONFIG", "GROUP", or "NOT_FOUND"
    fn get_type(&self, path: &str) -> String {
        let store = config_store::instance();
        store.get_type(path).to_string()
    }

    /// List items in a path
    ///
    /// Args:
    ///     path: The path to list
    ///
    /// Returns:
    ///     List of item names
    ///
    /// Raises:
    ///     OSError: If path not found or is not a group
    fn list(&self, path: &str) -> PyResult<Vec<String>> {
        let store = config_store::instance();
        match store.list(path) {
            Some(items) => Ok(items),
            None => Err(pyo3::exceptions::PyOSError::new_err(format!(
                "Path not found: {}",
                path
            ))),
        }
    }

    /// Check if a config exists
    fn config_exists(&self, config_path: &str) -> bool {
        let store = config_store::instance();
        store.config_exists(config_path)
    }

    /// Check if a group exists
    fn group_exists(&self, group_path: &str) -> bool {
        let store = config_store::instance();
        store.group_exists(group_path)
    }

    /// Clear all stored configs
    fn clear(&self) {
        let store = config_store::instance();
        store.clear();
    }
}

/// Register the config_store module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyConfigNode>()?;
    m.add_class::<PyConfigStore>()?;
    m.add_function(wrap_pyfunction!(test_config_store, m)?)?;
    Ok(())
}

/// Test function to verify config_store module is loaded
#[pyfunction]
fn test_config_store() -> String {
    "ConfigStore module loaded successfully!".to_string()
}
