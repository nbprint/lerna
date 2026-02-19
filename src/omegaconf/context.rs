// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Context managers for OmegaConf flag manipulation
//!
//! These provide Python context managers that temporarily change flags:
//! - open_dict: Temporarily disable struct flag
//! - read_write: Temporarily disable readonly flag
//! - flag_override: Temporarily override any flag

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use lerna::omegaconf::Node;

use super::dictconfig::PyDictConfig;

/// Context manager that temporarily disables the struct flag
///
/// Usage:
///     with open_dict(cfg):
///         cfg.new_key = "new_value"
#[pyclass(name = "open_dict")]
pub struct PyOpenDict {
    config: Py<PyDictConfig>,
    previous_struct: Option<bool>,
}

#[pymethods]
impl PyOpenDict {
    #[new]
    fn new(config: Py<PyDictConfig>) -> Self {
        Self {
            config,
            previous_struct: None,
        }
    }

    fn __enter__<'py>(mut slf: PyRefMut<'py, Self>, py: Python<'py>) -> PyResult<Py<PyDictConfig>> {
        // Get the current struct flag and set it to false in one scope
        {
            let config_ref = slf.config.bind(py);
            let config = config_ref.borrow();

            let mut inner = config.inner.write().map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
            })?;

            slf.previous_struct = inner.get_flag("struct");
            inner.set_flag("struct", Some(false));
        }

        Ok(slf.config.clone_ref(py))
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &mut self,
        py: Python,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_val: Option<&Bound<PyAny>>,
        _exc_tb: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        // Restore the previous struct flag
        let config_ref = self.config.bind(py);
        let config = config_ref.borrow_mut();

        let mut inner = config.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;

        inner.set_flag("struct", self.previous_struct);

        Ok(false)  // Don't suppress exceptions
    }
}

/// Context manager that temporarily disables the readonly flag
///
/// Usage:
///     with read_write(cfg):
///         cfg.key = "new_value"
#[pyclass(name = "read_write")]
pub struct PyReadWrite {
    config: Py<PyDictConfig>,
    previous_readonly: Option<bool>,
}

#[pymethods]
impl PyReadWrite {
    #[new]
    fn new(config: Py<PyDictConfig>) -> Self {
        Self {
            config,
            previous_readonly: None,
        }
    }

    fn __enter__<'py>(mut slf: PyRefMut<'py, Self>, py: Python<'py>) -> PyResult<Py<PyDictConfig>> {
        // Get the current readonly flag and set it to false in one scope
        {
            let config_ref = slf.config.bind(py);
            let config = config_ref.borrow();

            let mut inner = config.inner.write().map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
            })?;

            slf.previous_readonly = inner.get_flag("readonly");
            inner.set_flag("readonly", Some(false));
        }

        Ok(slf.config.clone_ref(py))
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &mut self,
        py: Python,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_val: Option<&Bound<PyAny>>,
        _exc_tb: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        // Restore the previous readonly flag
        let config_ref = self.config.bind(py);
        let config = config_ref.borrow_mut();

        let mut inner = config.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;

        inner.set_flag("readonly", self.previous_readonly);

        Ok(false)  // Don't suppress exceptions
    }
}

/// Context manager for overriding any flag temporarily
///
/// Usage:
///     with flag_override(cfg, "struct", True):
///         # cfg has struct=True here
#[pyclass(name = "flag_override")]
pub struct PyFlagOverride {
    config: Py<PyDictConfig>,
    flag_name: String,
    previous_value: Option<bool>,
}

#[pymethods]
impl PyFlagOverride {
    #[new]
    fn new(config: Py<PyDictConfig>, flag_name: String, new_value: Option<bool>, py: Python) -> PyResult<Self> {
        // Get and store the current flag value
        let config_ref = config.bind(py);
        let config_borrow = config_ref.borrow();

        let inner = config_borrow.inner.read().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;

        let previous_value = inner.get_flag(&flag_name);
        drop(inner);
        drop(config_borrow);

        // Set the new value
        let config_mut = config_ref.borrow_mut();
        let mut inner = config_mut.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;
        inner.set_flag(&flag_name, new_value);

        Ok(Self {
            config,
            flag_name,
            previous_value,
        })
    }

    fn __enter__<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> Py<PyDictConfig> {
        slf.config.clone_ref(py)
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &mut self,
        py: Python,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_val: Option<&Bound<PyAny>>,
        _exc_tb: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        // Restore the previous flag value
        let config_ref = self.config.bind(py);
        let config = config_ref.borrow_mut();

        let mut inner = config.inner.write().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to lock DictConfig: {}", e))
        })?;

        inner.set_flag(&self.flag_name, self.previous_value);

        Ok(false)  // Don't suppress exceptions
    }
}
