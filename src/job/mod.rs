// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Python bindings for job configuration module

use lerna::job::{compute_output_dir, generate_sweep_jobs, JobConfig};
use pyo3::prelude::*;

/// Python wrapper for JobConfig
#[pyclass(name = "JobConfig")]
#[derive(Clone)]
pub struct PyJobConfig {
    inner: JobConfig,
}

#[pymethods]
impl PyJobConfig {
    #[new]
    #[pyo3(signature = (name, idx, overrides))]
    fn new(name: &str, idx: usize, overrides: Vec<String>) -> Self {
        Self {
            inner: JobConfig::new(name, idx, overrides),
        }
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn idx(&self) -> usize {
        self.inner.idx
    }

    #[getter]
    fn num_jobs(&self) -> usize {
        self.inner.num_jobs
    }

    #[getter]
    fn overrides(&self) -> Vec<String> {
        self.inner.overrides.clone()
    }

    #[getter]
    fn output_dir(&self) -> String {
        self.inner.output_dir.to_string_lossy().to_string()
    }

    /// Get the override dirname for directory naming
    #[pyo3(signature = (kv_sep = "=", item_sep = ",", exclude_keys = None))]
    fn get_override_dirname(
        &self,
        kv_sep: &str,
        item_sep: &str,
        exclude_keys: Option<Vec<String>>,
    ) -> String {
        self.inner.get_override_dirname(kv_sep, item_sep, &exclude_keys.unwrap_or_default())
    }
}

/// Python wrapper for SweepConfig
#[pyclass(name = "SweepConfig")]
#[derive(Clone)]
pub struct PySweepConfig {
    #[pyo3(get, set)]
    dir: String,
    #[pyo3(get, set)]
    subdir: String,
    #[pyo3(get, set)]
    max_batch_size: Option<usize>,
}

#[pymethods]
impl PySweepConfig {
    #[new]
    #[pyo3(signature = (dir, subdir = None, max_batch_size = None))]
    fn new(dir: &str, subdir: Option<&str>, max_batch_size: Option<usize>) -> Self {
        Self {
            dir: dir.to_string(),
            subdir: subdir.unwrap_or("").to_string(),
            max_batch_size,
        }
    }
}

/// Compute output directory for a job
#[pyfunction]
#[pyo3(signature = (base_dir, job_idx, overrides, use_override_dirname = false))]
fn compute_job_output_dir(
    base_dir: &str,
    job_idx: usize,
    overrides: Vec<String>,
    use_override_dirname: bool,
) -> String {
    compute_output_dir(base_dir, job_idx, &overrides, use_override_dirname)
        .to_string_lossy()
        .to_string()
}

/// Generate job configurations for a sweep
#[pyfunction]
fn generate_jobs(name: &str, sweep_overrides: Vec<Vec<String>>, base_dir: &str) -> Vec<PyJobConfig> {
    generate_sweep_jobs(name, &sweep_overrides, base_dir)
        .into_iter()
        .map(|j| PyJobConfig { inner: j })
        .collect()
}

/// Register this module
pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "job")?;
    m.add_class::<PyJobConfig>()?;
    m.add_class::<PySweepConfig>()?;
    m.add_function(wrap_pyfunction!(compute_job_output_dir, &m)?)?;
    m.add_function(wrap_pyfunction!(generate_jobs, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
