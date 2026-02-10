//! SystemdService unit generation and install/uninstall methods.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::error::DaemonError;
use super::{SystemdConfig, SystemdStatus};

/// Linux Systemd service manager.
#[derive(Debug)]
pub struct SystemdService {
    pub(super) config: SystemdConfig,
}

impl SystemdService {
    /// Create a new SystemdService manager.
    pub fn new(config: SystemdConfig) -> Self {
        Self { config }
    }

    /// Get the service unit file path.
    pub fn unit_path(&self) -> PathBuf {
        if self.config.user_mode {
            dirs::config_dir()
                .map(|c| c.join("systemd").join("user").join(format!("{}.service", self.config.service_name)))
                .unwrap_or_else(|| {
                    dirs::home_dir()
                        .map(|h| h.join(".config").join("systemd").join("user").join(format!("{}.service", self.config.service_name)))
                        .unwrap_or_else(|| PathBuf::from(format!("/tmp/{}.service", self.config.service_name)))
                })
        } else {
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
    pub fn install(&self) -> Result<(), DaemonError> {
        let unit_path = self.unit_path();

        if let Some(parent) = unit_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                DaemonError::Custom(format!("Failed to create systemd directory: {}", e))
            })?;
        }

        let content = self.generate_unit();
        let mut file = fs::File::create(&unit_path).map_err(|e| {
            DaemonError::Custom(format!("Failed to create unit file: {}", e))
        })?;
        file.write_all(content.as_bytes()).map_err(|e| {
            DaemonError::Custom(format!("Failed to write unit file: {}", e))
        })?;

        tracing::info!("Created Systemd unit file at: {}", unit_path.display());

        self.daemon_reload()?;
        self.enable()?;

        Ok(())
    }

    /// Uninstall the Systemd service.
    pub fn uninstall(&self) -> Result<(), DaemonError> {
        let _ = self.stop();
        let _ = self.disable();

        let unit_path = self.unit_path();
        if unit_path.exists() {
            fs::remove_file(&unit_path).map_err(|e| {
                DaemonError::Custom(format!("Failed to remove unit file: {}", e))
            })?;
            tracing::info!("Removed Systemd unit file: {}", unit_path.display());
        }

        self.daemon_reload()?;

        Ok(())
    }

    /// Check if the service is installed.
    pub fn is_installed(&self) -> bool {
        self.unit_path().exists()
    }
}
