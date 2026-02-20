// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Job configuration module for multirun/sweep jobs
//!
//! Handles job naming, output directory computation, and job metadata.

use std::path::PathBuf;

/// Job configuration for a single run
#[derive(Clone, Debug)]
pub struct JobConfig {
    /// Job name
    pub name: String,
    /// Job index within the sweep
    pub idx: usize,
    /// Total number of jobs in the sweep
    pub num_jobs: usize,
    /// Overrides for this job
    pub overrides: Vec<String>,
    /// Working directory
    pub cwd: PathBuf,
    /// Output directory
    pub output_dir: PathBuf,
}

impl JobConfig {
    pub fn new(name: &str, idx: usize, overrides: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            idx,
            num_jobs: 1,
            overrides,
            cwd: PathBuf::new(),
            output_dir: PathBuf::new(),
        }
    }

    /// Set the output directory based on sweep configuration
    pub fn with_output_dir(mut self, base_dir: &str, subdir: &str) -> Self {
        self.output_dir = PathBuf::from(base_dir)
            .join(subdir)
            .join(self.idx.to_string());
        self
    }

    /// Get the override dirname (for directory naming)
    pub fn get_override_dirname(
        &self,
        kv_sep: &str,
        item_sep: &str,
        exclude_keys: &[String],
    ) -> String {
        let mut lines: Vec<String> = self
            .overrides
            .iter()
            .filter(|o| {
                if let Some(eq_pos) = o.find('=') {
                    let key = &o[..eq_pos];
                    !exclude_keys.contains(&key.to_string())
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        lines.sort();
        lines.join(item_sep).replace('=', kv_sep)
    }
}

/// Compute output directory for a job
pub fn compute_output_dir(
    base_dir: &str,
    job_idx: usize,
    overrides: &[String],
    use_override_dirname: bool,
) -> PathBuf {
    let mut path = PathBuf::from(base_dir);

    if use_override_dirname && !overrides.is_empty() {
        // Create directory based on overrides
        let mut parts: Vec<String> = overrides
            .iter()
            .map(|o| o.replace('=', "_").replace(',', "_"))
            .collect();
        parts.sort();
        path.push(parts.join("_"));
    } else {
        path.push(job_idx.to_string());
    }

    path
}

/// Generate job configurations for a sweep
pub fn generate_sweep_jobs(
    name: &str,
    sweep_overrides: &[Vec<String>],
    base_dir: &str,
) -> Vec<JobConfig> {
    let num_jobs = sweep_overrides.len();

    sweep_overrides
        .iter()
        .enumerate()
        .map(|(idx, overrides)| {
            let mut job = JobConfig::new(name, idx, overrides.clone());
            job.num_jobs = num_jobs;
            job.output_dir = PathBuf::from(base_dir).join(idx.to_string());
            job
        })
        .collect()
}

/// Configuration for sweep/multirun execution
#[derive(Clone, Debug, Default)]
pub struct SweepConfig {
    /// Base output directory
    pub dir: String,
    /// Subdirectory pattern
    pub subdir: String,
    /// Maximum batch size for parallel execution
    pub max_batch_size: Option<usize>,
}

impl SweepConfig {
    pub fn new(dir: &str) -> Self {
        Self {
            dir: dir.to_string(),
            subdir: String::new(),
            max_batch_size: None,
        }
    }

    pub fn with_subdir(mut self, subdir: &str) -> Self {
        self.subdir = subdir.to_string();
        self
    }

    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.max_batch_size = Some(size);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_config() {
        let job = JobConfig::new("myapp", 0, vec!["db=mysql".to_string()]);
        assert_eq!(job.name, "myapp");
        assert_eq!(job.idx, 0);
        assert_eq!(job.overrides.len(), 1);
    }

    #[test]
    fn test_override_dirname() {
        let job = JobConfig::new(
            "myapp",
            0,
            vec!["db=mysql".to_string(), "port=3306".to_string()],
        );
        let dirname = job.get_override_dirname("_", ",", &[]);
        assert!(dirname.contains("db_mysql"));
        assert!(dirname.contains("port_3306"));
    }

    #[test]
    fn test_override_dirname_exclude() {
        let job = JobConfig::new(
            "myapp",
            0,
            vec!["db=mysql".to_string(), "port=3306".to_string()],
        );
        let dirname = job.get_override_dirname("_", ",", &["port".to_string()]);
        assert!(dirname.contains("db_mysql"));
        assert!(!dirname.contains("port"));
    }

    #[test]
    fn test_compute_output_dir() {
        let dir = compute_output_dir("/output", 0, &[], false);
        assert_eq!(dir.to_str().unwrap(), "/output/0");
    }

    #[test]
    fn test_generate_sweep_jobs() {
        let sweeps = vec![
            vec!["db=mysql".to_string()],
            vec!["db=postgres".to_string()],
        ];
        let jobs = generate_sweep_jobs("myapp", &sweeps, "/output");
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].idx, 0);
        assert_eq!(jobs[1].idx, 1);
        assert_eq!(jobs[0].num_jobs, 2);
    }

    #[test]
    fn test_sweep_config() {
        let config = SweepConfig::new("/multirun")
            .with_subdir("${now:%Y-%m-%d}")
            .with_batch_size(4);

        assert_eq!(config.dir, "/multirun");
        assert_eq!(config.max_batch_size, Some(4));
    }
}
