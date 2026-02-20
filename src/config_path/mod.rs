// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyO3 bindings for configuration path utilities

use pyo3::prelude::*;

use lerna::{
    get_basename as rust_get_basename, get_parent_path as rust_get_parent,
    get_scheme as rust_get_scheme, is_absolute_config_path as rust_is_absolute,
    is_valid_config_name as rust_is_valid_config, is_valid_group_name as rust_is_valid_group,
    join_config_paths as rust_join, normalize_config_path as rust_normalize,
    strip_scheme as rust_strip_scheme,
};

/// Normalize a configuration path by removing redundant separators and dots
#[pyfunction]
pub fn normalize_config_path(path: &str) -> String {
    rust_normalize(path)
}

/// Get the parent directory of a configuration path
#[pyfunction]
pub fn get_parent_path(path: &str) -> Option<String> {
    rust_get_parent(path)
}

/// Get the basename (last component) of a configuration path
#[pyfunction]
pub fn get_basename(path: &str) -> String {
    rust_get_basename(path)
}

/// Join two configuration paths
#[pyfunction]
pub fn join_config_paths(base: &str, child: &str) -> String {
    rust_join(base, child)
}

/// Check if a path is absolute (starts with a known scheme or /)
#[pyfunction]
pub fn is_absolute_config_path(path: &str) -> bool {
    rust_is_absolute(path)
}

/// Strip a scheme from a path if present
#[pyfunction]
pub fn strip_scheme(path: &str) -> String {
    rust_strip_scheme(path).to_string()
}

/// Get the scheme from a path if present
#[pyfunction]
pub fn get_scheme(path: &str) -> Option<String> {
    rust_get_scheme(path).map(|s| s.to_string())
}

/// Validate that a config group name is valid
#[pyfunction]
pub fn is_valid_group_name(name: &str) -> bool {
    rust_is_valid_group(name)
}

/// Validate that a config name is valid (with optional .yaml extension)
#[pyfunction]
pub fn is_valid_config_name(name: &str) -> bool {
    rust_is_valid_config(name)
}
