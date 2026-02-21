// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyO3 bindings for configuration loading

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use lerna::config::interpolation::{resolve, ResolverContext};
use lerna::config::value::{ConfigDict, ConfigValue};
use lerna::config::{
    CachingConfigRepository, ConfigRepository as RustConfigRepository,
    SearchPathElement as RustSearchPathElement,
};
use lerna::config::{ConfigLoader as RustConfigLoader, SearchPathEntry as RustSearchPathEntry};
use lerna::ObjectType;

/// Convert ConfigValue to a Python object
fn config_value_to_py(py: Python, value: &ConfigValue) -> PyResult<Py<PyAny>> {
    match value {
        ConfigValue::Null => Ok(py.None()),
        ConfigValue::Bool(b) => Ok((*b).into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::Int(i) => Ok((*i).into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::Float(f) => Ok((*f).into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::String(s) => Ok(s.as_str().into_pyobject(py)?.to_owned().into_any().unbind()),
        ConfigValue::Interpolation(s) => {
            Ok(s.as_str().into_pyobject(py)?.to_owned().into_any().unbind())
        }
        ConfigValue::Missing => {
            // Return the string "???" to represent missing values
            Ok("???".into_pyobject(py)?.to_owned().into_any().unbind())
        }
        ConfigValue::List(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(config_value_to_py(py, item)?)?;
            }
            Ok(list.into_any().unbind())
        }
        ConfigValue::Dict(dict) => config_dict_to_py(py, dict),
    }
}

/// Convert ConfigDict to a Python dict
fn config_dict_to_py(py: Python, dict: &ConfigDict) -> PyResult<Py<PyAny>> {
    let py_dict = PyDict::new(py);
    for (key, value) in dict.iter() {
        py_dict.set_item(key, config_value_to_py(py, value)?)?;
    }
    Ok(py_dict.into_any().unbind())
}

/// Convert a Python object to ConfigValue
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
        } else {
            // Always store as String - interpolation resolution handles ${...}
            Ok(ConfigValue::String(s))
        }
    } else if let Ok(list) = obj.cast::<PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(py_to_config_value(py, &item)?);
        }
        Ok(ConfigValue::List(items))
    } else if let Ok(dict) = obj.cast::<PyDict>() {
        let mut config_dict = ConfigDict::new();
        for (key, value) in dict.iter() {
            if let Ok(k) = key.extract::<String>() {
                config_dict.insert(k, py_to_config_value(py, &value)?);
            }
        }
        Ok(ConfigValue::Dict(config_dict))
    } else {
        // Fallback to string representation
        Ok(ConfigValue::String(obj.str()?.to_string()))
    }
}

/// A search path entry for config loading
#[pyclass(name = "SearchPathEntry")]
#[derive(Clone)]
pub struct PySearchPathEntry {
    pub provider: String,
    pub path: String,
}

#[pymethods]
impl PySearchPathEntry {
    #[new]
    fn new(provider: String, path: String) -> Self {
        Self { provider, path }
    }

    #[getter]
    fn provider(&self) -> &str {
        &self.provider
    }

    #[getter]
    fn path(&self) -> &str {
        &self.path
    }

    fn __repr__(&self) -> String {
        format!(
            "SearchPathEntry(provider='{}', path='{}')",
            self.provider, self.path
        )
    }
}

impl From<&PySearchPathEntry> for RustSearchPathEntry {
    fn from(entry: &PySearchPathEntry) -> Self {
        RustSearchPathEntry::new(&entry.provider, &entry.path)
    }
}

/// Configuration loader that manages sources and loads configs
#[pyclass(name = "ConfigLoader")]
pub struct PyConfigLoader {
    loader: RustConfigLoader,
}

#[pymethods]
impl PyConfigLoader {
    /// Create a new config loader from search paths
    #[new]
    #[pyo3(signature = (search_paths=None, config_dir=None))]
    fn new(
        search_paths: Option<Vec<PySearchPathEntry>>,
        config_dir: Option<String>,
    ) -> PyResult<Self> {
        let loader = if let Some(dir) = config_dir {
            RustConfigLoader::from_config_dir(&dir)
        } else if let Some(paths) = search_paths {
            let rust_paths: Vec<RustSearchPathEntry> = paths.iter().map(|p| p.into()).collect();
            RustConfigLoader::new(rust_paths)
        } else {
            return Err(PyRuntimeError::new_err(
                "Either search_paths or config_dir must be provided",
            ));
        };

        Ok(Self { loader })
    }

