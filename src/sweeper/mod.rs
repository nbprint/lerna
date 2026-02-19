//! PyO3 bindings for Rust Sweeper trait
//!
//! This module provides:
//! - `PySweeperWrapper` - wraps Python Sweeper to implement Rust trait
//! - `PyBasicSweeper` - exposes Rust BasicSweeper to Python
//! - `PySweeperManager` - manages sweepers from Python

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::sync::Arc;

use lerna::callback::JobReturn;
use lerna::config::value::{ConfigDict, ConfigValue};
use lerna::launcher::{BasicLauncher, Launcher};
use lerna::sweeper::{BasicSweeper, Sweeper, SweeperError, SweeperManager};

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

/// Wrapper that allows Python Sweeper to implement Rust Sweeper trait
pub struct PySweeperWrapper {
    py_sweeper: Py<PyAny>,
    name: String,
}

impl PySweeperWrapper {
    pub fn new(py_sweeper: Py<PyAny>) -> Self {
        let name = Python::attach(|py| {
            py_sweeper.bind(py)
                .getattr("__class__")
                .and_then(|c| c.getattr("__name__"))
                .and_then(|n| n.extract::<String>())
                .unwrap_or_else(|_| "PySweeper".to_string())
        });
        Self { py_sweeper, name }
    }
}

impl std::fmt::Debug for PySweeperWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PySweeperWrapper")
            .field("name", &self.name)
            .finish()
    }
}

impl Sweeper for PySweeperWrapper {
    fn setup(
        &mut self,
        _config: &ConfigDict,
        _launcher: Arc<dyn Launcher>,
    ) -> Result<(), SweeperError> {
        // Python sweepers use setup() with different signature
        // They get HydraContext, task_function, config directly
        // This is a no-op since Python manages its own setup
        Ok(())
    }

    fn sweep(&self, arguments: &[String]) -> Result<Vec<JobReturn>, SweeperError> {
        Python::attach(|py| {
            let sweeper = self.py_sweeper.bind(py);

            // Convert arguments to Python list
            let py_args = PyList::new(py, arguments)
                .map_err(|e| SweeperError::new(e.to_string()))?;

            // Call sweep(arguments)
            let result = sweeper.call_method1("sweep", (py_args,))
                .map_err(|e| SweeperError::new(e.to_string()))?;

            // Convert results to Vec<JobReturn>
            let results_list = result.cast::<PyList>()
                .map_err(|e| SweeperError::new(format!("sweep must return list: {}", e)))?;

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
unsafe impl Send for PySweeperWrapper {}
unsafe impl Sync for PySweeperWrapper {}

/// Python-accessible Rust BasicSweeper
#[pyclass(name = "RustBasicSweeper")]
pub struct PyBasicSweeper {
    inner: BasicSweeper,
    launcher: Option<Arc<dyn Launcher>>,
}

#[pymethods]
impl PyBasicSweeper {
    #[new]
    #[pyo3(signature = (max_batch_size=None))]
    fn new(max_batch_size: Option<usize>) -> Self {
        Self {
            inner: BasicSweeper::new(max_batch_size),
            launcher: None,
        }
    }

    /// Get sweeper name
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// Setup the sweeper with config (creates internal BasicLauncher)
    fn setup(&mut self, config: Bound<'_, PyDict>, task_name: &str) -> PyResult<()> {
        let config_dict = py_dict_to_config_dict(&config)?;

        // Create a basic launcher for the sweeper
        let mut launcher = BasicLauncher::new();
        launcher.setup(&config_dict, task_name)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.message))?;

        let launcher = Arc::new(launcher);
        self.launcher = Some(launcher.clone());

        self.inner.setup(&config_dict, launcher)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.message))
    }

    /// Execute sweep with arguments
    fn sweep(&self, arguments: Vec<String>) -> PyResult<Vec<PyJobReturn>> {
        let results = self.inner.sweep(&arguments)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.message))?;

        Ok(results.iter().map(|r| PyJobReturn::from(r)).collect())
    }
}

/// Python-accessible sweeper manager
#[pyclass(name = "SweeperManager")]
pub struct PySweeperManager {
    inner: SweeperManager,
}

#[pymethods]
impl PySweeperManager {
    #[new]
    fn new() -> Self {
        Self {
            inner: SweeperManager::new(),
        }
    }

    /// Set BasicSweeper as the active sweeper
    #[pyo3(signature = (max_batch_size=None))]
    fn set_basic_sweeper(&mut self, max_batch_size: Option<usize>) {
        self.inner.set_basic_sweeper(max_batch_size);
    }

    /// Add a Python sweeper
    fn set_python_sweeper(&mut self, sweeper: Py<PyAny>) {
        self.inner.set_sweeper(Arc::new(PySweeperWrapper::new(sweeper)));
    }

    /// Check if a sweeper is configured
    fn has_sweeper(&self) -> bool {
        self.inner.sweeper().is_some()
    }

    /// Get sweeper name
    fn sweeper_name(&self) -> Option<String> {
        self.inner.sweeper().map(|s| s.name().to_string())
    }

    /// Execute sweep
    fn sweep(&self, arguments: Vec<String>) -> PyResult<Vec<PyJobReturn>> {
        let results = self.inner.sweep(&arguments)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.message))?;

        Ok(results.iter().map(|r| PyJobReturn::from(r)).collect())
    }
}

/// Register sweeper classes with the Python module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyBasicSweeper>()?;
    m.add_class::<PySweeperManager>()?;
    Ok(())
}
