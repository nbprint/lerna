// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyOmegaConf - Python bindings for OmegaConf API

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use lerna::omegaconf::{
    DictConfig, OmegaConf,
    ConfigValue as RustConfigValue,
};

use super::dictconfig::{py_to_config_value, PyDictConfig};
use super::listconfig::PyListConfig;

/// Python-facing ConfigValue enum
#[pyclass(name = "ConfigValue")]
#[derive(Debug, Clone)]
pub enum PyConfigValue {
    NoneValue {},
    Missing {},
    Bool { value: bool },
    Int { value: i64 },
    Float { value: f64 },
    String { value: String },
    List { items: Vec<PyConfigValue> },
    Dict { items: HashMap<String, PyConfigValue> },
    Interpolation { expr: String },
}

#[pymethods]
impl PyConfigValue {
    /// Create a MISSING value
    #[staticmethod]
    fn missing() -> Self {
        PyConfigValue::Missing {}
    }

    /// Create a None value
    #[staticmethod]
    fn none() -> Self {
        PyConfigValue::NoneValue {}
    }

    /// Check if this is MISSING
    fn is_missing(&self) -> bool {
        matches!(self, PyConfigValue::Missing {})
    }

    /// Check if this is None
    fn is_none(&self) -> bool {
        matches!(self, PyConfigValue::NoneValue {})
    }

    /// Check if this is an interpolation
    fn is_interpolation(&self) -> bool {
        matches!(self, PyConfigValue::Interpolation { .. })
    }

    fn __repr__(&self) -> String {
        match self {
            PyConfigValue::NoneValue {} => "ConfigValue(None)".to_string(),
            PyConfigValue::Missing {} => "ConfigValue(???)".to_string(),
            PyConfigValue::Bool { value } => format!("ConfigValue({})", value),
            PyConfigValue::Int { value } => format!("ConfigValue({})", value),
            PyConfigValue::Float { value } => format!("ConfigValue({})", value),
            PyConfigValue::String { value } => format!("ConfigValue('{}')", value),
            PyConfigValue::List { items } => format!("ConfigValue([{} items])", items.len()),
            PyConfigValue::Dict { items } => format!("ConfigValue({{{} items}})", items.len()),
            PyConfigValue::Interpolation { expr } => format!("ConfigValue({})", expr),
        }
    }
}

/// Python-facing OmegaConf class with static methods
#[pyclass(name = "OmegaConf")]
pub struct PyOmegaConf;

