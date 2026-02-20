// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyDictConfig - Python bindings for DictConfig

use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use lerna::omegaconf::{
    AnyNode, ConfigValue, DictConfig, ListConfig, Node, NodeContent, NodeType, NodeValue, OmegaConf,
};

use crate::omegaconf::listconfig::PyListConfig;

/// Python-facing DictConfig class
#[pyclass(name = "DictConfig")]
#[derive(Debug)]
pub struct PyDictConfig {
    pub inner: Arc<RwLock<DictConfig>>,
}

#[pymethods]
impl PyDictConfig {
    /// Create a new empty DictConfig
    #[new]
    #[pyo3(signature = (content=None))]
    pub fn new(content: Option<&Bound<PyDict>>) -> PyResult<Self> {
        let cfg = match content {
            Some(dict) => {
                let mut map = HashMap::new();
                for (key, value) in dict.iter() {
                    let key_str: String = key.extract()?;
                    let config_value = py_to_config_value(&value)?;
                    map.insert(key_str, config_value);
                }
                OmegaConf::create_dict(map)
            }
            None => DictConfig::new(),
        };
        Ok(Self {
            inner: Arc::new(RwLock::new(cfg)),
        })
    }

    /// Get a value by key
    fn __getitem__(&self, key: &str) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            let cfg = self.inner.read().map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
            })?;
            match cfg.get(key) {
                Some(node) => node_arc_to_py(node, py),
                None => Err(PyKeyError::new_err(format!("Key not found: {}", key))),
            }
        })
    }

    /// Set a value by key
    fn __setitem__(&mut self, key: &str, value: &Bound<PyAny>) -> PyResult<()> {
        let config_value = py_to_config_value(value)?;
        let node = config_value_to_node(config_value);
        let mut cfg = self
            .inner
            .write()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        cfg.set(key, node)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    /// Delete a value by key
    fn __delitem__(&mut self, key: &str) -> PyResult<()> {
        let mut cfg = self
            .inner
            .write()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        let _ = cfg.remove(key);
        Ok(())
    }

    /// Check if a key exists
    fn __contains__(&self, key: &str) -> PyResult<bool> {
        let cfg = self
            .inner
            .read()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        Ok(cfg.contains_key(key))
    }

    /// Get the number of keys
    fn __len__(&self) -> PyResult<usize> {
        let cfg = self
            .inner
            .read()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        Ok(cfg.len_internal())
    }

    /// Get all keys
    fn keys(&self) -> PyResult<Vec<String>> {
        let cfg = self
            .inner
            .read()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        Ok(cfg.keys_iter().cloned().collect())
    }

    /// Get all values
    fn values(&self, py: Python) -> PyResult<Vec<Py<PyAny>>> {
        let cfg = self
            .inner
            .read()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        let mut result = Vec::new();
        for key in cfg.keys_iter() {
            if let Some(node) = cfg.get(key) {
                result.push(node_arc_to_py(node, py)?);
            }
        }
        Ok(result)
    }

    /// Get all items as (key, value) pairs
    fn items(&self, py: Python) -> PyResult<Vec<(String, Py<PyAny>)>> {
        let cfg = self
            .inner
            .read()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        let mut result = Vec::new();
        for key in cfg.keys_iter() {
            if let Some(node) = cfg.get(key) {
                let value = node_arc_to_py(node, py)?;
                result.push((key.clone(), value));
            }
        }
        Ok(result)
    }

    /// Get a value with a default
    #[pyo3(signature = (key, default=None))]
    fn get(&self, py: Python, key: &str, default: Option<&Bound<PyAny>>) -> PyResult<Py<PyAny>> {
        let cfg = self
            .inner
            .read()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        match cfg.get(key) {
            Some(node) => node_arc_to_py(node, py),
            None => match default {
                Some(d) => Ok(d.clone().unbind()),
                None => Ok(py.None()),
            },
        }
    }

    /// String representation
    fn __repr__(&self) -> PyResult<String> {
        let cfg = self
            .inner
            .read()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        let keys: Vec<String> = cfg.keys_iter().cloned().collect();
        Ok(format!("DictConfig({{{}}})", keys.join(", ")))
    }

    /// Get the value of a flag (struct, readonly)
    fn _get_flag(&self, flag: &str) -> PyResult<Option<bool>> {
        let cfg = self
            .inner
            .read()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        Ok(cfg.get_flag(flag))
    }

    /// Set the value of a flag (struct, readonly)
    fn _set_flag(&mut self, flag: &str, value: Option<bool>) -> PyResult<()> {
        let mut cfg = self
            .inner
            .write()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        cfg.set_flag(flag, value);
        Ok(())
    }

    /// Check if this is a struct (frozen schema)
    fn _is_struct(&self) -> PyResult<bool> {
        let cfg = self
            .inner
            .read()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        Ok(cfg.is_struct())
    }

    /// Check if this is readonly
    fn _is_readonly(&self) -> PyResult<bool> {
        let cfg = self
            .inner
            .read()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e)))?;
        Ok(cfg.is_readonly())
    }
}

