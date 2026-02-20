// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyO3 bindings for sweep expansion

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Expand sweep overrides into individual override sets.
///
/// Given a list of overrides like:
/// - "db=mysql,postgresql"
/// - "server=dev,prod"
///
/// Returns a list of override sets (cartesian product):
/// - ["db=mysql", "server=dev"]
/// - ["db=mysql", "server=prod"]
/// - ["db=postgresql", "server=dev"]
/// - ["db=postgresql", "server=prod"]
///
/// Also supports range sweeps like "x=range(1,10)".
#[pyfunction]
pub fn expand_sweeps(py: Python, overrides: Vec<String>) -> PyResult<Py<PyList>> {
    let override_refs: Vec<&str> = overrides.iter().map(|s| s.as_str()).collect();
    let result = lerna::expand_simple_sweeps(&override_refs);

    let outer_list = PyList::empty(py);
    for combo in result {
        let inner_list = PyList::new(py, combo)?;
        outer_list.append(inner_list)?;
    }

    Ok(outer_list.into())
}

/// Get the number of combinations for a set of overrides.
///
/// This is useful for determining sweep size without expanding.
#[pyfunction]
pub fn count_sweep_combinations(overrides: Vec<String>) -> usize {
    let override_refs: Vec<&str> = overrides.iter().map(|s| s.as_str()).collect();
    let result = lerna::expand_simple_sweeps(&override_refs);
    result.len()
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(expand_sweeps, m)?)?;
    m.add_function(wrap_pyfunction!(count_sweep_combinations, m)?)?;
    Ok(())
}
