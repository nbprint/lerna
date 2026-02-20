// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Sweeper trait - defines the interface for parameter sweepers
//!
//! Sweepers are responsible for generating job parameter combinations
//! and coordinating job execution through a Launcher.

use std::fmt::Debug;
use std::sync::Arc;

use crate::callback::JobReturn;
use crate::config::value::ConfigDict;
use crate::launcher::{JobOverrideBatch, Launcher, LauncherError};

/// Error type for sweeper operations
#[derive(Debug, Clone)]
pub struct SweeperError {
    pub message: String,
}

impl SweeperError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SweeperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SweeperError: {}", self.message)
    }
}

impl std::error::Error for SweeperError {}

impl From<LauncherError> for SweeperError {
    fn from(err: LauncherError) -> Self {
        Self::new(err.message)
    }
}

/// Sweeper trait - implement this to create custom sweepers
pub trait Sweeper: Send + Sync + Debug {
    /// Setup the sweeper with context
    ///
    /// This is called before sweep() to provide:
    /// - config: The resolved Hydra config
    /// - launcher: The launcher to use for job execution
    fn setup(
        &mut self,
        config: &ConfigDict,
        launcher: Arc<dyn Launcher>,
    ) -> Result<(), SweeperError>;

    /// Execute a sweep with the given arguments
    ///
    /// # Arguments
    /// * `arguments` - Override arguments from command line
    ///
    /// # Returns
    /// A vector of JobReturn results from all launched jobs
    fn sweep(&self, arguments: &[String]) -> Result<Vec<JobReturn>, SweeperError>;

    /// Get the sweeper name/type
    fn name(&self) -> &str;
}

/// BasicSweeper - generates cartesian product of parameter values
#[derive(Debug)]
pub struct BasicSweeper {
    config: Option<ConfigDict>,
    launcher: Option<Arc<dyn Launcher>>,
    max_batch_size: Option<usize>,
}

impl Default for BasicSweeper {
    fn default() -> Self {
        Self::new(None)
    }
}

impl BasicSweeper {
    pub fn new(max_batch_size: Option<usize>) -> Self {
        Self {
            config: None,
            launcher: None,
            max_batch_size,
        }
    }

    /// Parse arguments into sweep combinations
    ///
    /// Handles patterns like:
    /// - `key=value` - single value
    /// - `key=a,b,c` - comma-separated sweep
    fn parse_sweep_arguments(&self, arguments: &[String]) -> Vec<Vec<(String, Vec<String>)>> {
        let mut param_lists: Vec<(String, Vec<String>)> = Vec::new();

        for arg in arguments {
            if let Some((key, value)) = arg.split_once('=') {
                let values: Vec<String> = if value.contains(',') && !value.contains('[') {
                    // Comma-separated sweep
                    value.split(',').map(|s| s.trim().to_string()).collect()
                } else {
                    // Single value
                    vec![value.to_string()]
                };
                param_lists.push((key.to_string(), values));
            }
        }

        // Generate cartesian product
        self.cartesian_product(&param_lists)
    }

    /// Generate cartesian product of parameter values
    fn cartesian_product(
        &self,
        params: &[(String, Vec<String>)],
    ) -> Vec<Vec<(String, Vec<String>)>> {
        if params.is_empty() {
            return vec![vec![]];
        }

        let mut combinations: Vec<Vec<String>> = vec![vec![]];

        for (key, values) in params {
            let mut new_combinations = Vec::new();
            for combo in &combinations {
                for value in values {
                    let mut new_combo = combo.clone();
                    new_combo.push(format!("{}={}", key, value));
                    new_combinations.push(new_combo);
                }
            }
            combinations = new_combinations;
        }

        // Return as single batch (BasicSweeper returns override strings, not tuples)
        combinations
            .into_iter()
            .map(|c| c.into_iter().map(|s| (s.clone(), vec![s])).collect())
            .collect()
    }

    /// Split combinations into batches
    fn split_into_batches(&self, combinations: Vec<Vec<String>>) -> Vec<Vec<Vec<String>>> {
        match self.max_batch_size {
            None => vec![combinations],
            Some(size) if size == 0 => vec![combinations],
            Some(size) => combinations.chunks(size).map(|c| c.to_vec()).collect(),
        }
    }
}

impl Sweeper for BasicSweeper {
    fn setup(
        &mut self,
        config: &ConfigDict,
        launcher: Arc<dyn Launcher>,
    ) -> Result<(), SweeperError> {
        self.config = Some(config.clone());
        self.launcher = Some(launcher);
        Ok(())
    }

