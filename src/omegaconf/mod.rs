// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! OmegaConf Python bindings
//!
//! This module provides Python bindings for the Rust OmegaConf implementation.

mod context;
mod dictconfig;
mod listconfig;
mod omegaconf;

pub use context::{PyOpenDict, PyReadWrite, PyFlagOverride};
pub use dictconfig::PyDictConfig;
pub use listconfig::PyListConfig;
pub use omegaconf::{PyConfigValue, PyOmegaConf};

use pyo3::prelude::*;
use pyo3::create_exception;

// Create MissingMandatoryValue exception
create_exception!(omegaconf, MissingMandatoryValue, pyo3::exceptions::PyException);

/// Container base class for DictConfig and ListConfig
/// This is used for type checking via isinstance(obj, Container)
#[pyclass(name = "Container", subclass)]
pub struct PyContainer {}

#[pymethods]
impl PyContainer {
    #[new]
    fn new() -> Self {
        Self {}
    }
}

/// Register OmegaConf types and functions with the Python module
pub fn register(m: &Bound<PyModule>) -> PyResult<()> {
    // Create omegaconf submodule
    let submod = PyModule::new(m.py(), "omegaconf")?;

    // Add classes
    submod.add_class::<PyContainer>()?;
    submod.add_class::<PyDictConfig>()?;
    submod.add_class::<PyListConfig>()?;
    submod.add_class::<PyOmegaConf>()?;
    submod.add_class::<PyConfigValue>()?;

    // Add context managers
    submod.add_class::<PyOpenDict>()?;
    submod.add_class::<PyReadWrite>()?;
    submod.add_class::<PyFlagOverride>()?;

    // Add constants
    submod.add("MISSING", "???")?;

    // Add exceptions
    submod.add("MissingMandatoryValue", m.py().get_type::<MissingMandatoryValue>())?;

    m.add_submodule(&submod)?;

    // Also register at top level for convenience
    m.add_class::<PyContainer>()?;
    m.add_class::<PyDictConfig>()?;
    m.add_class::<PyListConfig>()?;
    m.add_class::<PyOmegaConf>()?;

    // Add context managers at top level
    m.add_class::<PyOpenDict>()?;
    m.add_class::<PyReadWrite>()?;
    m.add_class::<PyFlagOverride>()?;

    // Add MISSING at top level
    m.add("MISSING", "???")?;

    // Add exceptions at top level
    m.add("MissingMandatoryValue", m.py().get_type::<MissingMandatoryValue>())?;

    Ok(())
}
