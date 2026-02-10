//! SystemdService operational methods (start, stop, restart, status, logs).

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::DaemonError;
use super::SystemdService;

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

impl SystemdService {
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
