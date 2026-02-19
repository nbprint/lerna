// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! OmegaConf reimplementation in Rust
//!
//! This module provides a Rust implementation of OmegaConf's core functionality:
//! - DictConfig and ListConfig containers
//! - ValueNode types (AnyNode, StringNode, IntegerNode, etc.)
//! - Interpolation resolution
//! - Merging with type validation
//! - Flags (struct, readonly)
//! - MISSING value support

pub mod base;
pub mod dictconfig;
pub mod errors;
pub mod flags;
pub mod listconfig;
pub mod nodes;
pub mod omegaconf;

pub use base::{
    Box as OmegaBox, Container, ContainerMetadata, Metadata, Node, NodeContent, NodeType, NodeValue,
};
pub use dictconfig::DictConfig;
pub use errors::{
    KeyValidationError, MissingMandatoryValue, OmegaConfError, ReadonlyConfigError, ValidationError,
};
pub use flags::Flags;
pub use listconfig::ListConfig;
pub use nodes::{AnyNode, BooleanNode, FloatNode, IntegerNode, StringNode, ValueNode};
pub use omegaconf::{ConfigValue, ListMergeMode, OmegaConf, SCMode};

/// The MISSING value marker
pub const MISSING: &str = "???";

/// Check if a value is the MISSING marker
pub fn is_missing_literal(value: &str) -> bool {
    value == MISSING
}

/// Check if a value is None
pub fn is_none(value: Option<&str>) -> bool {
    value.is_none()
}
