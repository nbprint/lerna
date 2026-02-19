//! PyO3 bindings for Rust Launcher trait
//!
//! This module provides:
//! - `PyLauncherWrapper` - wraps Python Launcher to implement Rust trait
//! - `PyBasicLauncher` - exposes Rust BasicLauncher to Python
//! - `PyLauncherManager` - manages launchers from Python

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::sync::Arc;

use lerna::callback::JobReturn;
use lerna::config::value::{ConfigDict, ConfigValue};
use lerna::launcher::{BasicLauncher, JobOverrideBatch, Launcher, LauncherError, LauncherManager};

use crate::callback::PyJobReturn;

/// Convert Python dict to ConfigDict
fn py_dict_to_config_dict(dict: &Bound<'_, PyDict>) -> PyResult<ConfigDict> {
    let mut config = ConfigDict::new();
    for (key, value) in dict.iter() {
        let k: String = key.extract()?;
        let v = py_to_config_value(&value)?;
        config.insert(k, v);
    }
    Ok(config)
}

/// Convert Python object to ConfigValue
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
        let items: PyResult<Vec<_>> = list.iter().map(|item| py_to_config_value(&item)).collect();
        Ok(ConfigValue::List(items?))
    } else if let Ok(dict) = obj.cast::<PyDict>() {
        let mut config_dict = ConfigDict::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            config_dict.insert(key, py_to_config_value(&v)?);
        }
        Ok(ConfigValue::Dict(config_dict))
    } else {
        Ok(ConfigValue::String(obj.str()?.to_string()))
    }
}

/// Convert job overrides from Python
fn py_to_job_overrides(overrides: &Bound<'_, PyList>) -> PyResult<JobOverrideBatch> {
    let mut batch = Vec::new();
    for job_overrides in overrides.iter() {
        let job_list: Vec<String> = job_overrides.extract()?;
        batch.push(job_list);
    }
    Ok(batch)
}

/// Wrapper that allows Python Launcher to implement Rust Launcher trait
pub struct PyLauncherWrapper {
    py_launcher: Py<PyAny>,
    name: String,
}

impl PyLauncherWrapper {
    pub fn new(py_launcher: Py<PyAny>) -> Self {
        let name = Python::attach(|py| {
            py_launcher.bind(py)
                .getattr("__class__")
                .and_then(|c| c.getattr("__name__"))
                .and_then(|n| n.extract::<String>())
                .unwrap_or_else(|_| "PyLauncher".to_string())
        });
        Self { py_launcher, name }
    }
}

impl std::fmt::Debug for PyLauncherWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PyLauncherWrapper")
            .field("name", &self.name)
            .finish()
    }
}

impl Launcher for PyLauncherWrapper {
    fn setup(
        &mut self,
        _config: &ConfigDict,
        _task_name: &str,
    ) -> Result<(), LauncherError> {
        // Python launchers use setup() with different signature
        // They get HydraContext, task_function, config directly
        // This is a no-op since Python manages its own setup
        Ok(())
    }

