// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Python bindings for job runner module

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::exceptions::{PyIOError, PyValueError};
use std::path::PathBuf;

use lerna::job_runner::{
    JobContext as RustJobContext,
    JobStatus as RustJobStatus,
    compute_output_dir as rust_compute_output_dir,
    create_output_dirs as rust_create_output_dirs,
    setup_job_environment as rust_setup_job_environment,
    serialize_config_to_yaml,
    save_config_file as rust_save_config_file,
    save_overrides_file as rust_save_overrides_file,
};
use lerna::config::value::{ConfigDict, ConfigValue};

/// Convert Python dict to ConfigDict
fn py_to_config_dict(py: Python, obj: &Bound<'_, PyAny>) -> PyResult<ConfigDict> {
    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut config_dict = ConfigDict::new();
        for (key, value) in dict.iter() {
            if let Ok(k) = key.extract::<String>() {
                config_dict.insert(k, py_to_config_value(py, &value)?);
            }
        }
        Ok(config_dict)
    } else {
        Err(PyValueError::new_err("Expected a dict"))
    }
}

/// Convert Python object to ConfigValue
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
        } else if s.contains("${") {
            Ok(ConfigValue::Interpolation(s))
        } else {
            Ok(ConfigValue::String(s))
        }
    } else if let Ok(list) = obj.cast::<PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(py_to_config_value(py, &item)?);
        }
        Ok(ConfigValue::List(items))
    } else if let Ok(_dict) = obj.cast::<PyDict>() {
        Ok(ConfigValue::Dict(py_to_config_dict(py, &obj.clone().into_any())?))
    } else {
        // Fallback: convert to string
        Ok(ConfigValue::String(obj.str()?.to_string()))
    }
}

/// Python wrapper for JobStatus
#[pyclass(name = "RustJobStatus")]
#[derive(Clone, Copy)]
pub struct PyJobStatus(RustJobStatus);

#[pymethods]
#[allow(non_snake_case)]
impl PyJobStatus {
    #[classattr]
    fn UNKNOWN() -> Self {
        Self(RustJobStatus::Unknown)
    }

    #[classattr]
    fn COMPLETED() -> Self {
        Self(RustJobStatus::Completed)
    }

    #[classattr]
    fn FAILED() -> Self {
        Self(RustJobStatus::Failed)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __repr__(&self) -> String {
        match self.0 {
            RustJobStatus::Unknown => "RustJobStatus.UNKNOWN".to_string(),
            RustJobStatus::Completed => "RustJobStatus.COMPLETED".to_string(),
            RustJobStatus::Failed => "RustJobStatus.FAILED".to_string(),
        }
    }
}

/// Python wrapper for JobContext
#[pyclass(name = "RustJobContext")]
#[derive(Clone)]
pub struct PyJobContext {
    inner: RustJobContext,
}

#[pymethods]
impl PyJobContext {
    #[new]
    #[pyo3(signature = (name, id, num))]
    fn new(name: &str, id: &str, num: usize) -> Self {
        Self {
            inner: RustJobContext::new(name, id, num),
        }
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    #[getter]
    fn num(&self) -> usize {
        self.inner.num
    }

    #[getter]
    fn output_dir(&self) -> String {
        self.inner.output_dir.to_string_lossy().to_string()
    }

    #[getter]
    fn working_dir(&self) -> String {
        self.inner.working_dir.to_string_lossy().to_string()
    }

    #[getter]
    fn original_cwd(&self) -> String {
        self.inner.original_cwd.to_string_lossy().to_string()
    }

    #[getter]
    fn chdir(&self) -> bool {
        self.inner.chdir
    }

    #[getter]
    fn overrides(&self) -> Vec<String> {
        self.inner.overrides.clone()
    }

    /// Set the output directory
    fn set_output_dir(&mut self, dir: &str) {
        self.inner.output_dir = PathBuf::from(dir);
    }

    /// Set chdir behavior
    fn set_chdir(&mut self, chdir: bool) {
        self.inner.chdir = chdir;
        if chdir {
            self.inner.working_dir = self.inner.output_dir.clone();
        } else {
            self.inner.working_dir = self.inner.original_cwd.clone();
        }
    }

    /// Set overrides
    fn set_overrides(&mut self, overrides: Vec<String>) {
        self.inner.overrides = overrides;
    }
}

/// Compute output directory for a job
#[pyfunction]
#[pyo3(signature = (job_dir_value, job_subdir_value=None))]
fn compute_output_dir(job_dir_value: &str, job_subdir_value: Option<&str>) -> String {
    rust_compute_output_dir(job_dir_value, job_subdir_value)
        .to_string_lossy()
        .to_string()
}

/// Create output directories
#[pyfunction]
#[pyo3(signature = (output_dir, subdir=None))]
fn create_output_dirs(output_dir: &str, subdir: Option<&str>) -> PyResult<String> {
    rust_create_output_dirs(&PathBuf::from(output_dir), subdir)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| PyIOError::new_err(e.to_string()))
}

/// Save a config dictionary to a YAML file
#[pyfunction]
fn save_config(py: Python, config: &Bound<'_, PyAny>, filename: &str, output_dir: &str) -> PyResult<String> {
    let config_dict = py_to_config_dict(py, config)?;
    rust_save_config_file(&config_dict, filename, &PathBuf::from(output_dir))
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| PyIOError::new_err(e.to_string()))
}

/// Save overrides to a YAML file
#[pyfunction]
fn save_overrides(overrides: Vec<String>, filename: &str, output_dir: &str) -> PyResult<String> {
    rust_save_overrides_file(&overrides, filename, &PathBuf::from(output_dir))
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| PyIOError::new_err(e.to_string()))
}

/// Setup job execution environment (create dirs, save configs)
#[pyfunction]
#[pyo3(signature = (output_dir, hydra_subdir, task_config, hydra_config, overrides))]
fn setup_job_environment(
    py: Python,
    output_dir: &str,
    hydra_subdir: Option<&str>,
    task_config: &Bound<'_, PyAny>,
    hydra_config: &Bound<'_, PyAny>,
    overrides: Vec<String>,
) -> PyResult<String> {
    let task_dict = py_to_config_dict(py, task_config)?;
    let hydra_dict = py_to_config_dict(py, hydra_config)?;

    rust_setup_job_environment(
        &PathBuf::from(output_dir),
        hydra_subdir,
        &task_dict,
        &hydra_dict,
        &overrides,
    )
    .map(|p| p.to_string_lossy().to_string())
    .map_err(|e| PyIOError::new_err(e.to_string()))
}

/// Serialize config dict to YAML string
#[pyfunction]
fn config_to_yaml(py: Python, config: &Bound<'_, PyAny>) -> PyResult<String> {
    let config_dict = py_to_config_dict(py, config)?;
    Ok(serialize_config_to_yaml(&config_dict))
}

/// Register this module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyJobStatus>()?;
    m.add_class::<PyJobContext>()?;
    m.add_function(wrap_pyfunction!(compute_output_dir, m)?)?;
    m.add_function(wrap_pyfunction!(create_output_dirs, m)?)?;
    m.add_function(wrap_pyfunction!(save_config, m)?)?;
    m.add_function(wrap_pyfunction!(save_overrides, m)?)?;
    m.add_function(wrap_pyfunction!(setup_job_environment, m)?)?;
    m.add_function(wrap_pyfunction!(config_to_yaml, m)?)?;
    Ok(())
}