/// Convert a Python object to a ConfigValue
pub fn py_to_config_value(obj: &Bound<PyAny>) -> PyResult<ConfigValue> {
    if obj.is_none() {
        return Ok(ConfigValue::None);
    }

    // Check for MISSING
    if let Ok(s) = obj.extract::<String>() {
        if s == "???" {
            return Ok(ConfigValue::Missing);
        } else if s.starts_with("${") && s.ends_with("}") {
            return Ok(ConfigValue::Interpolation(s));
        }
        return Ok(ConfigValue::String(s));
    }

    if let Ok(b) = obj.extract::<bool>() {
        return Ok(ConfigValue::Bool(b));
    }

    if let Ok(i) = obj.extract::<i64>() {
        return Ok(ConfigValue::Int(i));
    }

    if let Ok(f) = obj.extract::<f64>() {
        return Ok(ConfigValue::Float(f));
    }

    if let Ok(list) = obj.cast::<PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(py_to_config_value(&item)?);
        }
        return Ok(ConfigValue::List(items));
    }

    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut map = HashMap::new();
        for (key, value) in dict.iter() {
            let key_str: String = key.extract()?;
            map.insert(key_str, py_to_config_value(&value)?);
        }
        return Ok(ConfigValue::Dict(map));
    }

    Err(PyTypeError::new_err(format!(
        "Unsupported type for config value: {}",
        obj.get_type().name()?
    )))
}

/// Convert an Arc<RwLock<dyn Node>> to a Python object
/// This handles both value nodes and container nodes (DictConfig, ListConfig)
pub fn node_arc_to_py(node: Arc<RwLock<dyn Node>>, py: Python) -> PyResult<Py<PyAny>> {
    let guard = node
        .read()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock node: {}", e)))?;

    // Check the node type first
    match guard.node_type() {
        NodeType::Dict => {
            // Use as_any() to downcast to DictConfig
            if let Some(dict_config) = guard.as_any().downcast_ref::<DictConfig>() {
                let cloned = dict_config.clone();
                drop(guard); // Release the read lock
                let py_dict = PyDictConfig {
                    inner: Arc::new(RwLock::new(cloned)),
                };
                Ok(py_dict.into_pyobject(py)?.into_any().unbind())
            } else {
                Err(PyRuntimeError::new_err("Failed to downcast to DictConfig"))
            }
        }
        NodeType::List => {
            // Use as_any() to downcast to ListConfig
            if let Some(list_config) = guard.as_any().downcast_ref::<ListConfig>() {
                let cloned = list_config.clone();
                drop(guard); // Release the read lock
                let py_list = PyListConfig {
                    inner: Arc::new(RwLock::new(cloned)),
                };
                Ok(py_list.into_pyobject(py)?.into_any().unbind())
            } else {
                Err(PyRuntimeError::new_err("Failed to downcast to ListConfig"))
            }
        }
        NodeType::Value => {
            // Regular value node
            let content = guard.content();
            match content {
                NodeContent::None => Ok(py.None()),
                NodeContent::Missing => Ok("???".into_pyobject(py)?.into_any().unbind()),
                NodeContent::Interpolation(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
                NodeContent::Value(v) => match v {
                    NodeValue::Bool(b) => {
                        Ok((*b).into_pyobject(py)?.to_owned().into_any().unbind())
                    }
                    NodeValue::Int(i) => Ok((*i).into_pyobject(py)?.to_owned().into_any().unbind()),
                    NodeValue::Float(f) => {
                        Ok((*f).into_pyobject(py)?.to_owned().into_any().unbind())
                    }
                    NodeValue::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
                    NodeValue::Bytes(b) => Ok(b.clone().into_pyobject(py)?.into_any().unbind()),
                },
            }
        }
    }
}

/// Convert a ConfigValue to a Node
fn config_value_to_node(value: ConfigValue) -> Arc<RwLock<dyn Node>> {
    match value {
        ConfigValue::None => Arc::new(RwLock::new(AnyNode::new(None))),
        ConfigValue::Missing => Arc::new(RwLock::new(AnyNode::missing())),
        ConfigValue::Bool(v) => Arc::new(RwLock::new(AnyNode::with_value(NodeValue::Bool(v)))),
        ConfigValue::Int(v) => Arc::new(RwLock::new(AnyNode::with_value(NodeValue::Int(v)))),
        ConfigValue::Float(v) => Arc::new(RwLock::new(AnyNode::with_value(NodeValue::Float(v)))),
        ConfigValue::String(v) => Arc::new(RwLock::new(AnyNode::with_value(NodeValue::String(v)))),
        ConfigValue::Bytes(v) => Arc::new(RwLock::new(AnyNode::with_value(NodeValue::Bytes(v)))),
        ConfigValue::List(v) => {
            let child = OmegaConf::create_list(v);
            Arc::new(RwLock::new(child))
        }
        ConfigValue::Dict(v) => {
            let child = OmegaConf::create_dict(v);
            Arc::new(RwLock::new(child))
        }
        ConfigValue::Interpolation(v) => Arc::new(RwLock::new(AnyNode::interpolation(v))),
    }
}