    /// Create a loader from a config directory
    #[staticmethod]
    fn from_config_dir(config_dir: &str) -> Self {
        Self {
            loader: RustConfigLoader::from_config_dir(config_dir),
        }
    }

    /// Load a configuration by name with optional overrides
    #[pyo3(signature = (config_name=None, overrides=None))]
    fn load_config(
        &self,
        py: Python,
        config_name: Option<&str>,
        overrides: Option<Vec<String>>,
    ) -> PyResult<Py<PyAny>> {
        let overrides_ref: Vec<String> = overrides.unwrap_or_default();

        let config = self
            .loader
            .load_config(config_name, &overrides_ref)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        config_value_to_py(py, &config)
    }

    /// Check if a config exists
    fn config_exists(&self, config_path: &str) -> bool {
        self.loader.config_exists(config_path)
    }

    /// Check if a group exists
    fn group_exists(&self, group_path: &str) -> bool {
        self.loader.group_exists(group_path)
    }

    /// List configs in a group
    fn list_group(&self, group_path: &str) -> Vec<String> {
        self.loader.list_group(group_path)
    }

    /// List groups in a path
    fn list_groups(&self, parent_path: &str) -> Vec<String> {
        self.loader.list_groups(parent_path)
    }

    fn __repr__(&self) -> String {
        format!("ConfigLoader(sources={})", self.loader.sources().len())
    }
}

/// Parse a YAML string into a Python dict
#[pyfunction]
fn parse_yaml(py: Python, content: &str) -> PyResult<Py<PyAny>> {
    let config =
        lerna::config::parse_yaml(content).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    config_value_to_py(py, &config)
}

/// Load a YAML file into a Python dict
#[pyfunction]
fn load_yaml_file(py: Python, path: &str) -> PyResult<Py<PyAny>> {
    let config = lerna::config::load_yaml_file(std::path::Path::new(path))
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    config_value_to_py(py, &config)
}

/// Resolve interpolations in a config dict
///
/// This resolves ${...} references in the config values.
/// Simple references like ${db.host} are looked up in the config.
/// Resolver references like ${oc.env:VAR} call the appropriate resolver.
#[pyfunction]
fn resolve_interpolations(py: Python, config: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    // Convert Python config to Rust
    let config_value = py_to_config_value(py, &config)?;

    // Ensure it's a dict
    let dict = match &config_value {
        ConfigValue::Dict(d) => d.clone(),
        _ => return Err(PyRuntimeError::new_err("Config must be a dictionary")),
    };

    // Create resolver context and resolve
    let ctx = ResolverContext::new(&dict);
    let resolved =
        resolve(&config_value, &ctx).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    config_value_to_py(py, &resolved)
}

/// Compose a configuration entirely in Rust
///
/// This loads a config file, processes defaults, applies overrides,
/// and resolves interpolations all in Rust for maximum performance.
#[pyfunction]
#[pyo3(signature = (config_dir, config_name=None, overrides=None))]
fn compose_config(
    py: Python,
    config_dir: &str,
    config_name: Option<&str>,
    overrides: Option<Vec<String>>,
) -> PyResult<Py<PyAny>> {
    // Create loader
    let loader = RustConfigLoader::from_config_dir(config_dir);

    // Load config with overrides
    let overrides_ref: Vec<String> = overrides.unwrap_or_default();
    let config = loader
        .load_config(config_name, &overrides_ref)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    // Resolve interpolations
    let dict = match &config {
        ConfigValue::Dict(d) => d.clone(),
        _ => return Err(PyRuntimeError::new_err("Config must be a dictionary")),
    };

    let ctx = ResolverContext::new(&dict);
    let resolved = resolve(&config, &ctx).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    config_value_to_py(py, &resolved)
}

/// Extract header directives from config file content
/// Returns a dict of key -> value for @key value directives in comments
#[pyfunction]
fn extract_header_dict(py: Python, content: &str) -> PyResult<Py<PyAny>> {
    let header = lerna::config::extract_header(content);
    let py_dict = PyDict::new(py);
    for (key, value) in header {
        py_dict.set_item(key, value)?;
    }
    // Add package: None if not present (matching Python behavior)
    if !py_dict.contains("package")? {
        py_dict.set_item("package", py.None())?;
    }
    Ok(py_dict.into_any().unbind())
}

