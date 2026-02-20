// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyO3 bindings for core types

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use lerna::ObjectType as RustObjectType;

/// Python-exposed ObjectType enum
#[pyclass(name = "ObjectType", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyObjectType {
    #[pyo3(name = "NOT_FOUND")]
    NotFound = 0,
    #[pyo3(name = "CONFIG")]
    Config = 1,
    #[pyo3(name = "GROUP")]
    Group = 2,
}

#[pymethods]
impl PyObjectType {
    #[new]
    fn new(value: u8) -> PyResult<Self> {
        match value {
            0 => Ok(PyObjectType::NotFound),
            1 => Ok(PyObjectType::Config),
            2 => Ok(PyObjectType::Group),
            _ => Err(PyValueError::new_err(format!(
                "Invalid ObjectType value: {}",
                value
            ))),
        }
    }

    /// Create a CONFIG ObjectType
    #[staticmethod]
    fn config() -> Self {
        PyObjectType::Config
    }

    /// Create a GROUP ObjectType
    #[staticmethod]
    fn group() -> Self {
        PyObjectType::Group
    }

    /// Create a NOT_FOUND ObjectType
    #[staticmethod]
    fn not_found() -> Self {
        PyObjectType::NotFound
    }

    fn is_found(&self) -> bool {
        !matches!(self, PyObjectType::NotFound)
    }

    fn is_config(&self) -> bool {
        matches!(self, PyObjectType::Config)
    }

    fn is_group(&self) -> bool {
        matches!(self, PyObjectType::Group)
    }

    fn __str__(&self) -> &'static str {
        match self {
            PyObjectType::NotFound => "NOT_FOUND",
            PyObjectType::Config => "CONFIG",
            PyObjectType::Group => "GROUP",
        }
    }

    fn __repr__(&self) -> String {
        format!("ObjectType.{}", self.__str__())
    }
}

impl From<RustObjectType> for PyObjectType {
    fn from(ot: RustObjectType) -> Self {
        match ot {
            RustObjectType::NotFound => PyObjectType::NotFound,
            RustObjectType::Config => PyObjectType::Config,
            RustObjectType::Group => PyObjectType::Group,
        }
    }
}

impl From<PyObjectType> for RustObjectType {
    fn from(ot: PyObjectType) -> Self {
        match ot {
            PyObjectType::NotFound => RustObjectType::NotFound,
            PyObjectType::Config => RustObjectType::Config,
            PyObjectType::Group => RustObjectType::Group,
        }
    }
}
