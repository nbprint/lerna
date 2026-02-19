// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyO3 bindings for glob pattern matching

use pyo3::prelude::*;
use lerna::Glob;

/// A glob pattern for filtering names
#[pyclass(name = "Glob")]
#[derive(Clone)]
pub struct PyGlob {
    inner: Glob,
}

#[pymethods]
impl PyGlob {
    #[new]
    #[pyo3(signature = (include=None, exclude=None))]
    fn new(include: Option<Vec<String>>, exclude: Option<Vec<String>>) -> Self {
        let mut glob = Glob::new();
        if let Some(inc) = include {
            glob = glob.with_include(inc);
        }
        if let Some(exc) = exclude {
            glob = glob.with_exclude(exc);
        }
        Self { inner: glob }
    }

    #[getter]
    fn include(&self) -> Vec<String> {
        self.inner.include.clone()
    }

    #[setter]
    fn set_include(&mut self, value: Vec<String>) {
        self.inner.include = value;
    }

    #[getter]
    fn exclude(&self) -> Vec<String> {
        self.inner.exclude.clone()
    }

    #[setter]
    fn set_exclude(&mut self, value: Vec<String>) {
        self.inner.exclude = value;
    }

    /// Filter a list of names based on include and exclude patterns
    fn filter(&self, names: Vec<String>) -> Vec<String> {
        self.inner.filter(&names)
    }

    fn __repr__(&self) -> String {
        format!("Glob(include={:?}, exclude={:?})", self.inner.include, self.inner.exclude)
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGlob>()?;
    Ok(())
}
