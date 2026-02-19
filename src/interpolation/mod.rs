// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Python bindings for interpolation module

use lerna::config::{ConfigDict, ConfigValue};
use lerna::interpolation::{
    find_interpolations, parse_interpolation, resolve_config, resolve_string,
    InterpolationType, ResolutionContext,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// Convert Python object to ConfigValue
fn py_to_config_value(obj: &Bound<'_, PyAny>) -> PyResult<ConfigValue> {
    if obj.is_none() {
        return Ok(ConfigValue::Null);
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
    if let Ok(s) = obj.extract::<String>() {
        return Ok(ConfigValue::String(s));
    }
    if let Ok(list) = obj.cast::<PyList>() {
        let items: PyResult<Vec<ConfigValue>> = list
            .iter()
            .map(|item| py_to_config_value(&item))
            .collect();
        return Ok(ConfigValue::List(items?));
    }
    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut result = ConfigDict::new();
        for (key, val) in dict.iter() {
            let k: String = key.extract()?;
            let v = py_to_config_value(&val)?;
            result.insert(k, v);
        }
        return Ok(ConfigValue::Dict(result));
    }
    Ok(ConfigValue::String(obj.str()?.to_string()))
}

/// Convert ConfigValue to Python object
fn config_value_to_py(py: Python<'_>, value: &ConfigValue) -> PyResult<Py<PyAny>> {
    match value {
        ConfigValue::Null => Ok(py.None()),
        ConfigValue::Bool(b) => Ok((*b).into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::Int(i) => Ok((*i).into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::Float(f) => Ok((*f).into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::String(s) => Ok(s.as_str().into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::List(items) => {
            let list = PyList::new(py, items.iter().map(|i| config_value_to_py(py, i).unwrap()))?;
            Ok(list.into())
        }
        ConfigValue::Dict(dict) => {
            let py_dict = PyDict::new(py);
            for (k, v) in dict.iter() {
                py_dict.set_item(k, config_value_to_py(py, v)?)?;
            }
            Ok(py_dict.into())
        }
        ConfigValue::Interpolation(s) => Ok(s.as_str().into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::Missing => Ok(py.None()),
    }
}

/// Find all interpolations in a string
/// Returns list of (start, end, interpolation_str) tuples
#[pyfunction]
fn find_interpolations_in_string(s: &str) -> Vec<(usize, usize, String)> {
    find_interpolations(s)
}

/// Parse interpolation type from string
#[pyfunction]
fn get_interpolation_type(s: &str) -> PyResult<String> {
    match parse_interpolation(s) {
        Ok(interp_type) => Ok(match interp_type {
            InterpolationType::Key(k) => format!("key:{}", k),
            InterpolationType::NestedKey(parts) => format!("nested:{}", parts.join(".")),
            InterpolationType::Env(var, default) => {
                if let Some(d) = default {
                    format!("env:{},{}", var, d)
                } else {
                    format!("env:{}", var)
                }
            }
            InterpolationType::Decode(expr) => format!("decode:{}", expr),
            InterpolationType::Create(expr) => format!("create:{}", expr),
            InterpolationType::Select(key, dict) => format!("select:{},{}", key, dict),
            InterpolationType::EscapedLiteral(s) => format!("escaped:{}", s),
            InterpolationType::Literal(s) => format!("literal:{}", s),
        }),
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e)),
    }
}

/// Resolve interpolations in a string using the given config context
#[pyfunction]
fn resolve_string_interpolations(
    _py: Python<'_>,
    s: &str,
    config: &Bound<'_, PyDict>,
) -> PyResult<String> {
    // Convert Python dict to ConfigDict
    let mut cfg = ConfigDict::new();
    for (key, val) in config.iter() {
        let k: String = key.extract()?;
        let v = py_to_config_value(&val)?;
        cfg.insert(k, v);
    }

    let ctx = ResolutionContext::new(cfg);
    resolve_string(s, &ctx).map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
}

/// Resolve all interpolations in a config dict
#[pyfunction]
fn resolve_config_interpolations(py: Python<'_>, config: &Bound<'_, PyDict>) -> PyResult<Py<PyAny>> {
    // Convert Python dict to ConfigDict
    let mut cfg = ConfigDict::new();
    for (key, val) in config.iter() {
        let k: String = key.extract()?;
        let v = py_to_config_value(&val)?;
        cfg.insert(k, v);
    }

    match resolve_config(cfg) {
        Ok(resolved) => config_value_to_py(py, &ConfigValue::Dict(resolved)),
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e)),
    }
}

/// Check if a string contains interpolations
#[pyfunction]
fn has_interpolations(s: &str) -> bool {
    !find_interpolations(s).is_empty()
}

/// Register this module
pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "interpolation")?;
    m.add_function(wrap_pyfunction!(find_interpolations_in_string, &m)?)?;
    m.add_function(wrap_pyfunction!(get_interpolation_type, &m)?)?;
    m.add_function(wrap_pyfunction!(resolve_string_interpolations, &m)?)?;
    m.add_function(wrap_pyfunction!(resolve_config_interpolations, &m)?)?;
    m.add_function(wrap_pyfunction!(has_interpolations, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