/// Configuration repository that manages config sources
///
/// This mirrors the Python IConfigRepository interface with optimized Rust implementation.
#[pyclass(name = "RustConfigRepository")]
pub struct PyConfigRepository {
    inner: RustConfigRepository,
}

#[pymethods]
impl PyConfigRepository {
    /// Create a new config repository from search path elements
    #[new]
    fn new(search_paths: Vec<(String, String)>) -> Self {
        let elements: Vec<RustSearchPathElement> = search_paths
            .iter()
            .map(|(provider, path)| RustSearchPathElement::new(provider, path))
            .collect();

        Self {
            inner: RustConfigRepository::new(&elements),
        }
    }

    /// Load a config by path
    /// Returns None if config doesn't exist
    fn load_config(&self, py: Python, config_path: &str) -> PyResult<Option<Py<PyAny>>> {
        match self.inner.load_config(config_path) {
            Ok(Some(result)) => config_value_to_py(py, &result.config).map(Some),
            Ok(None) => Ok(None),
            Err(e) => Err(PyRuntimeError::new_err(e.to_string())),
        }
    }

    /// Load a config and return full result with header
    fn load_config_full(&self, py: Python, config_path: &str) -> PyResult<Option<Py<PyAny>>> {
        match self.inner.load_config(config_path) {
            Ok(Some(result)) => {
                let dict = PyDict::new(py);
                dict.set_item("provider", &result.provider)?;
                dict.set_item("path", &result.path)?;
                dict.set_item("config", config_value_to_py(py, &result.config)?)?;
                dict.set_item("is_schema_source", result.is_schema_source)?;

                let header_dict = PyDict::new(py);
                for (k, v) in &result.header {
                    header_dict.set_item(k, v)?;
                }
                dict.set_item("header", header_dict)?;

                Ok(Some(dict.into_any().unbind()))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(PyRuntimeError::new_err(e.to_string())),
        }
    }

    /// Check if a group (directory) exists
    fn group_exists(&self, config_path: &str) -> bool {
        self.inner.group_exists(config_path)
    }

    /// Check if a config file exists
    fn config_exists(&self, config_path: &str) -> bool {
        self.inner.config_exists(config_path)
    }

    /// Get available options for a config group
    #[pyo3(signature = (group_name, results_filter=None))]
    fn get_group_options(&self, group_name: &str, results_filter: Option<&str>) -> Vec<String> {
        let filter = match results_filter {
            Some("config") => Some(ObjectType::Config),
            Some("group") => Some(ObjectType::Group),
            _ => Some(ObjectType::Config), // default to config
        };
        self.inner.get_group_options(group_name, filter)
    }

    /// Get the number of sources
    fn num_sources(&self) -> usize {
        self.inner.get_sources().len()
    }

    fn __repr__(&self) -> String {
        format!("RustConfigRepository(sources={})", self.num_sources())
    }
}

/// Configuration repository with caching and composition support
///
/// This wraps CachingConfigRepository and provides load_and_compose
#[pyclass(name = "RustCachingConfigRepository")]
pub struct PyCachingConfigRepository {
    inner: CachingConfigRepository,
}

#[pymethods]
impl PyCachingConfigRepository {
    /// Create a new caching config repository from search path elements
    #[new]
    fn new(search_paths: Vec<(String, String)>) -> Self {
        let elements: Vec<RustSearchPathElement> = search_paths
            .iter()
            .map(|(provider, path)| RustSearchPathElement::new(provider, path))
            .collect();

        // Create the base ConfigRepository first, then wrap with caching
        let base_repo = RustConfigRepository::new(&elements);
        Self {
            inner: CachingConfigRepository::new(base_repo),
        }
    }

    /// Load a config by path
    /// Returns None if config doesn't exist
    fn load_config(&mut self, py: Python, config_path: &str) -> PyResult<Option<Py<PyAny>>> {
        match self.inner.load_config(config_path) {
            Ok(Some(result)) => config_value_to_py(py, &result.config).map(Some),
            Ok(None) => Ok(None),
            Err(e) => Err(PyRuntimeError::new_err(e.to_string())),
        }
    }

    /// Check if a group (directory) exists
    fn group_exists(&self, config_path: &str) -> bool {
        self.inner.group_exists(config_path)
    }

