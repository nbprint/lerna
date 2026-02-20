// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Python bindings for config merge module

use lerna::config::{ConfigDict, ConfigValue};
use lerna::merge::{
    apply_deletions, apply_override, collect_keys, diff_keys, get_nested, merge_configs,
    merge_dicts,
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
        // Check for MISSING marker
        if s == "???" {
            return Ok(ConfigValue::Missing);
        }
        return Ok(ConfigValue::String(s));
    }
    if let Ok(list) = obj.cast::<PyList>() {
        let items: PyResult<Vec<ConfigValue>> =
            list.iter().map(|item| py_to_config_value(&item)).collect();
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
        ConfigValue::Interpolation(s) => {
            Ok(s.as_str().into_pyobject(py)?.to_owned().into_any().unbind())
        }
        ConfigValue::Missing => Ok("???".into_pyobject(py)?.to_owned().into_any().unbind()),
    }
}

/// Merge two config dictionaries
#[pyfunction]
fn merge_config_dicts(
    py: Python<'_>,
    base: &Bound<'_, PyDict>,
    other: &Bound<'_, PyDict>,
) -> PyResult<Py<PyAny>> {
    let base_val = py_to_config_value(&base.as_any())?;
    let other_val = py_to_config_value(&other.as_any())?;

    let mut base_dict = match base_val {
        ConfigValue::Dict(d) => d,
        _ => return Err(pyo3::exceptions::PyValueError::new_err("Expected dict")),
    };

    let other_dict = match other_val {
        ConfigValue::Dict(d) => d,
        _ => return Err(pyo3::exceptions::PyValueError::new_err("Expected dict")),
    };

    merge_dicts(&mut base_dict, &other_dict);
    config_value_to_py(py, &ConfigValue::Dict(base_dict))
}

/// Merge multiple config dictionaries in order
#[pyfunction]
fn merge_multiple_configs(py: Python<'_>, configs: &Bound<'_, PyList>) -> PyResult<Py<PyAny>> {
    let mut cfg_list = Vec::new();

    for item in configs.iter() {
        let dict = item.cast::<PyDict>()?;
        let val = py_to_config_value(&dict.as_any())?;
        if let ConfigValue::Dict(d) = val {
            cfg_list.push(d);
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "All items must be dicts",
            ));
        }
    }

    let result = merge_configs(&cfg_list);
    config_value_to_py(py, &ConfigValue::Dict(result))
}

/// Apply deletions to a config
#[pyfunction]
fn apply_config_deletions(
    py: Python<'_>,
    config: &Bound<'_, PyDict>,
    deletions: Vec<String>,
) -> PyResult<Py<PyAny>> {
    let val = py_to_config_value(&config.as_any())?;
    let mut dict = match val {
        ConfigValue::Dict(d) => d,
        _ => return Err(pyo3::exceptions::PyValueError::new_err("Expected dict")),
    };

    apply_deletions(&mut dict, &deletions);
    config_value_to_py(py, &ConfigValue::Dict(dict))
}

/// Apply an override to a config at a specific path
#[pyfunction]
fn apply_config_override(
    py: Python<'_>,
    config: &Bound<'_, PyDict>,
    path: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let cfg_val = py_to_config_value(&config.as_any())?;
    let mut dict = match cfg_val {
        ConfigValue::Dict(d) => d,
        _ => return Err(pyo3::exceptions::PyValueError::new_err("Expected dict")),
    };

    let override_val = py_to_config_value(value)?;
    apply_override(&mut dict, path, override_val);
    config_value_to_py(py, &ConfigValue::Dict(dict))
}

/// Get a value from a nested path
#[pyfunction]
fn get_nested_value(py: Python<'_>, config: &Bound<'_, PyDict>, path: &str) -> PyResult<Py<PyAny>> {
    let val = py_to_config_value(&config.as_any())?;
    let dict = match val {
        ConfigValue::Dict(d) => d,
        _ => return Err(pyo3::exceptions::PyValueError::new_err("Expected dict")),
    };

    match get_nested(&dict, path) {
        Some(v) => config_value_to_py(py, &v),
        None => Ok(py.None()),
    }
}

/// Collect all keys from a config (flattened with dot notation)
#[pyfunction]
fn get_all_keys(config: &Bound<'_, PyDict>) -> PyResult<Vec<String>> {
    let val = py_to_config_value(&config.as_any())?;
    let dict = match val {
        ConfigValue::Dict(d) => d,
        _ => return Err(pyo3::exceptions::PyValueError::new_err("Expected dict")),
    };

    Ok(collect_keys(&dict, ""))
}

/// Find keys that differ between two configs
#[pyfunction]
fn get_diff_keys(
    config1: &Bound<'_, PyDict>,
    config2: &Bound<'_, PyDict>,
) -> PyResult<Vec<String>> {
    let val1 = py_to_config_value(&config1.as_any())?;
    let val2 = py_to_config_value(&config2.as_any())?;

    let dict1 = match val1 {
        ConfigValue::Dict(d) => d,
        _ => return Err(pyo3::exceptions::PyValueError::new_err("Expected dict")),
    };

    let dict2 = match val2 {
        ConfigValue::Dict(d) => d,
        _ => return Err(pyo3::exceptions::PyValueError::new_err("Expected dict")),
    };

    Ok(diff_keys(&dict1, &dict2))
}

/// Register this module
pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "merge")?;
    m.add_function(wrap_pyfunction!(merge_config_dicts, &m)?)?;
    m.add_function(wrap_pyfunction!(merge_multiple_configs, &m)?)?;
    m.add_function(wrap_pyfunction!(apply_config_deletions, &m)?)?;
    m.add_function(wrap_pyfunction!(apply_config_override, &m)?)?;
    m.add_function(wrap_pyfunction!(get_nested_value, &m)?)?;
    m.add_function(wrap_pyfunction!(get_all_keys, &m)?)?;
    m.add_function(wrap_pyfunction!(get_diff_keys, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
