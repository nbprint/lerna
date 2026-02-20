// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Job runner module - handles job execution setup and teardown
//!
//! This module manages:
//! - Output directory creation
//! - Config file serialization (config.yaml, hydra.yaml, overrides.yaml)
//! - Job state tracking

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::value::{ConfigDict, ConfigValue};

/// Job execution context
#[derive(Clone, Debug)]
pub struct JobContext {
    /// Job name
    pub name: String,
    /// Job ID
    pub id: String,
    /// Job number (index in sweep)
    pub num: usize,
    /// Output directory (absolute path)
    pub output_dir: PathBuf,
    /// Working directory (where job runs)
    pub working_dir: PathBuf,
    /// Original working directory (before chdir)
    pub original_cwd: PathBuf,
    /// Whether to change directory to output_dir
    pub chdir: bool,
    /// Overrides for this job
    pub overrides: Vec<String>,
}

impl JobContext {
    pub fn new(name: &str, id: &str, num: usize) -> Self {
        Self {
            name: name.to_string(),
            id: id.to_string(),
            num,
            output_dir: PathBuf::new(),
            working_dir: PathBuf::new(),
            original_cwd: std::env::current_dir().unwrap_or_default(),
            chdir: false,
            overrides: Vec::new(),
        }
    }

    /// Set the output directory
    pub fn with_output_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.output_dir = dir.as_ref().to_path_buf();
        self
    }

    /// Set chdir behavior
    pub fn with_chdir(mut self, chdir: bool) -> Self {
        self.chdir = chdir;
        if chdir {
            self.working_dir = self.output_dir.clone();
        } else {
            self.working_dir = self.original_cwd.clone();
        }
        self
    }

    /// Set overrides
    pub fn with_overrides(mut self, overrides: Vec<String>) -> Self {
        self.overrides = overrides;
        self
    }
}

/// Job execution result status
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobStatus {
    Unknown = 0,
    Completed = 1,
    Failed = 2,
}

/// Result of job execution
#[derive(Clone, Debug)]
pub struct JobResult {
    /// Job execution status
    pub status: JobStatus,
    /// Task name
    pub task_name: String,
    /// Working directory during execution
    pub working_dir: PathBuf,
    /// Overrides applied
    pub overrides: Vec<String>,
}

impl Default for JobResult {
    fn default() -> Self {
        Self {
            status: JobStatus::Unknown,
            task_name: String::new(),
            working_dir: PathBuf::new(),
            overrides: Vec::new(),
        }
    }
}

/// Compute the output directory for a job
pub fn compute_output_dir(job_dir_key_value: &str, job_subdir_key_value: Option<&str>) -> PathBuf {
    let mut output_dir = PathBuf::from(job_dir_key_value);
    if let Some(subdir) = job_subdir_key_value {
        output_dir = output_dir.join(subdir);
    }
    // Make absolute
    if output_dir.is_relative() {
        if let Ok(cwd) = std::env::current_dir() {
            output_dir = cwd.join(output_dir);
        }
    }
    output_dir
}

/// Create output directory structure
pub fn create_output_dirs(output_dir: &Path, subdir: Option<&str>) -> std::io::Result<PathBuf> {
    let full_path = if let Some(sub) = subdir {
        output_dir.join(sub)
    } else {
        output_dir.to_path_buf()
    };
    fs::create_dir_all(&full_path)?;
    Ok(full_path)
}

/// Convert ConfigValue to YAML string
fn config_value_to_yaml(value: &ConfigValue, indent: usize) -> String {
    let prefix = "  ".repeat(indent);
    match value {
        ConfigValue::Null => "null".to_string(),
        ConfigValue::Bool(b) => b.to_string(),
        ConfigValue::Int(i) => i.to_string(),
        ConfigValue::Float(f) => {
            if f.is_nan() {
                ".nan".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 {
                    ".inf".to_string()
                } else {
                    "-.inf".to_string()
                }
            } else {
                f.to_string()
            }
        }
        ConfigValue::String(s) => {
            // Quote if needed
            if s.contains(':')
                || s.contains('#')
                || s.contains('\n')
                || s.starts_with(' ')
                || s.ends_with(' ')
            {
                format!("'{}'", s.replace('\'', "''"))
            } else {
                s.clone()
            }
        }
        ConfigValue::Interpolation(s) => {
            // Always quote interpolations
            format!("'{}'", s)
        }
        ConfigValue::Missing => "???".to_string(),
        ConfigValue::List(items) => {
            if items.is_empty() {
                "[]".to_string()
            } else {
                let mut lines = Vec::new();
                for item in items {
                    let val = config_value_to_yaml(item, 0);
                    lines.push(format!("{}  - {}", prefix, val));
                }
                format!("\n{}", lines.join("\n"))
            }
        }
        ConfigValue::Dict(dict) => {
            if dict.is_empty() {
                "{}".to_string()
            } else {
                config_dict_to_yaml(dict, indent + 1)
            }
        }
    }
}

