//! Systemd service configuration types and builder methods.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Systemd service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemdConfig {
    /// Service name (without .service extension).
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// Service description.
    #[serde(default = "default_description")]
    pub description: String,

    /// Path to the autohands executable.
    #[serde(default = "default_exec_start")]
    pub exec_start: PathBuf,

    /// Arguments to pass to the executable.
    #[serde(default = "default_exec_args")]
    pub exec_args: Vec<String>,

    /// Working directory.
    #[serde(default)]
    pub working_directory: Option<PathBuf>,

    /// User to run the service as.
    #[serde(default)]
    pub user: Option<String>,

    /// Group to run the service as.
    #[serde(default)]
    pub group: Option<String>,

    /// Restart policy (no, on-success, on-failure, on-abnormal, on-watchdog, on-abort, always).
    #[serde(default = "default_restart")]
    pub restart: String,

    /// Time to wait before restarting (seconds).
    #[serde(default = "default_restart_sec")]
    pub restart_sec: u32,

    /// Service type (simple, exec, forking, oneshot, dbus, notify, idle).
    #[serde(default = "default_service_type")]
    pub service_type: String,

    /// Environment variables.
    #[serde(default)]
    pub environment: std::collections::HashMap<String, String>,

    /// Environment file path.
    #[serde(default)]
    pub environment_file: Option<PathBuf>,

    /// Whether to enable watchdog.
    #[serde(default)]
    pub watchdog_sec: Option<u32>,

    /// Standard output destination.
    #[serde(default = "default_standard_output")]
    pub standard_output: String,

    /// Standard error destination.
    #[serde(default = "default_standard_error")]
    pub standard_error: String,

    /// Syslog identifier.
    #[serde(default = "default_syslog_identifier")]
    pub syslog_identifier: String,

    /// Start limit interval (seconds).
    #[serde(default = "default_start_limit_interval")]
    pub start_limit_interval_sec: u32,

    /// Start limit burst (max starts within interval).
    #[serde(default = "default_start_limit_burst")]
    pub start_limit_burst: u32,

    /// After dependencies (services to start after).
    #[serde(default = "default_after")]
    pub after: Vec<String>,

    /// Wants dependencies (services to want).
    #[serde(default)]
    pub wants: Vec<String>,

    /// WantedBy targets (for enable/install).
    #[serde(default = "default_wanted_by")]
    pub wanted_by: Vec<String>,

    /// Whether to install as system service (requires root) or user service.
    #[serde(default)]
    pub user_mode: bool,
}

fn default_service_name() -> String {
    "autohands".to_string()
}

fn default_description() -> String {
    "AutoHands Autonomous Agent Framework".to_string()
}

fn default_exec_start() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        return exe;
    }
    PathBuf::from("/usr/local/bin/autohands")
}

fn default_exec_args() -> Vec<String> {
    vec!["daemon".to_string(), "start".to_string(), "--foreground".to_string()]
}

fn default_restart() -> String {
    "on-failure".to_string()
}

fn default_restart_sec() -> u32 {
    5
}

fn default_service_type() -> String {
    "simple".to_string()
}

fn default_standard_output() -> String {
    "journal".to_string()
}

fn default_standard_error() -> String {
    "journal".to_string()
}

fn default_syslog_identifier() -> String {
    "autohands".to_string()
}

fn default_start_limit_interval() -> u32 {
    300
}

fn default_start_limit_burst() -> u32 {
    10
}

fn default_after() -> Vec<String> {
    vec!["network.target".to_string()]
}

fn default_wanted_by() -> Vec<String> {
    vec!["default.target".to_string()]
}

impl Default for SystemdConfig {
    fn default() -> Self {
        Self {
            service_name: default_service_name(),
            description: default_description(),
            exec_start: default_exec_start(),
            exec_args: default_exec_args(),
            working_directory: dirs::home_dir(),
            user: None,
            group: None,
            restart: default_restart(),
            restart_sec: default_restart_sec(),
            service_type: default_service_type(),
            environment: std::collections::HashMap::new(),
            environment_file: None,
            watchdog_sec: None,
            standard_output: default_standard_output(),
            standard_error: default_standard_error(),
            syslog_identifier: default_syslog_identifier(),
            start_limit_interval_sec: default_start_limit_interval(),
            start_limit_burst: default_start_limit_burst(),
            after: default_after(),
            wants: vec![],
            wanted_by: default_wanted_by(),
            user_mode: true,
        }
    }
}

impl SystemdConfig {
    /// Create a new config with custom service name.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            service_name: name.into(),
            ..Default::default()
        }
    }

    /// Set the executable path.
    pub fn exec_start(mut self, path: impl Into<PathBuf>) -> Self {
        self.exec_start = path.into();
        self
    }

    /// Set executable arguments.
    pub fn exec_args(mut self, args: Vec<String>) -> Self {
        self.exec_args = args;
        self
    }

    /// Set working directory.
    pub fn working_directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    /// Set user to run as.
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Add an environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    /// Set to system mode (requires root).
    pub fn system_mode(mut self) -> Self {
        self.user_mode = false;
        self
    }

    /// Set to user mode.
    pub fn user_mode(mut self) -> Self {
        self.user_mode = true;
        self
    }
}
