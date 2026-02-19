// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! PyO3 bindings for utility functions

use pyo3::prelude::*;

use lerna::{
    escape_special_characters as rust_escape,
    unescape_string as rust_unescape,
    is_valid_key as rust_is_valid_key,
    split_key as rust_split_key,
    join_key as rust_join_key,
    normalize_file_name as rust_normalize_file_name,
    get_valid_filename as rust_get_valid_filename,
    sanitize_path_component as rust_sanitize_path_component,
};


/// Escape special characters in a string for use in configuration values
#[pyfunction]
pub fn escape_special_characters(s: &str) -> String {
    rust_escape(s)
}

/// Unescape a string by removing escape sequences
#[pyfunction]
pub fn unescape_string(s: &str) -> String {
    rust_unescape(s)
}

/// Check if a key name is valid
#[pyfunction]
pub fn is_valid_key(key: &str) -> bool {
    rust_is_valid_key(key)
}

/// Split a dotted key into parts
#[pyfunction]
pub fn split_key(key: &str) -> Vec<String> {
    rust_split_key(key).into_iter().map(|s| s.to_string()).collect()
}

/// Join key parts into a dotted key
#[pyfunction]
pub fn join_key(parts: Vec<String>) -> String {
    let refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
    rust_join_key(&refs)
}

/// Normalize a config file name by adding .yaml extension if needed
#[pyfunction]
pub fn normalize_file_name(filename: &str) -> String {
    rust_normalize_file_name(filename)
}

/// Get a valid filename by stripping invalid characters
#[pyfunction]
pub fn get_valid_filename(s: &str) -> String {
    rust_get_valid_filename(s)
}

/// Sanitize a string for use in a file path
#[pyfunction]
pub fn sanitize_path_component(s: &str) -> String {
    rust_sanitize_path_component(s)
}
