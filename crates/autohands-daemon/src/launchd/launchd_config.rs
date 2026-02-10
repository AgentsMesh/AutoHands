//! LaunchAgent configuration types and builder methods.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// LaunchAgent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchAgentConfig {
    /// Service label (reverse domain notation).
    #[serde(default = "default_label")]
    pub label: String,

    /// Path to the autohands executable.
    #[serde(default = "default_program")]
    pub program: PathBuf,

    /// Program arguments.
    #[serde(default = "default_program_args")]
    pub program_arguments: Vec<String>,

    /// Working directory.
    #[serde(default)]
    pub working_directory: Option<PathBuf>,

    /// Whether to run at load (system startup).
    #[serde(default = "default_run_at_load")]
    pub run_at_load: bool,

    /// Keep the service alive (restart on exit).
    #[serde(default = "default_keep_alive")]
    pub keep_alive: bool,

    /// Standard output log file path.
    #[serde(default = "default_stdout_path")]
    pub standard_out_path: PathBuf,

    /// Standard error log file path.
    #[serde(default = "default_stderr_path")]
    pub standard_error_path: PathBuf,

    /// Environment variables.
    #[serde(default)]
    pub environment_variables: std::collections::HashMap<String, String>,

    /// Throttle interval (seconds) - minimum time between restarts.
    #[serde(default = "default_throttle_interval")]
    pub throttle_interval: u32,

    /// Nice value (process priority, -20 to 20).
    #[serde(default)]
    pub nice: Option<i32>,

    /// Low priority IO.
    #[serde(default)]
    pub low_priority_io: bool,

    /// Process type (Background, Standard, Adaptive, Interactive).
    #[serde(default = "default_process_type")]
    pub process_type: String,
}

fn default_label() -> String {
    "com.autohands.agent".to_string()
}

fn default_program() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        return exe;
    }
    PathBuf::from("/usr/local/bin/autohands")
}

fn default_program_args() -> Vec<String> {
    vec!["daemon".to_string(), "start".to_string(), "--foreground".to_string()]
}

fn default_run_at_load() -> bool {
    true
}

fn default_keep_alive() -> bool {
    true
}

fn default_stdout_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".autohands").join("logs").join("stdout.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/autohands-stdout.log"))
}

fn default_stderr_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".autohands").join("logs").join("stderr.log"))
        .unwrap_or_else(|| PathBuf::from("/tmp/autohands-stderr.log"))
}

fn default_throttle_interval() -> u32 {
    10
}

fn default_process_type() -> String {
    "Background".to_string()
}

impl Default for LaunchAgentConfig {
    fn default() -> Self {
        Self {
            label: default_label(),
            program: default_program(),
            program_arguments: default_program_args(),
            working_directory: dirs::home_dir(),
            run_at_load: default_run_at_load(),
            keep_alive: default_keep_alive(),
            standard_out_path: default_stdout_path(),
            standard_error_path: default_stderr_path(),
            environment_variables: std::collections::HashMap::new(),
            throttle_interval: default_throttle_interval(),
            nice: None,
            low_priority_io: false,
            process_type: default_process_type(),
        }
    }
}

impl LaunchAgentConfig {
    /// Create a new config with custom label.
    pub fn with_label(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Default::default()
        }
    }

    /// Set the program path.
    pub fn program(mut self, path: impl Into<PathBuf>) -> Self {
        self.program = path.into();
        self
    }

    /// Set program arguments.
    pub fn program_arguments(mut self, args: Vec<String>) -> Self {
        self.program_arguments = args;
        self
    }

    /// Set working directory.
    pub fn working_directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    /// Add an environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment_variables.insert(key.into(), value.into());
        self
    }
}
