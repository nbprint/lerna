//! PyO3 bindings for Rust callback system

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::sync::Arc;

use lerna::callback::{Callback, CallbackManager, CallbackResult, JobReturn, LoggingCallback, NoOpCallback};
use lerna::config::ConfigDict;
use lerna::ConfigValue;

fn config_to_py<'py>(py: Python<'py>, config: &ConfigDict) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    for (key, value) in config.iter() {
        dict.set_item(key, value.to_string())?;
    }
    Ok(dict)
}

fn kwargs_to_py<'py>(py: Python<'py>, kwargs: &HashMap<String, String>) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    for (key, value) in kwargs {
        dict.set_item(key, value)?;
    }
    Ok(dict)
}

/// Python-accessible JobReturn info
#[pyclass(name = "JobReturn")]
#[derive(Clone)]
pub struct PyJobReturn {
    #[pyo3(get, set)]
    pub return_value: Option<String>,
    #[pyo3(get, set)]
    pub working_dir: String,
    #[pyo3(get, set)]
    pub output_dir: String,
    #[pyo3(get, set)]
    pub job_name: String,
    #[pyo3(get, set)]
    pub task_name: String,
    #[pyo3(get, set)]
    pub status_code: i32,
}

#[pymethods]
impl PyJobReturn {
    #[new]
    #[pyo3(signature = (job_name, task_name, working_dir, output_dir, status_code=0, return_value=None))]
    fn new(
        job_name: String,
        task_name: String,
        working_dir: String,
        output_dir: String,
        status_code: i32,
        return_value: Option<String>,
    ) -> Self {
        Self { return_value, working_dir, output_dir, job_name, task_name, status_code }
    }

    fn is_success(&self) -> bool {
        self.status_code == 0
    }
}

impl From<&JobReturn> for PyJobReturn {
    fn from(jr: &JobReturn) -> Self {
        Self {
            return_value: jr.return_value.as_ref().map(|_| "ConfigDict".to_string()),
            working_dir: jr.working_dir.clone(),
            output_dir: jr.output_dir.clone(),
            job_name: jr.job_name.clone(),
            task_name: jr.task_name.clone(),
            status_code: jr.status_code,
        }
    }
}

impl From<&PyJobReturn> for JobReturn {
    fn from(pj: &PyJobReturn) -> Self {
        Self {
            return_value: None,
            working_dir: pj.working_dir.clone(),
            output_dir: pj.output_dir.clone(),
            job_name: pj.job_name.clone(),
            task_name: pj.task_name.clone(),
            status_code: pj.status_code,
        }
    }
}

/// Wrapper that allows Python callbacks to implement Rust Callback trait
pub struct PyCallbackWrapper {
    py_callback: Py<PyAny>,
}

impl PyCallbackWrapper {
    pub fn new(py_callback: Py<PyAny>) -> Self {
        Self { py_callback }
    }

    fn call_method(
        &self,
        method: &str,
        config: &ConfigDict,
        kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        Python::attach(|py| {
            let py_config = config_to_py(py, config).map_err(|e| e.to_string())?;
            let py_kwargs = kwargs_to_py(py, kwargs).map_err(|e| e.to_string())?;
            let callback = self.py_callback.bind(py);
            if callback.hasattr(method).map_err(|e| e.to_string())? {
                callback.call_method1(method, (py_config, py_kwargs)).map_err(|e| e.to_string())?;
            }
            Ok(())
        })
    }

    fn call_job_end(
        &self,
        config: &ConfigDict,
        job_return: &JobReturn,
        kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        Python::attach(|py| {
            let py_config = config_to_py(py, config).map_err(|e| e.to_string())?;
            let py_job_return = PyJobReturn::from(job_return);
            let py_kwargs = kwargs_to_py(py, kwargs).map_err(|e| e.to_string())?;
            let callback = self.py_callback.bind(py);
            if callback.hasattr("on_job_end").map_err(|e| e.to_string())? {
                callback.call_method1("on_job_end", (py_config, py_job_return, py_kwargs)).map_err(|e| e.to_string())?;
            }
            Ok(())
        })
    }

    fn call_compose_config(
        &self,
        config: &ConfigDict,
        config_name: Option<&str>,
        overrides: &[String],
    ) -> CallbackResult<()> {
        Python::attach(|py| {
            let py_config = config_to_py(py, config).map_err(|e| e.to_string())?;
            let py_config_name = config_name.map(|s| s.to_string());
            let py_overrides: Vec<String> = overrides.to_vec();
            let callback = self.py_callback.bind(py);
            if callback.hasattr("on_compose_config").map_err(|e| e.to_string())? {
                callback.call_method1("on_compose_config", (py_config, py_config_name, py_overrides)).map_err(|e| e.to_string())?;
            }
            Ok(())
        })
    }
}