    /// Check if a config file exists
    fn config_exists(&self, config_path: &str) -> bool {
        self.inner.config_exists(config_path)
    }

    /// Get available options for a config group
    #[pyo3(signature = (group_name, results_filter=None))]
    fn get_group_options(&self, group_name: &str, results_filter: Option<&str>) -> Vec<String> {
        let filter = match results_filter {
            Some("config") => Some(ObjectType::Config),
            Some("group") => Some(ObjectType::Group),
            _ => Some(ObjectType::Config),
        };
        self.inner.get_group_options(group_name, filter)
    }

    /// Clear the internal cache
    fn clear_cache(&mut self) {
        self.inner.clear_cache();
    }

    /// Load and compose a full configuration with defaults processing
    ///
    /// This processes the defaults list, merges all configurations,
    /// and applies overrides.
    ///
    /// Returns a dict with:
    /// - config: The fully composed configuration
    /// - defaults: List of ResultDefault objects
    /// - overrides: Dict of overrides used
    #[pyo3(signature = (config_name=None, overrides=None))]
    fn load_and_compose(
        &mut self,
        py: Python,
        config_name: Option<&str>,
        overrides: Option<Vec<String>>,
    ) -> PyResult<Py<PyAny>> {
        let overrides_ref: Vec<String> = overrides.unwrap_or_default();

        let result = self
            .inner
            .load_and_compose(config_name, &overrides_ref)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        // Build result dict
        let dict = PyDict::new(py);

        // Config as nested Python dicts
        dict.set_item("config", config_dict_to_py(py, &result.config)?)?;

        // Defaults list as list of dicts
        let defaults_list = PyList::empty(py);
        for rd in &result.defaults_result.defaults {
            let rd_dict = PyDict::new(py);
            rd_dict.set_item("config_path", rd.config_path.as_deref())?;
            rd_dict.set_item("parent", rd.parent.as_deref())?;
            rd_dict.set_item("package", rd.package.as_deref())?;
            rd_dict.set_item("is_self", rd.is_self)?;
            rd_dict.set_item("primary", rd.primary)?;
            rd_dict.set_item("override_key", rd.override_key.as_deref())?;
            defaults_list.append(rd_dict)?;
        }
        dict.set_item("defaults", defaults_list)?;

        // Config overrides as list
        let config_ovrs = PyList::empty(py);
        for ovr in &result.defaults_result.config_overrides {
            config_ovrs.append(ovr)?;
        }
        dict.set_item("config_overrides", config_ovrs)?;

        Ok(dict.into_any().unbind())
    }

