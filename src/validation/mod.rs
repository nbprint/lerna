// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Python bindings for config validation

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use lerna::validation::{ConfigSchema, TypeSpec};
use lerna::config::value::{ConfigDict, ConfigValue};

/// Python wrapper for TypeSpec
#[pyclass(name = "TypeSpec")]
#[derive(Clone)]
pub struct PyTypeSpec {
    inner: TypeSpec,
}

#[pymethods]
impl PyTypeSpec {
    /// Parse a type specification from a string
    #[staticmethod]
    fn parse(s: &str) -> Option<Self> {
        TypeSpec::parse(s).map(|t| Self { inner: t })
    }

    /// Create an Int type spec
    #[staticmethod]
    fn int() -> Self {
        Self { inner: TypeSpec::Int }
    }

    /// Create a String type spec
    #[staticmethod]
    fn string() -> Self {
        Self { inner: TypeSpec::String }
    }

    /// Create a Bool type spec
    #[staticmethod]
    fn bool() -> Self {
        Self { inner: TypeSpec::Bool }
    }

    /// Create a Float type spec
    #[staticmethod]
    fn float() -> Self {
        Self { inner: TypeSpec::Float }
    }

    /// Create an Any type spec
    #[staticmethod]
    fn any() -> Self {
        Self { inner: TypeSpec::Any }
    }

    /// Create a List type spec
    #[staticmethod]
    fn list(inner: &PyTypeSpec) -> Self {
        Self {
            inner: TypeSpec::List(Box::new(inner.inner.clone())),
        }
    }

    /// Create an Optional type spec
    #[staticmethod]
    fn optional(inner: &PyTypeSpec) -> Self {
        Self {
            inner: TypeSpec::Optional(Box::new(inner.inner.clone())),
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Python wrapper for ValidationError
#[pyclass(name = "ValidationError")]
#[derive(Clone)]
pub struct PyValidationError {
    #[pyo3(get)]
    pub path: String,
    #[pyo3(get)]
    pub message: String,
}

#[pymethods]
impl PyValidationError {
    fn __repr__(&self) -> String {
        format!("{}: {}", self.path, self.message)
    }

    fn __str__(&self) -> String {
        format!("{}: {}", self.path, self.message)
    }
}

/// Python wrapper for ConfigSchema
#[pyclass(name = "ConfigSchema")]
#[derive(Clone)]
pub struct PyConfigSchema {
    inner: ConfigSchema,
}

#[pymethods]
impl PyConfigSchema {
    #[new]
    fn new() -> Self {
        Self {
            inner: ConfigSchema::new(),
        }
    }

    /// Add a required field
    fn required(&mut self, name: &str, type_spec: &PyTypeSpec) {
        self.inner.fields.insert(
            name.to_string(),
            (type_spec.inner.clone(), true, None),
        );
    }

    /// Add an optional field with a default value
    fn optional(&mut self, name: &str, type_spec: &PyTypeSpec, default: &Bound<'_, PyAny>) -> PyResult<()> {
        let default_value = py_to_config_value(default)?;
        self.inner.fields.insert(
            name.to_string(),
            (type_spec.inner.clone(), false, Some(default_value)),
        );
        Ok(())
    }

    /// Validate a config dictionary
    fn validate(&self, config: &Bound<'_, PyDict>) -> PyResult<Vec<PyValidationError>> {
        let config_dict = py_dict_to_config_dict(config)?;
        match self.inner.validate(&config_dict) {
            Ok(()) => Ok(vec![]),
            Err(errors) => Ok(errors
                .into_iter()
                .map(|e| PyValidationError {
                    path: e.path,
                    message: e.message,
                })
                .collect()),
        }
    }

    /// Check if config is valid (no errors)
    fn is_valid(&self, config: &Bound<'_, PyDict>) -> PyResult<bool> {
        let config_dict = py_dict_to_config_dict(config)?;
        Ok(self.inner.validate(&config_dict).is_ok())
    }

    fn __repr__(&self) -> String {
        format!("ConfigSchema({} fields)", self.inner.fields.len())
    }
}

/// Convert a Python object to ConfigValue
fn py_to_config_value(obj: &Bound<'_, PyAny>) -> PyResult<ConfigValue> {
    if obj.is_none() {
        Ok(ConfigValue::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(ConfigValue::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(ConfigValue::Int(i))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(ConfigValue::Float(f))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(ConfigValue::String(s))
    } else if let Ok(list) = obj.cast::<PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(py_to_config_value(&item)?);
        }
        Ok(ConfigValue::List(items))
    } else if let Ok(dict) = obj.cast::<PyDict>() {
        let config_dict = py_dict_to_config_dict(dict)?;
        Ok(ConfigValue::Dict(config_dict))
    } else {
        Ok(ConfigValue::String(obj.str()?.to_string()))
    }
}

/// Convert a Python dict to ConfigDict
fn py_dict_to_config_dict(dict: &Bound<'_, PyDict>) -> PyResult<ConfigDict> {
    let mut config_dict = ConfigDict::new();
    for (key, value) in dict.iter() {
        if let Ok(k) = key.extract::<String>() {
            config_dict.insert(k, py_to_config_value(&value)?);
        }
    }
    Ok(config_dict)
}

/// Validate a config against a type spec
#[pyfunction]
fn validate_type(value: &Bound<'_, PyAny>, type_spec: &PyTypeSpec) -> PyResult<bool> {
    let config_value = py_to_config_value(value)?;
    Ok(type_spec.inner.matches(&config_value))
}

/// Register the module
pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "validation")?;
    m.add_class::<PyTypeSpec>()?;
    m.add_class::<PyValidationError>()?;
    m.add_class::<PyConfigSchema>()?;
    m.add_function(wrap_pyfunction!(validate_type, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
