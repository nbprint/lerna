// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Configuration loading and management

pub mod interpolation;
pub mod loader;
pub mod parser;
pub mod repository;
pub mod source;
pub mod value;

pub use interpolation::{resolve, InterpolationError, ResolverContext};
pub use loader::{CachingConfigLoader, ConfigLoader, SearchPathEntry};
pub use parser::{extract_header, load_yaml_file, parse_yaml, ConfigLoadError};
pub use repository::{
    get_scheme as get_path_scheme, CachingConfigRepository, ConfigRepository, SearchPathElement,
};
pub use source::{ConfigResult, ConfigSource, FileConfigSource};
pub use value::{ConfigDict, ConfigValue};
