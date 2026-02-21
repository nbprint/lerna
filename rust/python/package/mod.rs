// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Python bindings for package resolution module

use lerna::package::{
    compute_target_path, join_path, parse_package_header, split_path, PackageResolver,
};
use pyo3::prelude::*;

/// Python wrapper for PackageResolver
#[pyclass(name = "PackageResolver")]
#[derive(Clone)]
pub struct PyPackageResolver {
    inner: PackageResolver,
}

#[pymethods]
impl PyPackageResolver {
    #[new]
    fn new() -> Self {
        Self {
            inner: PackageResolver::new(),
        }
    }

    /// Set the config group path
    fn with_config_group(&self, group: &str) -> Self {
        Self {
            inner: self.inner.clone().with_config_group(group),
        }
    }

    /// Set the package override
    fn with_package_override(&self, package: &str) -> Self {
        Self {
            inner: self.inner.clone().with_package_override(package),
        }
    }

    /// Set the header package
    fn with_header_package(&self, package: &str) -> Self {
        Self {
            inner: self.inner.clone().with_header_package(package),
        }
    }

    /// Resolve the final package path
    fn resolve(&self) -> String {
        self.inner.resolve()
    }
}

/// Parse @package directive from config header
#[pyfunction]
fn parse_package_from_header(content: &str) -> Option<String> {
    parse_package_header(content)
}

/// Compute the target path for a config value
#[pyfunction]
fn compute_config_target_path(package: &str, key_path: &str) -> String {
    compute_target_path(package, key_path)
}

/// Split a dotted path into components
#[pyfunction]
fn split_dotted_path(path: &str) -> Vec<String> {
    split_path(path)
}

/// Join path components with dots
#[pyfunction]
fn join_dotted_path(components: Vec<String>) -> String {
    join_path(&components)
}

/// Register this module
pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "package")?;
    m.add_class::<PyPackageResolver>()?;
    m.add_function(wrap_pyfunction!(parse_package_from_header, &m)?)?;
    m.add_function(wrap_pyfunction!(compute_config_target_path, &m)?)?;
    m.add_function(wrap_pyfunction!(split_dotted_path, &m)?)?;
    m.add_function(wrap_pyfunction!(join_dotted_path, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