    fn launch(
        &self,
        job_overrides: &JobOverrideBatch,
        initial_job_idx: usize,
    ) -> Result<Vec<JobReturn>, LauncherError> {
        Python::attach(|py| {
            let launcher = self.py_launcher.bind(py);

            // Convert overrides to Python list of lists
            let py_overrides = PyList::empty(py);
            for job_ov in job_overrides {
                let job_list = PyList::new(py, job_ov)
                    .map_err(|e| LauncherError::new(e.to_string()))?;
                py_overrides.append(job_list)
                    .map_err(|e| LauncherError::new(e.to_string()))?;
            }

            // Call launch(job_overrides, initial_job_idx)
            let result = launcher.call_method1("launch", (py_overrides, initial_job_idx))
                .map_err(|e| LauncherError::new(e.to_string()))?;

            // Convert results to Vec<JobReturn>
            let results_list = result.cast::<PyList>()
                .map_err(|e| LauncherError::new(format!("launch must return list: {}", e)))?;

            let mut returns = Vec::new();
            for item in results_list.iter() {
                // Extract JobReturn fields from Python object
                let return_value = item.getattr("return_value").ok()
                    .and_then(|v| if v.is_none() { None } else { Some(ConfigDict::new()) });
                let working_dir: String = item.getattr("working_dir")
                    .and_then(|v| v.extract())
                    .unwrap_or_default();
                let output_dir: String = item.getattr("hydra")
                    .and_then(|h| h.getattr("run"))
                    .and_then(|r| r.getattr("dir"))
                    .and_then(|d| d.extract())
                    .unwrap_or_default();
                let job_name: String = item.getattr("hydra")
                    .and_then(|h| h.getattr("job"))
                    .and_then(|j| j.getattr("name"))
                    .and_then(|n| n.extract())
                    .unwrap_or_default();
                let task_name: String = item.getattr("task_name")
                    .and_then(|v| v.extract())
                    .unwrap_or_default();
                let status_code: i32 = item.getattr("status")
                    .and_then(|s| s.getattr("value"))
                    .and_then(|v| v.extract())
                    .unwrap_or(0);

                returns.push(JobReturn {
                    return_value,
                    working_dir,
                    output_dir,
                    job_name,
                    task_name,
                    status_code,
                });
            }

            Ok(returns)
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// SAFETY: Python GIL ensures thread safety
unsafe impl Send for PyLauncherWrapper {}
unsafe impl Sync for PyLauncherWrapper {}

/// Python-accessible Rust BasicLauncher
#[pyclass(name = "RustBasicLauncher")]
pub struct PyBasicLauncher {
    inner: BasicLauncher,
}

#[pymethods]
impl PyBasicLauncher {
    #[new]
    fn new() -> Self {
        Self {
            inner: BasicLauncher::new(),
        }
    }

    /// Get launcher name
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// Setup the launcher
    fn setup(&mut self, config: Bound<'_, PyDict>, task_name: &str) -> PyResult<()> {
        let config_dict = py_dict_to_config_dict(&config)?;
        self.inner.setup(&config_dict, task_name)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.message))
    }

    /// Launch jobs
    fn launch(&self, job_overrides: Bound<'_, PyList>, initial_job_idx: usize) -> PyResult<Vec<PyJobReturn>> {
        let overrides = py_to_job_overrides(&job_overrides)?;
        let results = self.inner.launch(&overrides, initial_job_idx)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.message))?;

        Ok(results.iter().map(|r| PyJobReturn::from(r)).collect())
    }
}

/// Python-accessible launcher manager
#[pyclass(name = "LauncherManager")]
pub struct PyLauncherManager {
    inner: LauncherManager,
}

#[pymethods]
impl PyLauncherManager {
    #[new]
    fn new() -> Self {
        Self {
            inner: LauncherManager::new(),
        }
    }

    /// Set BasicLauncher as the active launcher
    fn set_basic_launcher(&mut self) {
        self.inner.set_basic_launcher();
    }

    /// Add a Python launcher
    fn set_python_launcher(&mut self, launcher: Py<PyAny>) {
        self.inner.set_launcher(Arc::new(PyLauncherWrapper::new(launcher)));
    }

    /// Check if a launcher is configured
    fn has_launcher(&self) -> bool {
        self.inner.launcher().is_some()
    }

    /// Get launcher name
    fn launcher_name(&self) -> Option<String> {
        self.inner.launcher().map(|l| l.name().to_string())
    }

    /// Launch jobs
    fn launch(&self, job_overrides: Bound<'_, PyList>, initial_job_idx: usize) -> PyResult<Vec<PyJobReturn>> {
        let overrides = py_to_job_overrides(&job_overrides)?;
        let results = self.inner.launch(&overrides, initial_job_idx)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.message))?;

        Ok(results.iter().map(|r| PyJobReturn::from(r)).collect())
    }
}

/// Register launcher classes with the Python module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyBasicLauncher>()?;
    m.add_class::<PyLauncherManager>()?;
    Ok(())
}
