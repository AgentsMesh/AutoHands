//! Linux Systemd service management.
//!
//! This module provides functionality to generate and manage Linux Systemd
//! service unit files for running AutoHands as a system service.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use autohands_daemon::systemd::{SystemdService, SystemdConfig};
//!
//! let config = SystemdConfig::default();
//! let service = SystemdService::new(config);
//! service.install()?;
//! ```

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::DaemonError;

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

    /// Standard output destination (inherit, null, tty, journal, kmsg, journal+console, kmsg+console, file:path).
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
            user_mode: true, // Default to user mode for non-root installation
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

/// Linux Systemd service manager.
#[derive(Debug)]
pub struct SystemdService {
    config: SystemdConfig,
}

impl SystemdService {
    /// Create a new SystemdService manager.
    pub fn new(config: SystemdConfig) -> Self {
        Self { config }
    }

    /// Get the service unit file path.
    pub fn unit_path(&self) -> PathBuf {
        if self.config.user_mode {
            // User mode: ~/.config/systemd/user/
            dirs::config_dir()
                .map(|c| c.join("systemd").join("user").join(format!("{}.service", self.config.service_name)))
                .unwrap_or_else(|| {
                    dirs::home_dir()
                        .map(|h| h.join(".config").join("systemd").join("user").join(format!("{}.service", self.config.service_name)))
                        .unwrap_or_else(|| PathBuf::from(format!("/tmp/{}.service", self.config.service_name)))
                })
        } else {
            // System mode: /etc/systemd/system/
            PathBuf::from(format!("/etc/systemd/system/{}.service", self.config.service_name))
        }
    }

    /// Generate the systemd unit file content.
    pub fn generate_unit(&self) -> String {
        let mut unit = String::new();

        // [Unit] section
        unit.push_str("[Unit]\n");
        unit.push_str(&format!("Description={}\n", self.config.description));

        if !self.config.after.is_empty() {
            unit.push_str(&format!("After={}\n", self.config.after.join(" ")));
        }

        if !self.config.wants.is_empty() {
            unit.push_str(&format!("Wants={}\n", self.config.wants.join(" ")));
        }

        unit.push_str(&format!("StartLimitIntervalSec={}\n", self.config.start_limit_interval_sec));
        unit.push_str(&format!("StartLimitBurst={}\n", self.config.start_limit_burst));
        unit.push('\n');

        // [Service] section
        unit.push_str("[Service]\n");
        unit.push_str(&format!("Type={}\n", self.config.service_type));

        // Build ExecStart command
        let exec_start = if self.config.exec_args.is_empty() {
            self.config.exec_start.display().to_string()
        } else {
            format!(
                "{} {}",
                self.config.exec_start.display(),
                self.config.exec_args.join(" ")
            )
        };
        unit.push_str(&format!("ExecStart={}\n", exec_start));

        if let Some(ref dir) = self.config.working_directory {
            unit.push_str(&format!("WorkingDirectory={}\n", dir.display()));
        }

        if let Some(ref user) = self.config.user {
            unit.push_str(&format!("User={}\n", user));
        }

        if let Some(ref group) = self.config.group {
            unit.push_str(&format!("Group={}\n", group));
        }

        unit.push_str(&format!("Restart={}\n", self.config.restart));
        unit.push_str(&format!("RestartSec={}\n", self.config.restart_sec));

        // Environment variables
        for (key, value) in &self.config.environment {
            unit.push_str(&format!("Environment=\"{}={}\"\n", key, value));
        }

        if let Some(ref env_file) = self.config.environment_file {
            unit.push_str(&format!("EnvironmentFile={}\n", env_file.display()));
        }

        if let Some(watchdog_sec) = self.config.watchdog_sec {
            unit.push_str(&format!("WatchdogSec={}\n", watchdog_sec));
        }

        unit.push_str(&format!("StandardOutput={}\n", self.config.standard_output));
        unit.push_str(&format!("StandardError={}\n", self.config.standard_error));
        unit.push_str(&format!("SyslogIdentifier={}\n", self.config.syslog_identifier));

        // Security hardening (only for system services)
        if !self.config.user_mode {
            unit.push_str("NoNewPrivileges=true\n");
            unit.push_str("PrivateTmp=true\n");
            unit.push_str("ProtectSystem=strict\n");
            unit.push_str("ProtectHome=read-only\n");
        }

        unit.push('\n');

        // [Install] section
        unit.push_str("[Install]\n");
        if !self.config.wanted_by.is_empty() {
            unit.push_str(&format!("WantedBy={}\n", self.config.wanted_by.join(" ")));
        }

        unit
    }

