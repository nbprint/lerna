// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! OmegaConf error types

use std::error::Error;
use std::fmt;

/// Base error type for OmegaConf operations
#[derive(Debug, Clone)]
pub enum OmegaConfError {
    MissingMandatoryValue(MissingMandatoryValue),
    ValidationError(ValidationError),
    ReadonlyConfigError(ReadonlyConfigError),
    KeyValidationError(KeyValidationError),
    ConfigTypeError(ConfigTypeError),
    InterpolationError(InterpolationError),
    InterpolationResolutionError(InterpolationResolutionError),
    KeyError(KeyError),
    IOError(IOError),
}

impl fmt::Display for OmegaConfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OmegaConfError::MissingMandatoryValue(e) => write!(f, "{}", e),
            OmegaConfError::ValidationError(e) => write!(f, "{}", e),
            OmegaConfError::ReadonlyConfigError(e) => write!(f, "{}", e),
            OmegaConfError::KeyValidationError(e) => write!(f, "{}", e),
            OmegaConfError::ConfigTypeError(e) => write!(f, "{}", e),
            OmegaConfError::InterpolationError(e) => write!(f, "{}", e),
            OmegaConfError::InterpolationResolutionError(e) => write!(f, "{}", e),
            OmegaConfError::KeyError(e) => write!(f, "{}", e),
            OmegaConfError::IOError(e) => write!(f, "{}", e),
        }
    }
}

impl Error for OmegaConfError {}

/// Error when trying to access a missing mandatory value
#[derive(Debug, Clone)]
pub struct MissingMandatoryValue {
    pub message: String,
    pub key: Option<String>,
    pub full_key: Option<String>,
}

impl MissingMandatoryValue {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            key: None,
            full_key: None,
        }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn with_full_key(mut self, full_key: impl Into<String>) -> Self {
        self.full_key = Some(full_key.into());
        self
    }
}

impl fmt::Display for MissingMandatoryValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut msg = self.message.clone();
        if let Some(ref key) = self.key {
            msg = msg.replace("$KEY", key);
        }
        if let Some(ref full_key) = self.full_key {
            msg = msg.replace("$FULL_KEY", full_key);
        }
        write!(f, "{}", msg)
    }
}

impl Error for MissingMandatoryValue {}

/// Error for validation failures
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    pub key: Option<String>,
    pub value: Option<String>,
}

impl ValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            key: None,
            value: None,
        }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut msg = self.message.clone();
        if let Some(ref key) = self.key {
            msg = msg.replace("$KEY", key);
        }
        if let Some(ref value) = self.value {
            msg = msg.replace("$VALUE", value);
        }
        write!(f, "{}", msg)
    }
}

impl Error for ValidationError {}

/// Error when trying to modify a readonly config
#[derive(Debug, Clone)]
pub struct ReadonlyConfigError {
    pub message: String,
}

impl ReadonlyConfigError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ReadonlyConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ReadonlyConfigError {}

/// Error for invalid keys
#[derive(Debug, Clone)]
pub struct KeyValidationError {
    pub message: String,
    pub key: Option<String>,
}

impl KeyValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            key: None,
        }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }
}

impl fmt::Display for KeyValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut msg = self.message.clone();
        if let Some(ref key) = self.key {
            msg = msg.replace("$KEY", key);
            msg = msg.replace("$KEY_TYPE", &format!("{}", std::any::type_name_of_val(key)));
        }
        write!(f, "{}", msg)
    }
}

impl Error for KeyValidationError {}

/// Error for type-related config issues
#[derive(Debug, Clone)]
pub struct ConfigTypeError {
    pub message: String,
}

impl ConfigTypeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ConfigTypeError {}

/// Error for interpolation resolution failures
#[derive(Debug, Clone)]
pub struct InterpolationError {
    pub message: String,
    pub key: Option<String>,
}

impl InterpolationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            key: None,
        }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }
}

impl fmt::Display for InterpolationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for InterpolationError {}

/// Conversion implementations
impl From<MissingMandatoryValue> for OmegaConfError {
    fn from(e: MissingMandatoryValue) -> Self {
        OmegaConfError::MissingMandatoryValue(e)
    }
}

impl From<ValidationError> for OmegaConfError {
    fn from(e: ValidationError) -> Self {
        OmegaConfError::ValidationError(e)
    }
}

impl From<ReadonlyConfigError> for OmegaConfError {
    fn from(e: ReadonlyConfigError) -> Self {
        OmegaConfError::ReadonlyConfigError(e)
    }
}

impl From<KeyValidationError> for OmegaConfError {
    fn from(e: KeyValidationError) -> Self {
        OmegaConfError::KeyValidationError(e)
    }
}

impl From<ConfigTypeError> for OmegaConfError {
    fn from(e: ConfigTypeError) -> Self {
        OmegaConfError::ConfigTypeError(e)
    }
}

impl From<InterpolationError> for OmegaConfError {
    fn from(e: InterpolationError) -> Self {
        OmegaConfError::InterpolationError(e)
    }
}

/// Error for interpolation resolution failures (when the referenced value cannot be found)
#[derive(Debug, Clone)]
pub struct InterpolationResolutionError {
    pub message: String,
}

impl InterpolationResolutionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for InterpolationResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for InterpolationResolutionError {}

impl From<InterpolationResolutionError> for OmegaConfError {
    fn from(e: InterpolationResolutionError) -> Self {
        OmegaConfError::InterpolationResolutionError(e)
    }
}

/// Error for key not found
#[derive(Debug, Clone)]
pub struct KeyError {
    pub message: String,
}

impl KeyError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for KeyError {}

impl From<KeyError> for OmegaConfError {
    fn from(e: KeyError) -> Self {
        OmegaConfError::KeyError(e)
    }
}

/// Error for IO operations (file loading, etc.)
#[derive(Debug, Clone)]
pub struct IOError {
    pub message: String,
}

impl IOError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for IOError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for IOError {}

impl From<IOError> for OmegaConfError {
    fn from(e: IOError) -> Self {
        OmegaConfError::IOError(e)
    }
}

pub type Result<T> = std::result::Result<T, OmegaConfError>;