#[pymethods]
impl PyOmegaConf {
    /// Create a new OmegaConf config from various types
    #[staticmethod]
    #[pyo3(signature = (obj=None))]
    fn create(obj: Option<&Bound<PyAny>>) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            match obj {
                None => {
                    let cfg = PyDictConfig::new(None)?;
                    Ok(cfg.into_pyobject(py)?.into_any().unbind())
                }
                Some(o) => {
                    if let Ok(dict) = o.cast::<PyDict>() {
                        let cfg = PyDictConfig::new(Some(dict))?;
                        Ok(cfg.into_pyobject(py)?.into_any().unbind())
                    } else if let Ok(list) = o.cast::<PyList>() {
                        let cfg = PyListConfig::new(Some(list))?;
                        Ok(cfg.into_pyobject(py)?.into_any().unbind())
                    } else if let Ok(s) = o.extract::<String>() {
                        // Parse YAML string
                        // For now, just create a dict with the string as a key
                        let mut map = HashMap::new();
                        map.insert(s, RustConfigValue::None);
                        let cfg = OmegaConf::create_dict(map);
                        let py_cfg = PyDictConfig {
                            inner: Arc::new(RwLock::new(cfg)),
                        };
                        Ok(py_cfg.into_pyobject(py)?.into_any().unbind())
                    } else {
                        Err(PyRuntimeError::new_err(format!(
                            "Cannot create OmegaConf from type: {}",
                            o.get_type().name()?
                        )))
                    }
                }
            }
        })
    }

    /// Check if a value is MISSING
    #[staticmethod]
    fn is_missing(cfg: &PyDictConfig, key: &str) -> PyResult<bool> {
        let inner = cfg.inner.read().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;
        Ok(OmegaConf::is_missing_dict(&inner, key))
    }

    /// Check if a value is an interpolation
    #[staticmethod]
    fn is_interpolation(cfg: &PyDictConfig, key: &str) -> PyResult<bool> {
        let inner = cfg.inner.read().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;
        Ok(OmegaConf::is_interpolation_dict(&inner, key))
    }

    /// Set the readonly flag
    #[staticmethod]
    fn set_readonly(cfg: &mut PyDictConfig, value: Option<bool>) -> PyResult<()> {
        let mut inner = cfg.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;
        OmegaConf::set_readonly_dict(&mut inner, value);
        Ok(())
    }

    /// Get the readonly flag
    #[staticmethod]
    fn is_readonly(cfg: &PyDictConfig) -> PyResult<Option<bool>> {
        let inner = cfg.inner.read().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;
        Ok(OmegaConf::is_readonly_dict(&inner))
    }

    /// Set the struct flag
    #[staticmethod]
    fn set_struct(cfg: &mut PyDictConfig, value: Option<bool>) -> PyResult<()> {
        let mut inner = cfg.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;
        OmegaConf::set_struct_dict(&mut inner, value);
        Ok(())
    }

    /// Get the struct flag
    #[staticmethod]
    fn is_struct(cfg: &PyDictConfig) -> PyResult<Option<bool>> {
        let inner = cfg.inner.read().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;
        Ok(OmegaConf::is_struct_dict(&inner))
    }

    /// Convert a config to a Python dict
    #[staticmethod]
    #[pyo3(signature = (cfg, resolve=false, throw_on_missing=false))]
    fn to_container(
        py: Python,
        cfg: &PyDictConfig,
        resolve: bool,
        throw_on_missing: bool,
    ) -> PyResult<Py<PyAny>> {
        let inner = cfg.inner.read().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;
        let container = OmegaConf::to_container_dict(&inner, resolve, throw_on_missing)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
        rust_config_value_to_py(&RustConfigValue::Dict(container), py)
    }

    /// Convert a config to YAML string
    #[staticmethod]
    #[pyo3(signature = (cfg, resolve=false, sort_keys=false))]
    fn to_yaml(cfg: &PyDictConfig, resolve: bool, sort_keys: bool) -> PyResult<String> {
        let inner = cfg.inner.read().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;
        OmegaConf::to_yaml_dict(&inner, resolve, sort_keys)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    /// Merge multiple configs
    #[staticmethod]
    #[pyo3(signature = (*configs))]
    fn merge(configs: &Bound<pyo3::types::PyTuple>) -> PyResult<PyDictConfig> {
        let mut dict_configs: Vec<DictConfig> = Vec::new();

        for config in configs.iter() {
            if let Ok(py_cfg) = config.extract::<PyRef<PyDictConfig>>() {
                let inner = py_cfg.inner.read().map_err(|e| {
                    PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
                })?;
                // Clone the inner config
                dict_configs.push((*inner).clone());
            } else if let Ok(dict) = config.cast::<PyDict>() {
                let mut map = HashMap::new();
                for (key, value) in dict.iter() {
                    let key_str: String = key.extract()?;
                    let config_value = py_to_config_value(&value)?;
                    map.insert(key_str, config_value);
                }
                dict_configs.push(OmegaConf::create_dict(map));
            } else {
                return Err(PyRuntimeError::new_err(format!(
                    "Cannot merge type: {}",
                    config.get_type().name()?
                )));
            }
        }

        let refs: Vec<&DictConfig> = dict_configs.iter().collect();
        let merged = OmegaConf::merge_dicts(refs, lerna::omegaconf::ListMergeMode::Replace)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;

        Ok(PyDictConfig {
            inner: Arc::new(RwLock::new(merged)),
        })
    }

    /// Check if an object is a config (DictConfig or ListConfig)
    #[staticmethod]
    fn is_config(obj: &Bound<PyAny>) -> bool {
        obj.is_instance_of::<PyDictConfig>() || obj.is_instance_of::<PyListConfig>()
    }

    /// Check if an object is a DictConfig
    #[staticmethod]
    fn is_dict(obj: &Bound<PyAny>) -> bool {
        obj.is_instance_of::<PyDictConfig>()
    }

    /// Check if an object is a ListConfig
    #[staticmethod]
    fn is_list(obj: &Bound<PyAny>) -> bool {
        obj.is_instance_of::<PyListConfig>()
    }

    /// Select a value using a dot-separated key path
    #[staticmethod]
    #[pyo3(signature = (cfg, key, default=None, throw_on_missing=false))]
    fn select(
        py: Python,
        cfg: &PyDictConfig,
        key: &str,
        default: Option<&Bound<PyAny>>,
        throw_on_missing: bool,
    ) -> PyResult<Py<PyAny>> {
        let inner = cfg.inner.read().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;

        match OmegaConf::select_dict(&inner, key, throw_on_missing) {
            Ok(Some(value)) => rust_config_value_to_py(&value, py),
            Ok(None) => match default {
                Some(d) => Ok(d.clone().unbind()),
                None => Ok(py.None()),
            },
            Err(e) => Err(PyRuntimeError::new_err(format!("{}", e))),
        }
    }

    /// Update a value in a config
    #[staticmethod]
    #[pyo3(signature = (cfg, key, value=None))]
    fn update(cfg: &mut PyDictConfig, key: &str, value: Option<&Bound<PyAny>>) -> PyResult<()> {
        let config_value = match value {
            Some(v) => py_to_config_value(v)?,
            None => RustConfigValue::None,
        };
        let mut inner = cfg.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;
        OmegaConf::update_dict(&mut inner, key, config_value)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    /// Resolve all interpolations in a config in-place
    /// This replaces ${...} references with their actual values
    #[staticmethod]
    fn resolve(cfg: &mut PyDictConfig) -> PyResult<()> {
        let mut inner = cfg.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;
        OmegaConf::resolve_dict(&mut inner)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    /// Load a YAML file and return a DictConfig
    #[staticmethod]
    fn load(_py: Python, path: &str) -> PyResult<PyDictConfig> {
        use std::path::Path;
        let path = Path::new(path);
        let dict_config = OmegaConf::load(path)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;

        Ok(PyDictConfig {
            inner: Arc::new(RwLock::new(dict_config)),
        })
    }

    /// Create a DictConfig from YAML string
    #[staticmethod]
    fn from_yaml(_py: Python, yaml: &str) -> PyResult<PyDictConfig> {
        let dict_config = OmegaConf::from_yaml(yaml)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;

        Ok(PyDictConfig {
            inner: Arc::new(RwLock::new(dict_config)),
        })
    }
}

/// Convert a Rust ConfigValue to a Python object
fn rust_config_value_to_py(value: &RustConfigValue, py: Python) -> PyResult<Py<PyAny>> {
    match value {
        RustConfigValue::None => Ok(py.None()),
        RustConfigValue::Missing => Ok("???".into_pyobject(py)?.into_any().unbind()),
        RustConfigValue::Bool(b) => Ok((*b).into_pyobject(py)?.to_owned().into_any().unbind()),
        RustConfigValue::Int(i) => Ok((*i).into_pyobject(py)?.to_owned().into_any().unbind()),
        RustConfigValue::Float(f) => Ok((*f).into_pyobject(py)?.to_owned().into_any().unbind()),
        RustConfigValue::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        RustConfigValue::Bytes(b) => Ok(b.clone().into_pyobject(py)?.into_any().unbind()),
        RustConfigValue::Interpolation(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        RustConfigValue::List(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(rust_config_value_to_py(item, py)?)?;
            }
            Ok(list.into_any().unbind())
        }
        RustConfigValue::Dict(map) => {
            let dict = PyDict::new(py);
            for (key, value) in map {
                dict.set_item(key, rust_config_value_to_py(value, py)?)?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}