/// Convert ConfigDict to YAML string
fn config_dict_to_yaml(dict: &ConfigDict, indent: usize) -> String {
    let prefix = "  ".repeat(indent);
    let mut lines = Vec::new();

    // Sort keys for consistent output
    let mut keys: Vec<_> = dict.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(value) = dict.get(key) {
            let val_str = config_value_to_yaml(value, indent);
            if val_str.starts_with('\n') {
                lines.push(format!("{}{}:{}", prefix, key, val_str));
            } else {
                lines.push(format!("{}{}: {}", prefix, key, val_str));
            }
        }
    }

    if indent == 0 {
        lines.join("\n")
    } else {
        format!("\n{}", lines.join("\n"))
    }
}

/// Serialize config to YAML string
pub fn serialize_config_to_yaml(config: &ConfigDict) -> String {
    config_dict_to_yaml(config, 0)
}

/// Save a config to a YAML file
pub fn save_config_file(
    config: &ConfigDict,
    filename: &str,
    output_dir: &Path,
) -> std::io::Result<PathBuf> {
    let file_path = output_dir.join(filename);
    let yaml = serialize_config_to_yaml(config);
    fs::write(&file_path, yaml)?;
    Ok(file_path)
}

/// Save overrides list to a YAML file
pub fn save_overrides_file(
    overrides: &[String],
    filename: &str,
    output_dir: &Path,
) -> std::io::Result<PathBuf> {
    let file_path = output_dir.join(filename);
    let yaml = if overrides.is_empty() {
        "[]".to_string()
    } else {
        overrides
            .iter()
            .map(|o| format!("- {}", o))
            .collect::<Vec<_>>()
            .join("\n")
    };
    fs::write(&file_path, yaml)?;
    Ok(file_path)
}

/// Setup job execution environment
pub fn setup_job_environment(
    output_dir: &Path,
    hydra_subdir: Option<&str>,
    task_config: &ConfigDict,
    hydra_config: &ConfigDict,
    overrides: &[String],
) -> std::io::Result<PathBuf> {
    // Create output directory
    fs::create_dir_all(output_dir)?;

    // Create hydra output subdirectory if specified
    if let Some(subdir) = hydra_subdir {
        let hydra_output = output_dir.join(subdir);
        fs::create_dir_all(&hydra_output)?;

        // Save config files
        save_config_file(task_config, "config.yaml", &hydra_output)?;
        save_config_file(hydra_config, "hydra.yaml", &hydra_output)?;
        save_overrides_file(overrides, "overrides.yaml", &hydra_output)?;

        Ok(hydra_output)
    } else {
        Ok(output_dir.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_context() {
        let ctx = JobContext::new("myapp", "abc123", 0)
            .with_output_dir("/output/test")
            .with_chdir(true)
            .with_overrides(vec!["db=mysql".to_string()]);

        assert_eq!(ctx.name, "myapp");
        assert_eq!(ctx.id, "abc123");
        assert_eq!(ctx.num, 0);
        assert!(ctx.chdir);
        // Normalize path separators for cross-platform comparison
        let output_dir_str = ctx.output_dir.to_string_lossy().replace('\\', "/");
        assert_eq!(output_dir_str, "/output/test");
        assert_eq!(ctx.overrides.len(), 1);
    }

    #[test]
    fn test_compute_output_dir() {
        let dir = compute_output_dir("/output/run", None);
        // Normalize path separators for cross-platform comparison
        let dir_str = dir.to_string_lossy().replace('\\', "/");
        assert_eq!(dir_str, "/output/run");

        let dir = compute_output_dir("/output/sweep", Some("0"));
        let dir_str = dir.to_string_lossy().replace('\\', "/");
        assert_eq!(dir_str, "/output/sweep/0");
    }

    #[test]
    fn test_serialize_config() {
        let mut config = ConfigDict::new();
        config.insert("name".to_string(), ConfigValue::String("test".to_string()));
        config.insert("port".to_string(), ConfigValue::Int(8080));
        config.insert("debug".to_string(), ConfigValue::Bool(true));

        let yaml = serialize_config_to_yaml(&config);
        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("port: 8080"));
        assert!(yaml.contains("debug: true"));
    }

    #[test]
    fn test_job_status() {
        assert_eq!(JobStatus::Unknown as i32, 0);
        assert_eq!(JobStatus::Completed as i32, 1);
        assert_eq!(JobStatus::Failed as i32, 2);
    }
}
