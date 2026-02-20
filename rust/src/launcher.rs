// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Launcher trait - defines the interface for job launchers
//!
//! Launchers are responsible for executing jobs, either locally
//! (BasicLauncher) or on remote systems (e.g., Submitit, RQ).

use std::fmt::Debug;
use std::sync::Arc;

use crate::callback::JobReturn;
use crate::config::value::ConfigDict;

/// Error type for launcher operations
#[derive(Debug, Clone)]
pub struct LauncherError {
    pub message: String,
}

impl LauncherError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for LauncherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LauncherError: {}", self.message)
    }
}

impl std::error::Error for LauncherError {}

/// Job overrides for a single job
pub type JobOverrides = Vec<String>;

/// Batch of job overrides (multiple jobs)
pub type JobOverrideBatch = Vec<JobOverrides>;

/// Launcher trait - implement this to create custom launchers
pub trait Launcher: Send + Sync + Debug {
    /// Setup the launcher with context
    ///
    /// This is called before launch() to provide:
    /// - config: The resolved Hydra config
    /// - task_info: Information about the task being run
    fn setup(&mut self, config: &ConfigDict, task_name: &str) -> Result<(), LauncherError>;

    /// Launch a batch of jobs
    ///
    /// # Arguments
    /// * `job_overrides` - A batch of jobs to run, each with their override list
    /// * `initial_job_idx` - Starting index for job numbering (used by sweepers)
    ///
    /// # Returns
    /// A vector of JobReturn results, one for each job
    fn launch(
        &self,
        job_overrides: &JobOverrideBatch,
        initial_job_idx: usize,
    ) -> Result<Vec<JobReturn>, LauncherError>;

    /// Get the launcher name/type
    fn name(&self) -> &str;
}

/// BasicLauncher - runs jobs locally and sequentially
#[derive(Debug, Default)]
pub struct BasicLauncher {
    config: Option<ConfigDict>,
    task_name: String,
}

impl BasicLauncher {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Launcher for BasicLauncher {
    fn setup(&mut self, config: &ConfigDict, task_name: &str) -> Result<(), LauncherError> {
        self.config = Some(config.clone());
        self.task_name = task_name.to_string();
        Ok(())
    }

    fn launch(
        &self,
        job_overrides: &JobOverrideBatch,
        initial_job_idx: usize,
    ) -> Result<Vec<JobReturn>, LauncherError> {
        let mut results = Vec::with_capacity(job_overrides.len());

        for (idx, _overrides) in job_overrides.iter().enumerate() {
            let job_idx = initial_job_idx + idx;

            // Create a basic JobReturn for this job
            // In practice, this would actually run the task
            let job_return = JobReturn {
                return_value: None,
                working_dir: std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                output_dir: format!("outputs/{}", job_idx),
                job_name: format!("job_{}", job_idx),
                task_name: self.task_name.clone(),
                status_code: 0,
            };
            results.push(job_return);
        }

        Ok(results)
    }

    fn name(&self) -> &str {
        "basic"
    }
}

/// Launcher manager - holds and manages launcher instances
#[derive(Default)]
pub struct LauncherManager {
    launcher: Option<Arc<dyn Launcher>>,
}

impl LauncherManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the active launcher
    pub fn set_launcher(&mut self, launcher: Arc<dyn Launcher>) {
        self.launcher = Some(launcher);
    }

    /// Set BasicLauncher as the active launcher
    pub fn set_basic_launcher(&mut self) {
        self.launcher = Some(Arc::new(BasicLauncher::new()));
    }

    /// Get current launcher
    pub fn launcher(&self) -> Option<&Arc<dyn Launcher>> {
        self.launcher.as_ref()
    }

    /// Launch jobs using the configured launcher
    pub fn launch(
        &self,
        job_overrides: &JobOverrideBatch,
        initial_job_idx: usize,
    ) -> Result<Vec<JobReturn>, LauncherError> {
        match &self.launcher {
            Some(l) => l.launch(job_overrides, initial_job_idx),
            None => Err(LauncherError::new("No launcher configured")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_launcher_setup() {
        let mut launcher = BasicLauncher::new();
        let config = ConfigDict::new();
        assert!(launcher.setup(&config, "my_task").is_ok());
        assert_eq!(launcher.name(), "basic");
    }

    #[test]
    fn test_basic_launcher_launch() {
        let mut launcher = BasicLauncher::new();
        let config = ConfigDict::new();
        launcher.setup(&config, "test_task").unwrap();

        let overrides = vec![
            vec!["db=mysql".to_string()],
            vec!["db=postgres".to_string()],
        ];

        let results = launcher.launch(&overrides, 0).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].job_name, "job_0");
        assert_eq!(results[1].job_name, "job_1");
    }

    #[test]
    fn test_launcher_manager() {
        let mut manager = LauncherManager::new();
        manager.set_basic_launcher();

        let overrides = vec![vec!["key=value".to_string()]];
        let results = manager.launch(&overrides, 0).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_launcher_manager_no_launcher() {
        let manager = LauncherManager::new();
        let overrides = vec![vec!["key=value".to_string()]];
        assert!(manager.launch(&overrides, 0).is_err());
    }
}