    fn __repr__(&self) -> String {
        format!("RustCachingConfigRepository()")
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySearchPathEntry>()?;
    m.add_class::<PyConfigLoader>()?;
    m.add_class::<PyConfigRepository>()?;
    m.add_class::<PyCachingConfigRepository>()?;
    m.add_class::<PyHybridConfigRepository>()?;
    m.add_function(wrap_pyfunction!(parse_yaml, m)?)?;
    m.add_function(wrap_pyfunction!(load_yaml_file, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_interpolations, m)?)?;
    m.add_function(wrap_pyfunction!(compose_config, m)?)?;
    m.add_function(wrap_pyfunction!(extract_header_dict, m)?)?;
    Ok(())
}

/// Configuration repository that delegates pkg:// and structured:// to Python
///
/// This hybrid approach uses Rust for file:// sources and delegates
/// to Python for pkg:// (importlib.resources) and structured:// (ConfigStore) sources.
#[pyclass(name = "RustHybridConfigRepository")]
pub struct PyHybridConfigRepository {
    /// File sources handled by Rust
    file_sources: Vec<(String, String)>, // (provider, path)
    /// Pkg sources handled via Python callback
    pkg_sources: Vec<(String, String)>, // (provider, module_path)
    /// Structured sources handled via Python callback
    structured_sources: Vec<String>, // list of providers
    /// Python function to load pkg:// configs: (module_path, config_path) -> Optional[dict]
    pkg_loader: Option<Py<PyAny>>,
    /// Python function to check if config exists: (module_path, config_path) -> bool
    pkg_config_exists: Option<Py<PyAny>>,
    /// Python function to check if group exists: (module_path, group_path) -> bool
    pkg_group_exists: Option<Py<PyAny>>,
    /// Python function to list group options: (module_path, group_path) -> List[str]
    pkg_list_options: Option<Py<PyAny>>,
    /// Python function to load structured:// configs: (config_path) -> Optional[dict]
    structured_loader: Option<Py<PyAny>>,
    /// Python function to check if structured config exists: (config_path) -> bool
    structured_config_exists: Option<Py<PyAny>>,
    /// Python function to check if structured group exists: (group_path) -> bool
    structured_group_exists: Option<Py<PyAny>>,
    /// Python function to list structured group options: (group_path) -> List[str]
    structured_list_options: Option<Py<PyAny>>,
    /// Rust repository for file:// sources
    rust_repo: Option<RustConfigRepository>,
    /// Cache for loaded configs
    cache: std::collections::HashMap<String, Option<ConfigValue>>,
}

#[pymethods]
impl PyHybridConfigRepository {
    /// Create a new hybrid repository
    ///
    /// Args:
    ///     search_paths: List of (provider, path) tuples
    ///     pkg_loader: Python function(module_path, config_path) -> Optional[dict]
    ///     pkg_config_exists: Python function(module_path, config_path) -> bool
    ///     pkg_group_exists: Python function(module_path, group_path) -> bool
    ///     pkg_list_options: Python function(module_path, group_path) -> List[str]
    ///     structured_loader: Python function(config_path) -> Optional[dict]
    ///     structured_config_exists: Python function(config_path) -> bool
    ///     structured_group_exists: Python function(group_path) -> bool
    ///     structured_list_options: Python function(group_path) -> List[str]
    #[new]
    #[pyo3(signature = (search_paths, pkg_loader=None, pkg_config_exists=None, pkg_group_exists=None, pkg_list_options=None, structured_loader=None, structured_config_exists=None, structured_group_exists=None, structured_list_options=None))]
    fn new(
        search_paths: Vec<(String, String)>,
        pkg_loader: Option<Py<PyAny>>,
        pkg_config_exists: Option<Py<PyAny>>,
        pkg_group_exists: Option<Py<PyAny>>,
        pkg_list_options: Option<Py<PyAny>>,
        structured_loader: Option<Py<PyAny>>,
        structured_config_exists: Option<Py<PyAny>>,
        structured_group_exists: Option<Py<PyAny>>,
        structured_list_options: Option<Py<PyAny>>,
    ) -> Self {
        let mut file_sources = Vec::new();
        let mut pkg_sources = Vec::new();
        let mut structured_sources = Vec::new();

        for (provider, path) in &search_paths {
            if path.starts_with("pkg://") {
                let module_path = path.strip_prefix("pkg://").unwrap_or(path);
                pkg_sources.push((provider.clone(), module_path.to_string()));
            } else if path.starts_with("structured://") {
                structured_sources.push(provider.clone());
            } else if path.starts_with("file://") {
                file_sources.push((provider.clone(), path.clone()));
            } else {
                // Default to file
                file_sources.push((provider.clone(), path.clone()));
            }
        }

        // Create Rust repo for file sources
        let rust_repo = if !file_sources.is_empty() {
            let elements: Vec<RustSearchPathElement> = file_sources
                .iter()
                .map(|(p, path)| RustSearchPathElement::new(p, path))
                .collect();
            Some(RustConfigRepository::new(&elements))
        } else {
            None
        };

        Self {
            file_sources,
            pkg_sources,
            structured_sources,
            pkg_loader,
            pkg_config_exists,
            pkg_group_exists,
            pkg_list_options,
            structured_loader,
            structured_config_exists,
            structured_group_exists,
            structured_list_options,
            rust_repo,
            cache: std::collections::HashMap::new(),
        }
    }