impl Callback for PyCallbackWrapper {
    fn on_run_start(&self, config: &ConfigDict, kwargs: &HashMap<String, String>) -> CallbackResult<()> {
        self.call_method("on_run_start", config, kwargs)
    }
    fn on_run_end(&self, config: &ConfigDict, kwargs: &HashMap<String, String>) -> CallbackResult<()> {
        self.call_method("on_run_end", config, kwargs)
    }
    fn on_multirun_start(&self, config: &ConfigDict, kwargs: &HashMap<String, String>) -> CallbackResult<()> {
        self.call_method("on_multirun_start", config, kwargs)
    }
    fn on_multirun_end(&self, config: &ConfigDict, kwargs: &HashMap<String, String>) -> CallbackResult<()> {
        self.call_method("on_multirun_end", config, kwargs)
    }
    fn on_job_start(&self, config: &ConfigDict, kwargs: &HashMap<String, String>) -> CallbackResult<()> {
        self.call_method("on_job_start", config, kwargs)
    }
    fn on_job_end(&self, config: &ConfigDict, job_return: &JobReturn, kwargs: &HashMap<String, String>) -> CallbackResult<()> {
        self.call_job_end(config, job_return, kwargs)
    }
    fn on_compose_config(&self, config: &ConfigDict, config_name: Option<&str>, overrides: &[String]) -> CallbackResult<()> {
        self.call_compose_config(config, config_name, overrides)
    }
}

unsafe impl Send for PyCallbackWrapper {}
unsafe impl Sync for PyCallbackWrapper {}

/// Python-accessible CallbackManager
#[pyclass(name = "CallbackManager")]
pub struct PyCallbackManager {
    inner: CallbackManager,
}

#[pymethods]
impl PyCallbackManager {
    #[new]
    fn new() -> Self {
        Self { inner: CallbackManager::new() }
    }

    /// Add a Python callback
    fn add_callback(&mut self, callback: Py<PyAny>) {
        self.inner.add(Arc::new(PyCallbackWrapper::new(callback)));
    }

    /// Add the built-in logging callback
    fn add_logging_callback(&mut self) {
        self.inner.add(Arc::new(LoggingCallback));
    }

    /// Add the built-in no-op callback
    fn add_noop_callback(&mut self) {
        self.inner.add(Arc::new(NoOpCallback));
    }

    /// Trigger on_run_start for all callbacks
    fn on_run_start(&self, config: Bound<'_, PyDict>) -> PyResult<()> {
        let rust_config = py_dict_to_config(&config)?;
        self.inner.on_run_start(&rust_config, &HashMap::new())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Trigger on_run_end for all callbacks
    fn on_run_end(&self, config: Bound<'_, PyDict>) -> PyResult<()> {
        let rust_config = py_dict_to_config(&config)?;
        self.inner.on_run_end(&rust_config, &HashMap::new())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Trigger on_job_start for all callbacks
    fn on_job_start(&self, config: Bound<'_, PyDict>) -> PyResult<()> {
        let rust_config = py_dict_to_config(&config)?;
        self.inner.on_job_start(&rust_config, &HashMap::new())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Trigger on_job_end for all callbacks
    fn on_job_end(&self, config: Bound<'_, PyDict>, job_return: &PyJobReturn) -> PyResult<()> {
        let rust_config = py_dict_to_config(&config)?;
        let rust_jr = JobReturn::from(job_return);
        self.inner.on_job_end(&rust_config, &rust_jr, &HashMap::new())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Number of registered callbacks
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clear all callbacks
    fn clear(&mut self) {
        self.inner.clear();
    }
}

/// Convert Python dict to ConfigDict
fn py_dict_to_config(dict: &Bound<'_, PyDict>) -> PyResult<ConfigDict> {
    let mut config = ConfigDict::new();
    for (key, value) in dict.iter() {
        let key_str: String = key.extract()?;
        let value_str: String = value.str()?.extract()?;
        config.insert(key_str, ConfigValue::String(value_str));
    }
    Ok(config)
}

/// Register callback classes with the Python module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyJobReturn>()?;
    m.add_class::<PyCallbackManager>()?;
    Ok(())
}
