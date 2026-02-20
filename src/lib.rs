// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Lerna - A Rust implementation of core Hydra types
//!
//! This crate provides PyO3 bindings for performance-critical Hydra components.

use pyo3::prelude::*;

mod callback;
mod config;
mod config_path;
mod config_source;
mod config_store;
mod core;
mod defaults;
mod defaults_list;
mod env;
mod example;
mod glob;
mod interpolation;
mod job;
mod job_runner;
mod launcher;
mod merge;
mod omegaconf;
mod override_types;
mod package;
mod parser;
mod search_path;
mod sweep;
mod sweeper;
mod utils;
mod validation;

pub use example::Example;

#[pymodule]
fn lerna(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    // Core types
    m.add_class::<core::PyObjectType>()?;

    // Override types
    m.add_class::<override_types::PyQuote>()?;
    m.add_class::<override_types::PyQuotedString>()?;
    m.add_class::<override_types::PyOverrideType>()?;
    m.add_class::<override_types::PyValueType>()?;
    m.add_class::<override_types::PyKey>()?;

    // Parser types
    m.add_class::<parser::PyOverride>()?;
    m.add_class::<parser::PyOverrideParser>()?;

    // Default element types
    defaults::register(m)?;

    // Glob pattern matching
    glob::register(m)?;

    // Config loading
    config::register(m)?;

    // Sweep expansion
    sweep::register(m)?;

    // Defaults list builder
    defaults_list::register(m)?;

    // Validation
    validation::register(m)?;

    // Job configuration
    job::register(m)?;

    // Interpolation resolution
    interpolation::register(m)?;

    // Package resolution
    package::register(m)?;

    // Config merging
    merge::register(m)?;

    // Search path management
    search_path::register(m)?;

    // Environment variable resolution
    env::register(m)?;

    // ConfigStore singleton
    config_store::register(m)?;

    // Job runner utilities
    job_runner::register(m)?;

    // TODO: Callback system - add after fixing module creation
    // callback::register(m)?;

    // Utility functions
    m.add_function(wrap_pyfunction!(utils::escape_special_characters, m)?)?;
    m.add_function(wrap_pyfunction!(utils::unescape_string, m)?)?;
    m.add_function(wrap_pyfunction!(utils::is_valid_key, m)?)?;
    m.add_function(wrap_pyfunction!(utils::split_key, m)?)?;
    m.add_function(wrap_pyfunction!(utils::join_key, m)?)?;
    m.add_function(wrap_pyfunction!(utils::normalize_file_name, m)?)?;
    m.add_function(wrap_pyfunction!(utils::get_valid_filename, m)?)?;
    m.add_function(wrap_pyfunction!(utils::sanitize_path_component, m)?)?;

    // Config path functions
    m.add_function(wrap_pyfunction!(config_path::normalize_config_path, m)?)?;
    m.add_function(wrap_pyfunction!(config_path::get_parent_path, m)?)?;
    m.add_function(wrap_pyfunction!(config_path::get_basename, m)?)?;
    m.add_function(wrap_pyfunction!(config_path::join_config_paths, m)?)?;
    m.add_function(wrap_pyfunction!(config_path::is_absolute_config_path, m)?)?;
    m.add_function(wrap_pyfunction!(config_path::strip_scheme, m)?)?;
    m.add_function(wrap_pyfunction!(config_path::get_scheme, m)?)?;
    m.add_function(wrap_pyfunction!(config_path::is_valid_group_name, m)?)?;
    m.add_function(wrap_pyfunction!(config_path::is_valid_config_name, m)?)?;

    // OmegaConf bindings
    omegaconf::register(m)?;

    // Callback bindings
    callback::register(m)?;

    // ConfigSource bindings
    config_source::register(m)?;

    // Launcher bindings
    launcher::register(m)?;

    // Sweeper bindings
    sweeper::register(m)?;

    // Legacy example
    m.add_class::<Example>()?;

    Ok(())
}