    /// Load a config by path
    fn load_config(&mut self, py: Python, config_path: &str) -> PyResult<Option<Py<PyAny>>> {
        // Check cache first
        let cache_key = format!("config:{}", config_path);
        if let Some(cached) = self.cache.get(&cache_key) {
            return match cached {
                Some(v) => config_value_to_py(py, v).map(Some),
                None => Ok(None),
            };
        }

        // Try Rust file sources first
        if let Some(ref rust_repo) = self.rust_repo {
            if let Ok(Some(result)) = rust_repo.load_config(config_path) {
                self.cache.insert(cache_key, Some(result.config.clone()));
                return config_value_to_py(py, &result.config).map(Some);
            }
        }

        // Try pkg sources via Python
        if let Some(ref loader) = self.pkg_loader {
            for (_provider, module_path) in &self.pkg_sources {
                let result = loader.call1(py, (module_path.as_str(), config_path))?;
                if !result.is_none(py) {
                    // Convert Python dict to ConfigValue for caching
                    let config_value = py_to_config_value(py, result.bind(py))?;
                    self.cache.insert(cache_key, Some(config_value.clone()));
                    return config_value_to_py(py, &config_value).map(Some);
                }
            }
        }

        // Try structured sources via Python (ConfigStore)
        if !self.structured_sources.is_empty() {
            if let Some(ref loader) = self.structured_loader {
                let result = loader.call1(py, (config_path,))?;
                if !result.is_none(py) {
                    let config_value = py_to_config_value(py, result.bind(py))?;
                    self.cache.insert(cache_key, Some(config_value.clone()));
                    return config_value_to_py(py, &config_value).map(Some);
                }
            }
        }

        self.cache.insert(cache_key, None);
        Ok(None)
    }

    /// Check if a config file exists
    fn config_exists(&self, py: Python, config_path: &str) -> PyResult<bool> {
        // Check Rust file sources
        if let Some(ref rust_repo) = self.rust_repo {
            if rust_repo.config_exists(config_path) {
                return Ok(true);
            }
        }

        // Check pkg sources via Python
        if let Some(ref exists_fn) = self.pkg_config_exists {
            for (_provider, module_path) in &self.pkg_sources {
                let result = exists_fn.call1(py, (module_path.as_str(), config_path))?;
                if result.extract::<bool>(py)? {
                    return Ok(true);
                }
            }
        }

        // Check structured sources via Python (ConfigStore)
        if !self.structured_sources.is_empty() {
            if let Some(ref exists_fn) = self.structured_config_exists {
                let result = exists_fn.call1(py, (config_path,))?;
                if result.extract::<bool>(py)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if a group (directory) exists
    fn group_exists(&self, py: Python, config_path: &str) -> PyResult<bool> {
        // Check Rust file sources
        if let Some(ref rust_repo) = self.rust_repo {
            if rust_repo.group_exists(config_path) {
                return Ok(true);
            }
        }

        // Check pkg sources via Python
        if let Some(ref exists_fn) = self.pkg_group_exists {
            for (_provider, module_path) in &self.pkg_sources {
                let result = exists_fn.call1(py, (module_path.as_str(), config_path))?;
                if result.extract::<bool>(py)? {
                    return Ok(true);
                }
            }
        }

        // Check structured sources via Python (ConfigStore)
        if !self.structured_sources.is_empty() {
            if let Some(ref exists_fn) = self.structured_group_exists {
                let result = exists_fn.call1(py, (config_path,))?;
                if result.extract::<bool>(py)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Get available options for a config group
    #[pyo3(signature = (group_name, results_filter=None))]
    fn get_group_options(
        &self,
        py: Python,
        group_name: &str,
        results_filter: Option<&str>,
    ) -> PyResult<Vec<String>> {
        let mut options = Vec::new();

        // Get from Rust file sources
        if let Some(ref rust_repo) = self.rust_repo {
            let filter = match results_filter {
                Some("config") => Some(ObjectType::Config),
                Some("group") => Some(ObjectType::Group),
                _ => Some(ObjectType::Config),
            };
            options.extend(rust_repo.get_group_options(group_name, filter));
        }

        // Get from pkg sources via Python
        if let Some(ref list_fn) = self.pkg_list_options {
            for (_provider, module_path) in &self.pkg_sources {
                let result = list_fn.call1(py, (module_path.as_str(), group_name))?;
                let items: Vec<String> = result.extract(py)?;
                options.extend(items);
            }
        }

        // Get from structured sources via Python (ConfigStore)
        if !self.structured_sources.is_empty() {
            if let Some(ref list_fn) = self.structured_list_options {
                let result = list_fn.call1(py, (group_name,))?;
                let items: Vec<String> = result.extract(py)?;
                options.extend(items);
            }
        }

        // Deduplicate and sort
        options.sort();
        options.dedup();
        Ok(options)
    }

    /// Clear the cache
    fn clear_cache(&mut self) {
        self.cache.clear();
    }

    fn __repr__(&self) -> String {
        format!(
            "RustHybridConfigRepository(file_sources={}, pkg_sources={}, structured_sources={})",
            self.file_sources.len(),
            self.pkg_sources.len(),
            self.structured_sources.len()
        )
    }
}
