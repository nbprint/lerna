// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyListConfig - Python bindings for ListConfig

use pyo3::exceptions::{PyIndexError, PyRuntimeError};
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::sync::{Arc, RwLock};

use lerna::omegaconf::{
    AnyNode, ListConfig, Node, NodeValue, OmegaConf, ConfigValue,
};

use super::dictconfig::{py_to_config_value, node_arc_to_py};

/// Python-facing ListConfig class
#[pyclass(name = "ListConfig")]
#[derive(Debug)]
pub struct PyListConfig {
    pub inner: Arc<RwLock<ListConfig>>,
}

#[pymethods]
impl PyListConfig {
    /// Create a new empty ListConfig
    #[new]
    #[pyo3(signature = (content=None))]
    pub fn new(content: Option<&Bound<PyList>>) -> PyResult<Self> {
        let cfg = match content {
            Some(list) => {
                let mut items = Vec::new();
                for item in list.iter() {
                    items.push(py_to_config_value(&item)?);
                }
                OmegaConf::create_list(items)
            }
            None => ListConfig::new(),
        };
        Ok(Self {
            inner: Arc::new(RwLock::new(cfg)),
        })
    }

    /// Get a value by index
    fn __getitem__(&self, index: isize) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            let cfg = self.inner.read().map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to lock ListConfig: {}", e))
            })?;

            let len = cfg.len() as isize;
            let actual_index = if index < 0 {
                len + index
            } else {
                index
            };

            if actual_index < 0 || actual_index >= len {
                return Err(PyIndexError::new_err("list index out of range"));
            }

            match cfg.get(actual_index as usize) {
                Some(node) => node_arc_to_py(node, py),
                None => Err(PyIndexError::new_err("list index out of range")),
            }
        })
    }

    /// Set a value by index
    fn __setitem__(&mut self, index: isize, value: &Bound<PyAny>) -> PyResult<()> {
        let config_value = py_to_config_value(value)?;
        let node = config_value_to_node(config_value);
        let mut cfg = self.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock ListConfig: {}", e))
        })?;

        let len = cfg.len() as isize;
        let actual_index = if index < 0 {
            len + index
        } else {
            index
        };

        if actual_index < 0 || actual_index >= len {
            return Err(PyIndexError::new_err("list index out of range"));
        }

        cfg.set(actual_index as usize, node)
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    /// Delete a value by index
    fn __delitem__(&mut self, index: isize) -> PyResult<()> {
        let mut cfg = self.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock ListConfig: {}", e))
        })?;

        let len = cfg.len() as isize;
        let actual_index = if index < 0 {
            len + index
        } else {
            index
        };

        if actual_index < 0 || actual_index >= len {
            return Err(PyIndexError::new_err("list index out of range"));
        }

        cfg.remove(actual_index as usize)
            .map(|_| ()) // Discard the removed value
            .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    /// Get the length
    fn __len__(&self) -> PyResult<usize> {
        let cfg = self.inner.read().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock ListConfig: {}", e))
        })?;
        Ok(cfg.len())
    }

    /// Append a value
    fn append(&mut self, value: &Bound<PyAny>) -> PyResult<()> {
        let config_value = py_to_config_value(value)?;
        let node = config_value_to_node(config_value);
        let mut cfg = self.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock ListConfig: {}", e))
        })?;
        cfg.append(node).map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    /// Insert a value at index
    fn insert(&mut self, index: usize, value: &Bound<PyAny>) -> PyResult<()> {
        let config_value = py_to_config_value(value)?;
        let node = config_value_to_node(config_value);
        let mut cfg = self.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock ListConfig: {}", e))
        })?;
        cfg.insert(index, node).map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    /// Pop and return the last element
    fn pop(&mut self) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            let mut cfg = self.inner.write().map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to lock ListConfig: {}", e))
            })?;
            match cfg.pop() {
                Ok(Some(node)) => node_arc_to_py(node, py),
                Ok(None) => Err(PyIndexError::new_err("pop from empty list")),
                Err(e) => Err(PyRuntimeError::new_err(format!("{}", e))),
            }
        })
    }

    /// Clear the list
    fn clear(&mut self) -> PyResult<()> {
        let mut cfg = self.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock ListConfig: {}", e))
        })?;
        cfg.clear().map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
    }

    /// String representation
    fn __repr__(&self) -> PyResult<String> {
        let cfg = self.inner.read().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock ListConfig: {}", e))
        })?;
        Ok(format!("ListConfig([... {} items ...])", cfg.len()))
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