    /// Install the Systemd service.
    ///
    /// This will:
    /// 1. Create the unit file
    /// 2. Reload systemd daemon
    /// 3. Enable the service
    pub fn install(&self) -> Result<(), DaemonError> {
        let unit_path = self.unit_path();

        // Ensure parent directory exists
        if let Some(parent) = unit_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                DaemonError::Custom(format!("Failed to create systemd directory: {}", e))
            })?;
        }

        // Write unit file
        let content = self.generate_unit();
        let mut file = fs::File::create(&unit_path).map_err(|e| {
            DaemonError::Custom(format!("Failed to create unit file: {}", e))
        })?;
        file.write_all(content.as_bytes()).map_err(|e| {
            DaemonError::Custom(format!("Failed to write unit file: {}", e))
        })?;

        tracing::info!("Created Systemd unit file at: {}", unit_path.display());

        // Reload systemd daemon
        self.daemon_reload()?;

        // Enable the service
        self.enable()?;

        Ok(())
    }

    /// Uninstall the Systemd service.
    ///
    /// This will:
    /// 1. Stop the service
    /// 2. Disable the service
    /// 3. Remove the unit file
    /// 4. Reload systemd daemon
    pub fn uninstall(&self) -> Result<(), DaemonError> {
        // Stop the service (ignore errors if not running)
        let _ = self.stop();

        // Disable the service (ignore errors if not enabled)
        let _ = self.disable();

        // Remove unit file
        let unit_path = self.unit_path();
        if unit_path.exists() {
            fs::remove_file(&unit_path).map_err(|e| {
                DaemonError::Custom(format!("Failed to remove unit file: {}", e))
            })?;
            tracing::info!("Removed Systemd unit file: {}", unit_path.display());
        }

        // Reload systemd daemon
        self.daemon_reload()?;

        Ok(())
    }

    /// Get systemctl command arguments for user/system mode.
    fn systemctl_args(&self) -> Vec<&str> {
        if self.config.user_mode {
            vec!["--user"]
        } else {
            vec![]
        }
    }

    /// Reload systemd daemon configuration.
    pub fn daemon_reload(&self) -> Result<(), DaemonError> {
        let mut args = self.systemctl_args();
        args.push("daemon-reload");

        let output = Command::new("systemctl")
            .args(&args)
            .output()
            .map_err(|e| DaemonError::Custom(format!("Failed to execute systemctl: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DaemonError::Custom(format!(
                "Failed to reload systemd daemon: {}",
                stderr
            )));
        }

        tracing::debug!("Reloaded systemd daemon");
        Ok(())
    }

    /// Enable the service.
    pub fn enable(&self) -> Result<(), DaemonError> {
        let mut args = self.systemctl_args();
        args.push("enable");
        args.push(&self.config.service_name);

        let output = Command::new("systemctl")
            .args(&args)
            .output()
            .map_err(|e| DaemonError::Custom(format!("Failed to execute systemctl: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DaemonError::Custom(format!(
                "Failed to enable service: {}",
                stderr
            )));
        }

        tracing::info!("Enabled service: {}", self.config.service_name);
        Ok(())
    }

    /// Disable the service.
    pub fn disable(&self) -> Result<(), DaemonError> {
        let mut args = self.systemctl_args();
        args.push("disable");
        args.push(&self.config.service_name);

        let output = Command::new("systemctl")
            .args(&args)
            .output()
            .map_err(|e| DaemonError::Custom(format!("Failed to execute systemctl: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If not enabled, that's fine
            if !stderr.contains("not enabled") {
                return Err(DaemonError::Custom(format!(
                    "Failed to disable service: {}",
                    stderr
                )));
            }
        }

        tracing::info!("Disabled service: {}", self.config.service_name);
        Ok(())
    }

    /// Start the service.
    pub fn start(&self) -> Result<(), DaemonError> {
        let mut args = self.systemctl_args();
        args.push("start");
        args.push(&self.config.service_name);

        let output = Command::new("systemctl")
            .args(&args)
            .output()
            .map_err(|e| DaemonError::Custom(format!("Failed to execute systemctl: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DaemonError::Custom(format!(
                "Failed to start service: {}",
                stderr
            )));
        }

        tracing::info!("Started service: {}", self.config.service_name);
        Ok(())
    }

    /// Stop the service.
    pub fn stop(&self) -> Result<(), DaemonError> {
        let mut args = self.systemctl_args();
        args.push("stop");
        args.push(&self.config.service_name);

        let output = Command::new("systemctl")
            .args(&args)
            .output()
            .map_err(|e| DaemonError::Custom(format!("Failed to execute systemctl: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If not running, that's fine
            if !stderr.contains("not loaded") {
                return Err(DaemonError::Custom(format!(
                    "Failed to stop service: {}",
                    stderr
                )));
            }
        }

        tracing::info!("Stopped service: {}", self.config.service_name);
        Ok(())
    }

    /// Restart the service.
    pub fn restart(&self) -> Result<(), DaemonError> {
        let mut args = self.systemctl_args();
        args.push("restart");
        args.push(&self.config.service_name);

        let output = Command::new("systemctl")
            .args(&args)
            .output()
            .map_err(|e| DaemonError::Custom(format!("Failed to execute systemctl: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DaemonError::Custom(format!(
                "Failed to restart service: {}",
                stderr
            )));
        }

        tracing::info!("Restarted service: {}", self.config.service_name);
        Ok(())
    }

    /// Get the status of the service.
    pub fn status(&self) -> Result<SystemdStatus, DaemonError> {
        let mut args = self.systemctl_args();
        args.push("show");
        args.push(&self.config.service_name);
        args.push("--property=ActiveState,SubState,MainPID,LoadState");

        let output = Command::new("systemctl")
            .args(&args)
            .output()
            .map_err(|e| DaemonError::Custom(format!("Failed to execute systemctl: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut status = SystemdStatus::default();

        for line in stdout.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "ActiveState" => status.active_state = value.to_string(),
                    "SubState" => status.sub_state = value.to_string(),
                    "MainPID" => status.main_pid = value.parse().ok(),
                    "LoadState" => status.load_state = value.to_string(),
                    _ => {}
                }
            }
        }

        status.running = status.active_state == "active";
        status.enabled = self.is_enabled();

        Ok(status)
    }

    /// Check if the service is enabled.
    pub fn is_enabled(&self) -> bool {
        let mut args = self.systemctl_args();
        args.push("is-enabled");
        args.push(&self.config.service_name);

        Command::new("systemctl")
            .args(&args)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if the service is installed.
    pub fn is_installed(&self) -> bool {
        self.unit_path().exists()
    }

    /// Get logs from the service.
    pub fn logs(&self, lines: u32) -> Result<String, DaemonError> {
        let mut args = vec![];
        if self.config.user_mode {
            args.push("--user");
        }
        args.push("-u");
        args.push(&self.config.service_name);
        args.push("-n");
        let lines_str = lines.to_string();
        args.push(&lines_str);
        args.push("--no-pager");

        let output = Command::new("journalctl")
            .args(&args)
            .output()
            .map_err(|e| DaemonError::Custom(format!("Failed to execute journalctl: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// Systemd service status information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemdStatus {
    /// Whether the service is currently running.
    pub running: bool,
    /// Whether the service is enabled (starts on boot).
    pub enabled: bool,
    /// Active state (active, inactive, activating, deactivating, failed).
    pub active_state: String,
    /// Sub state (running, exited, dead, etc.).
    pub sub_state: String,
    /// Load state (loaded, not-found, error, masked).
    pub load_state: String,
    /// Main process ID if running.
    pub main_pid: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SystemdConfig::default();
        assert_eq!(config.service_name, "autohands");
        assert_eq!(config.restart, "on-failure");
        assert!(config.user_mode);
    }

    #[test]
    fn test_config_builder() {
        let config = SystemdConfig::with_name("myservice")
            .exec_start("/usr/bin/myservice")
            .working_directory("/var/lib/myservice")
            .user("myuser")
            .env("FOO", "bar")
            .system_mode();

        assert_eq!(config.service_name, "myservice");
        assert_eq!(config.exec_start, PathBuf::from("/usr/bin/myservice"));
        assert_eq!(config.working_directory, Some(PathBuf::from("/var/lib/myservice")));
        assert_eq!(config.user, Some("myuser".to_string()));
        assert_eq!(config.environment.get("FOO"), Some(&"bar".to_string()));
        assert!(!config.user_mode);
    }

    #[test]
    fn test_generate_unit() {
        let config = SystemdConfig::with_name("testservice")
            .exec_start("/usr/bin/test")
            .exec_args(vec!["--daemon".to_string()])
            .working_directory("/tmp")
            .env("TEST_VAR", "test_value");

        let service = SystemdService::new(config);
        let unit = service.generate_unit();

        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("[Service]"));
        assert!(unit.contains("[Install]"));
        assert!(unit.contains("Description="));
        assert!(unit.contains("ExecStart=/usr/bin/test --daemon"));
        assert!(unit.contains("WorkingDirectory=/tmp"));
        assert!(unit.contains("Environment=\"TEST_VAR=test_value\""));
        assert!(unit.contains("Restart=on-failure"));
    }

    #[test]
    fn test_unit_path_user_mode() {
        let config = SystemdConfig::with_name("testservice").user_mode();
        let service = SystemdService::new(config);
        let path = service.unit_path();

        assert!(path.to_string_lossy().contains("systemd/user"));
        assert!(path.to_string_lossy().contains("testservice.service"));
    }

    #[test]
    fn test_unit_path_system_mode() {
        let config = SystemdConfig::with_name("testservice").system_mode();
        let service = SystemdService::new(config);
        let path = service.unit_path();

        assert_eq!(path, PathBuf::from("/etc/systemd/system/testservice.service"));
    }
}
