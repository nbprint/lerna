// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Lerna - A configuration management library

pub mod callback;
pub mod config;
pub mod config_path;
pub mod config_store;
pub mod core;
pub mod defaults;
pub mod defaults_list;
pub mod env;
pub mod glob;
pub mod interpolation;
pub mod job;
pub mod job_runner;
pub mod launcher;
pub mod merge;
pub mod omegaconf;
pub mod package;
pub mod parser;
pub mod search_path;
pub mod sweep;
pub mod sweeper;
pub mod utils;
pub mod validation;

pub use callback::{
    Callback, CallbackError, CallbackManager, CallbackResult, JobReturn, LoggingCallback,
    NoOpCallback,
};
pub use config::{
    get_path_scheme, CachingConfigRepository, ConfigDict, ConfigLoadError, ConfigLoader,
    ConfigRepository, ConfigResult, ConfigValue, SearchPathElement, SearchPathEntry,
};
pub use config_path::*;
pub use config_store::{instance as config_store_instance, ConfigNode, ConfigStore};
pub use core::object_type::ObjectType;
pub use core::override_types::*;
pub use defaults::*;
pub use glob::Glob;
pub use job_runner::{
    compute_output_dir as compute_job_output_dir, create_output_dirs, save_config_file,
    save_overrides_file, serialize_config_to_yaml, setup_job_environment, JobContext,
    JobResult as JobRunnerResult, JobStatus,
};
pub use launcher::{
    BasicLauncher, JobOverrideBatch, JobOverrides, Launcher, LauncherError, LauncherManager,
};
pub use parser::{FunctionCallback, OverrideParser};
pub use sweep::{expand_simple_sweeps, expand_sweeps};
pub use sweeper::{BasicSweeper, Sweeper, SweeperError, SweeperManager};
pub use utils::{
    escape_special_characters, get_valid_filename, is_special_char, is_valid_key, join_key,
    normalize_file_name, sanitize_path_component, split_key, unescape_string,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Example {
    pub stuff: String,
}

impl Example {
    pub fn new(value: String) -> Self {
        Example { stuff: value }
    }
}
