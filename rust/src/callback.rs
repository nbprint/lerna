// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Callback trait for lifecycle hooks
//!
//! This module provides:
//! - `Callback` trait with lifecycle methods matching Python's Callback class
//! - `NoOpCallback` default implementation that does nothing
//! - Support for optionally wrapping Python callbacks via PyO3

use std::collections::HashMap;
use std::sync::Arc;

use crate::config::value::ConfigDict;

/// Error type for callback operations
#[derive(Debug, Clone)]
pub struct CallbackError {
    pub message: String,
}

impl std::fmt::Display for CallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CallbackError {}

impl From<String> for CallbackError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for CallbackError {
    fn from(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

/// Result type for callback operations
pub type CallbackResult<T> = Result<T, CallbackError>;

/// Job return information passed to callbacks
#[derive(Clone, Debug, Default)]
pub struct JobReturn {
    /// Job return value (serialized as config)
    pub return_value: Option<ConfigDict>,
    /// Working directory
    pub working_dir: String,
    /// Output directory
    pub output_dir: String,
    /// Job name
    pub job_name: String,
    /// Task name
    pub task_name: String,
    /// Status code (0 = success)
    pub status_code: i32,
}

/// Callback trait for lifecycle hooks
///
/// This trait mirrors Python's `lerna.experimental.callback.Callback` class.
/// All methods have default no-op implementations, so you only need to
/// implement the ones you care about.
///
/// # Usage
///
/// ## Pure Rust (standalone)
/// ```rust
/// use lerna::callback::{Callback, NoOpCallback};
///
/// // Use default no-op callback
/// let callback = NoOpCallback;
/// callback.on_run_start(&config, &kwargs).unwrap();
///
/// // Or implement your own
/// struct MyCallback;
/// impl Callback for MyCallback {
///     fn on_job_start(&self, config: &ConfigDict, kwargs: &HashMap<String, String>) -> CallbackResult<()> {
///         println!("Job starting!");
///         Ok(())
///     }
/// }
/// ```
///
/// ## With Python callbacks (via PyO3)
/// When used from Python, the `PyCallback` wrapper (in src/callback/mod.rs)
/// implements this trait by delegating to Python methods.
pub trait Callback: Send + Sync {
    /// Called in RUN mode before job/application code starts.
    /// `config` is composed with overrides.
    fn on_run_start(
        &self,
        _config: &ConfigDict,
        _kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        Ok(())
    }

    /// Called in RUN mode after job/application code returns.
    fn on_run_end(
        &self,
        _config: &ConfigDict,
        _kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        Ok(())
    }

    /// Called in MULTIRUN mode before any job starts.
    fn on_multirun_start(
        &self,
        _config: &ConfigDict,
        _kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        Ok(())
    }

    /// Called in MULTIRUN mode after all jobs returns.
    fn on_multirun_end(
        &self,
        _config: &ConfigDict,
        _kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        Ok(())
    }

    /// Called once for each job (before running application code).
    /// Works in both RUN and MULTIRUN modes.
    fn on_job_start(
        &self,
        _config: &ConfigDict,
        _kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        Ok(())
    }

    /// Called once for each job (after running application code).
    /// Works in both RUN and MULTIRUN modes.
    fn on_job_end(
        &self,
        _config: &ConfigDict,
        _job_return: &JobReturn,
        _kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        Ok(())
    }

    /// Called during compose phase before config is returned.
    fn on_compose_config(
        &self,
        _config: &ConfigDict,
        _config_name: Option<&str>,
        _overrides: &[String],
    ) -> CallbackResult<()> {
        Ok(())
    }
}

/// No-op callback that does nothing
///
/// This is the default callback used when no callbacks are registered.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoOpCallback;

impl Callback for NoOpCallback {}

/// Callback manager that holds multiple callbacks
#[derive(Default)]
pub struct CallbackManager {
    callbacks: Vec<Arc<dyn Callback>>,
}

impl CallbackManager {
    /// Create a new empty callback manager
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    /// Add a callback
    pub fn add(&mut self, callback: Arc<dyn Callback>) {
        self.callbacks.push(callback);
    }

    /// Add a callback, consuming the manager and returning it
    pub fn with(mut self, callback: Arc<dyn Callback>) -> Self {
        self.add(callback);
        self
    }

    /// Check if there are any callbacks registered
    pub fn is_empty(&self) -> bool {
        self.callbacks.is_empty()
    }

    /// Get the number of callbacks
    pub fn len(&self) -> usize {
        self.callbacks.len()
    }

    /// Clear all callbacks
    pub fn clear(&mut self) {
        self.callbacks.clear();
    }
}

impl Callback for CallbackManager {
    fn on_run_start(
        &self,
        config: &ConfigDict,
        kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        for callback in &self.callbacks {
            callback.on_run_start(config, kwargs)?;
        }
        Ok(())
    }

    fn on_run_end(
        &self,
        config: &ConfigDict,
        kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        for callback in &self.callbacks {
            callback.on_run_end(config, kwargs)?;
        }
        Ok(())
    }

    fn on_multirun_start(
        &self,
        config: &ConfigDict,
        kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        for callback in &self.callbacks {
            callback.on_multirun_start(config, kwargs)?;
        }
        Ok(())
    }

    fn on_multirun_end(
        &self,
        config: &ConfigDict,
        kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        for callback in &self.callbacks {
            callback.on_multirun_end(config, kwargs)?;
        }
        Ok(())
    }

    fn on_job_start(
        &self,
        config: &ConfigDict,
        kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        for callback in &self.callbacks {
            callback.on_job_start(config, kwargs)?;
        }
        Ok(())
    }

    fn on_job_end(
        &self,
        config: &ConfigDict,
        job_return: &JobReturn,
        kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        for callback in &self.callbacks {
            callback.on_job_end(config, job_return, kwargs)?;
        }
        Ok(())
    }

    fn on_compose_config(
        &self,
        config: &ConfigDict,
        config_name: Option<&str>,
        overrides: &[String],
    ) -> CallbackResult<()> {
        for callback in &self.callbacks {
            callback.on_compose_config(config, config_name, overrides)?;
        }
        Ok(())
    }
}

/// A callback that logs lifecycle events (uses eprintln for simplicity)
#[derive(Clone, Copy, Debug, Default)]
pub struct LoggingCallback;

impl Callback for LoggingCallback {
    fn on_run_start(
        &self,
        _config: &ConfigDict,
        _kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        eprintln!("[Callback] on_run_start");
        Ok(())
    }

    fn on_run_end(
        &self,
        _config: &ConfigDict,
        _kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        eprintln!("[Callback] on_run_end");
        Ok(())
    }

    fn on_multirun_start(
        &self,
        _config: &ConfigDict,
        _kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        eprintln!("[Callback] on_multirun_start");
        Ok(())
    }

    fn on_multirun_end(
        &self,
        _config: &ConfigDict,
        _kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        eprintln!("[Callback] on_multirun_end");
        Ok(())
    }

    fn on_job_start(
        &self,
        _config: &ConfigDict,
        _kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        eprintln!("[Callback] on_job_start");
        Ok(())
    }

    fn on_job_end(
        &self,
        _config: &ConfigDict,
        job_return: &JobReturn,
        _kwargs: &HashMap<String, String>,
    ) -> CallbackResult<()> {
        eprintln!("[Callback] on_job_end: status={}", job_return.status_code);
        Ok(())
    }

    fn on_compose_config(
        &self,
        _config: &ConfigDict,
        config_name: Option<&str>,
        overrides: &[String],
    ) -> CallbackResult<()> {
        eprintln!(
            "[Callback] on_compose_config: config={:?}, overrides={:?}",
            config_name, overrides
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_callback() {
        let callback = NoOpCallback;
        let config = ConfigDict::new();
        let kwargs = HashMap::new();

        assert!(callback.on_run_start(&config, &kwargs).is_ok());
        assert!(callback.on_run_end(&config, &kwargs).is_ok());
        assert!(callback.on_job_start(&config, &kwargs).is_ok());
        assert!(callback
            .on_job_end(&config, &JobReturn::default(), &kwargs)
            .is_ok());
    }

    #[test]
    fn test_callback_manager() {
        let mut manager = CallbackManager::new();
        assert!(manager.is_empty());

        manager.add(Arc::new(NoOpCallback));
        assert_eq!(manager.len(), 1);

        let config = ConfigDict::new();
        let kwargs = HashMap::new();
        assert!(manager.on_run_start(&config, &kwargs).is_ok());
    }

    #[test]
    fn test_custom_callback() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingCallback {
            count: Arc<AtomicUsize>,
        }

        impl Callback for CountingCallback {
            fn on_job_start(
                &self,
                _config: &ConfigDict,
                _kwargs: &HashMap<String, String>,
            ) -> CallbackResult<()> {
                self.count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }

        let count = Arc::new(AtomicUsize::new(0));
        let callback = CountingCallback {
            count: count.clone(),
        };

        let config = ConfigDict::new();
        let kwargs = HashMap::new();

        callback.on_job_start(&config, &kwargs).unwrap();
        callback.on_job_start(&config, &kwargs).unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
}