    fn sweep(&self, arguments: &[String]) -> Result<Vec<JobReturn>, SweeperError> {
        let launcher = self
            .launcher
            .as_ref()
            .ok_or_else(|| SweeperError::new("Sweeper not set up - no launcher"))?;

        // Parse arguments and generate combinations
        let mut all_combinations: Vec<Vec<String>> = Vec::new();

        // Simple parsing: split comma-separated values
        let mut param_values: Vec<(String, Vec<String>)> = Vec::new();

        for arg in arguments {
            if let Some((key, value)) = arg.split_once('=') {
                let values: Vec<String> = if value.contains(',') && !value.starts_with('[') {
                    value.split(',').map(|s| s.trim().to_string()).collect()
                } else {
                    vec![value.to_string()]
                };
                param_values.push((key.to_string(), values));
            }
        }

        // Generate cartesian product
        if param_values.is_empty() {
            all_combinations.push(vec![]);
        } else {
            let mut combos: Vec<Vec<String>> = vec![vec![]];
            for (key, values) in &param_values {
                let mut new_combos = Vec::new();
                for combo in &combos {
                    for value in values {
                        let mut new_combo = combo.clone();
                        new_combo.push(format!("{}={}", key, value));
                        new_combos.push(new_combo);
                    }
                }
                combos = new_combos;
            }
            all_combinations = combos;
        }

        // Split into batches
        let batches = self.split_into_batches(all_combinations);

        // Launch all batches
        let mut all_results = Vec::new();
        let mut job_idx = 0;

        for batch in batches {
            let results = launcher.launch(&batch, job_idx)?;
            job_idx += results.len();
            all_results.extend(results);
        }

        Ok(all_results)
    }

    fn name(&self) -> &str {
        "basic"
    }
}

/// Sweeper manager - holds and manages sweeper instances
#[derive(Default)]
pub struct SweeperManager {
    sweeper: Option<Arc<dyn Sweeper>>,
}

impl SweeperManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the active sweeper
    pub fn set_sweeper(&mut self, sweeper: Arc<dyn Sweeper>) {
        self.sweeper = Some(sweeper);
    }

    /// Set BasicSweeper as the active sweeper
    pub fn set_basic_sweeper(&mut self, max_batch_size: Option<usize>) {
        self.sweeper = Some(Arc::new(BasicSweeper::new(max_batch_size)));
    }

    /// Get current sweeper
    pub fn sweeper(&self) -> Option<&Arc<dyn Sweeper>> {
        self.sweeper.as_ref()
    }

    /// Sweep with the configured sweeper
    pub fn sweep(&self, arguments: &[String]) -> Result<Vec<JobReturn>, SweeperError> {
        match &self.sweeper {
            Some(s) => s.sweep(arguments),
            None => Err(SweeperError::new("No sweeper configured")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher::BasicLauncher;

    #[test]
    fn test_basic_sweeper_setup() {
        let mut sweeper = BasicSweeper::new(None);
        let config = ConfigDict::new();
        let launcher = Arc::new(BasicLauncher::new());
        assert!(sweeper.setup(&config, launcher.clone()).is_ok());
        assert_eq!(sweeper.name(), "basic");
    }

    #[test]
    fn test_basic_sweeper_single_value() {
        let mut sweeper = BasicSweeper::new(None);
        let config = ConfigDict::new();
        let mut launcher = BasicLauncher::new();
        launcher.setup(&config, "test").unwrap();
        let launcher = Arc::new(launcher);
        sweeper.setup(&config, launcher.clone()).unwrap();

        let args = vec!["key=value".to_string()];
        let results = sweeper.sweep(&args).unwrap();

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_basic_sweeper_cartesian_product() {
        let mut sweeper = BasicSweeper::new(None);
        let config = ConfigDict::new();
        let mut launcher = BasicLauncher::new();
        launcher.setup(&config, "test").unwrap();
        let launcher = Arc::new(launcher);
        sweeper.setup(&config, launcher.clone()).unwrap();

        // a=1,2 b=x,y should produce 4 combinations
        let args = vec!["a=1,2".to_string(), "b=x,y".to_string()];
        let results = sweeper.sweep(&args).unwrap();

        assert_eq!(results.len(), 4);
    }

    #[test]
    fn test_basic_sweeper_with_batch_size() {
        let mut sweeper = BasicSweeper::new(Some(2));
        let config = ConfigDict::new();
        let mut launcher = BasicLauncher::new();
        launcher.setup(&config, "test").unwrap();
        let launcher = Arc::new(launcher);
        sweeper.setup(&config, launcher.clone()).unwrap();

        // 4 combinations, batch size 2 = 2 batches
        let args = vec!["a=1,2".to_string(), "b=x,y".to_string()];
        let results = sweeper.sweep(&args).unwrap();

        assert_eq!(results.len(), 4);
    }

    #[test]
    fn test_sweeper_manager() {
        let mut manager = SweeperManager::new();
        manager.set_basic_sweeper(None);
        assert!(manager.sweeper().is_some());
    }

    #[test]
    fn test_sweeper_manager_no_sweeper() {
        let manager = SweeperManager::new();
        assert!(manager.sweep(&[]).is_err());
    }
}
