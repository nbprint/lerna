//! PyO3 bindings for Rust ConfigSource trait
//!
//! This module provides:
//! - `PyConfigSourceWrapper` - wraps Python ConfigSource to implement Rust trait
//! - `PyFileConfigSource` - exposes Rust FileConfigSource to Python
//! - `PyConfigResult` - exposes config load results to Python

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use std::sync::Arc;

use lerna::config::source::{ConfigResult, ConfigSource, FileConfigSource};
use lerna::config::value::{ConfigDict, ConfigValue};
use lerna::config::ConfigLoadError;
use lerna::ObjectType;

/// Convert ConfigValue to Python object
fn config_value_to_py(py: Python<'_>, value: &ConfigValue) -> PyResult<Py<PyAny>> {
    match value {
        ConfigValue::Null => Ok(py.None()),
        ConfigValue::Bool(b) => Ok((*b).into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::Int(i) => Ok((*i).into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::Float(f) => Ok((*f).into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::String(s) => Ok(s.as_str().into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::List(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(config_value_to_py(py, item)?)?;
            }
            Ok(list.into_any().unbind())
        }
        ConfigValue::Dict(d) => {
            let dict = PyDict::new(py);
            for (k, v) in d.iter() {
                dict.set_item(k, config_value_to_py(py, v)?)?;
            }
            Ok(dict.into_any().unbind())
        }
        ConfigValue::Missing => Ok("???".into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::Interpolation(s) => Ok(s.as_str().into_pyobject(py)?.to_owned().into_any().unbind()),
    }
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

/// Python-accessible config load result
#[pyclass(name = "ConfigResult")]
#[derive(Clone)]
pub struct PyConfigResult {
    #[pyo3(get)]
    pub provider: String,
    #[pyo3(get)]
    pub path: String,
    #[pyo3(get)]
    pub is_schema_source: bool,
    config: ConfigValue,
    header: HashMap<String, String>,
}

#[pymethods]
impl PyConfigResult {
    #[new]
    #[pyo3(signature = (provider, path, config, header=None, is_schema_source=false))]
    fn new(
        provider: String,
        path: String,
        config: Bound<'_, PyAny>,
        header: Option<Bound<'_, PyDict>>,
        is_schema_source: bool,
    ) -> PyResult<Self> {
        let config_value = py_to_config_value(&config)?;
        let header_map = if let Some(h) = header {
            let mut map = HashMap::new();
            for (k, v) in h.iter() {
                let key: String = k.extract()?;
                let value: String = v.extract()?;
                map.insert(key, value);
            }
            map
        } else {
            HashMap::new()
        };
        Ok(Self {
            provider,
            path,
            config: config_value,
            header: header_map,
            is_schema_source,
        })
    }

    /// Get the config as a Python dict
    fn get_config(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        config_value_to_py(py, &self.config)
    }

    /// Get the header as a Python dict
    fn get_header(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.header {
            dict.set_item(k, v)?;
        }
        Ok(dict.unbind())
    }
}

impl From<ConfigResult> for PyConfigResult {
    fn from(result: ConfigResult) -> Self {
        Self {
            provider: result.provider,
            path: result.path,
            config: result.config,
            header: result.header,
            is_schema_source: result.is_schema_source,
        }
    }
}

/// Wrapper that allows Python ConfigSource to implement Rust ConfigSource trait
pub struct PyConfigSourceWrapper {
    py_source: Py<PyAny>,
}

impl PyConfigSourceWrapper {
    pub fn new(py_source: Py<PyAny>) -> Self {
        Self { py_source }
    }
}

impl ConfigSource for PyConfigSourceWrapper {
    fn scheme(&self) -> &str {
        // This is a bit awkward - we need to cache the scheme
        // For now, return a static value that gets overridden
        "py"
    }

    fn provider(&self) -> &str {
        "python"
    }

    fn path(&self) -> &str {
        ""
    }

    fn available(&self) -> bool {
        Python::attach(|py| {
            let source = self.py_source.bind(py);
            source.call_method0("available")
                .and_then(|r| r.extract::<bool>())
                .unwrap_or(false)
        })
    }

    fn load_config(&self, config_path: &str) -> Result<ConfigResult, ConfigLoadError> {
        Python::attach(|py| {
            let source = self.py_source.bind(py);
            let result = source.call_method1("load_config", (config_path,))
                .map_err(|e| ConfigLoadError::new(e.to_string()))?;

            // Extract ConfigResult fields from Python object
            let provider: String = result.getattr("provider")
                .and_then(|v| v.extract())
                .unwrap_or_default();
            let path: String = result.getattr("path")
                .and_then(|v| v.extract())
                .unwrap_or_default();
            let is_schema_source: bool = result.getattr("is_schema_source")
                .and_then(|v| v.extract())
                .unwrap_or(false);

            // Extract header - call get_header() method
            let header: HashMap<String, String> = result.call_method0("get_header")
                .and_then(|h| {
                    if let Ok(dict) = h.cast::<PyDict>() {
                        let mut map = HashMap::new();
                        for (k, v) in dict.iter() {
                            if let (Ok(key), Ok(val)) = (k.extract::<String>(), v.extract::<String>()) {
                                map.insert(key, val);
                            }
                        }
                        Ok(map)
                    } else {
                        Ok(HashMap::new())
                    }
                })
                .unwrap_or_default();

            // Extract config - call get_config() method
            let config = result.call_method0("get_config")
                .map_err(|e| ConfigLoadError::new(e.to_string()))?;
            let config_value = py_to_config_value(&config)
                .map_err(|e| ConfigLoadError::new(e.to_string()))?;

            Ok(ConfigResult {
                provider,
                path,
                config: config_value,
                header,
                is_schema_source,
            })
        })
    }

    fn is_group(&self, config_path: &str) -> bool {
        Python::attach(|py| {
            let source = self.py_source.bind(py);
            source.call_method1("is_group", (config_path,))
                .and_then(|r| r.extract::<bool>())
                .unwrap_or(false)
        })
    }

    fn is_config(&self, config_path: &str) -> bool {
        Python::attach(|py| {
            let source = self.py_source.bind(py);
            source.call_method1("is_config", (config_path,))
                .and_then(|r| r.extract::<bool>())
                .unwrap_or(false)
        })
    }

    fn list(&self, config_path: &str, results_filter: Option<ObjectType>) -> Vec<String> {
        Python::attach(|py| {
            let source = self.py_source.bind(py);

            // Convert ObjectType to Python-compatible value
            let filter_arg = match results_filter {
                None => py.None(),
                Some(ObjectType::Group) => "GROUP".into_pyobject(py).unwrap().into_any().unbind(),
                Some(ObjectType::Config) => "CONFIG".into_pyobject(py).unwrap().into_any().unbind(),
                Some(ObjectType::NotFound) => py.None(),
            };

            source.call_method1("list", (config_path, filter_arg))
                .and_then(|r| r.extract::<Vec<String>>())
                .unwrap_or_default()
        })
    }
}

// SAFETY: Python GIL ensures thread safety
unsafe impl Send for PyConfigSourceWrapper {}
unsafe impl Sync for PyConfigSourceWrapper {}

/// Python-accessible Rust FileConfigSource
#[pyclass(name = "RustFileConfigSource")]
pub struct PyFileConfigSource {
    inner: FileConfigSource,
}

#[pymethods]
impl PyFileConfigSource {
    #[new]
    fn new(provider: String, path: String) -> Self {
        Self {
            inner: FileConfigSource::new(&provider, &path),
        }
    }

    /// Get the scheme (always "file")
    fn scheme(&self) -> &str {
        self.inner.scheme()
    }

    /// Get the provider name
    fn provider(&self) -> &str {
        self.inner.provider()
    }

    /// Get the base path
    fn path(&self) -> &str {
        self.inner.path()
    }

    /// Check if available
    fn available(&self) -> bool {
        self.inner.available()
    }

    /// Load a config
    fn load_config(&self, _py: Python<'_>, config_path: &str) -> PyResult<PyConfigResult> {
        self.inner.load_config(config_path)
            .map(|r| PyConfigResult::from(r))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
    }

    /// Check if path is a group
    fn is_group(&self, config_path: &str) -> bool {
        self.inner.is_group(config_path)
    }

    /// Check if path is a config
    fn is_config(&self, config_path: &str) -> bool {
        self.inner.is_config(config_path)
    }

    /// Check if path exists
    fn exists(&self, config_path: &str) -> bool {
        self.inner.exists(config_path)
    }

    /// List items in path
    #[pyo3(signature = (config_path, results_filter=None))]
    fn list(&self, config_path: &str, results_filter: Option<&str>) -> Vec<String> {
        let filter = match results_filter {
            Some("GROUP") => Some(ObjectType::Group),
            Some("CONFIG") => Some(ObjectType::Config),
            _ => None,
        };
        self.inner.list(config_path, filter)
    }
}

/// Config source manager that can hold both Rust and Python sources
#[pyclass(name = "ConfigSourceManager")]
pub struct PyConfigSourceManager {
    sources: Vec<Arc<dyn ConfigSource>>,
}

#[pymethods]
impl PyConfigSourceManager {
    #[new]
    fn new() -> Self {
        Self { sources: Vec::new() }
    }

    /// Add a Rust file source
    fn add_file_source(&mut self, provider: String, path: String) {
        self.sources.push(Arc::new(FileConfigSource::new(&provider, &path)));
    }

    /// Add a Python source
    fn add_python_source(&mut self, source: Py<PyAny>) {
        self.sources.push(Arc::new(PyConfigSourceWrapper::new(source)));
    }

    /// Number of sources
    fn len(&self) -> usize {
        self.sources.len()
    }

    /// Check if empty
    fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Find a config in any source
    fn find_config(&self, config_path: &str) -> Option<usize> {
        for (i, source) in self.sources.iter().enumerate() {
            if source.is_config(config_path) {
                return Some(i);
            }
        }
        None
    }

    /// Load config from first matching source
    fn load_config(&self, _py: Python<'_>, config_path: &str) -> PyResult<Option<PyConfigResult>> {
        for source in &self.sources {
            if source.is_config(config_path) {
                return source.load_config(config_path)
                    .map(|r| Some(PyConfigResult::from(r)))
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()));
            }
        }
        Ok(None)
    }

    /// List items from all sources
    #[pyo3(signature = (config_path, results_filter=None))]
    fn list_all(&self, config_path: &str, results_filter: Option<&str>) -> Vec<String> {
        let filter = match results_filter {
            Some("GROUP") => Some(ObjectType::Group),
            Some("CONFIG") => Some(ObjectType::Config),
            _ => None,
        };

        let mut items: Vec<String> = self.sources
            .iter()
            .flat_map(|s| s.list(config_path, filter))
            .collect();
        items.sort();
        items.dedup();
        items
    }

    /// Clear all sources
    fn clear(&mut self) {
        self.sources.clear();
    }
}

/// Register config source classes with the Python module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyConfigResult>()?;
    m.add_class::<PyFileConfigSource>()?;
    m.add_class::<PyConfigSourceManager>()?;
    Ok(())
}
