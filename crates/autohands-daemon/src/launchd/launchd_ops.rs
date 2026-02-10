//! LaunchAgent operational methods (load, unload, start, stop, status).

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::DaemonError;
use super::LaunchAgent;

/// LaunchAgent status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchAgentStatus {
    /// Whether the agent is loaded.
    pub loaded: bool,
    /// Whether the agent is currently running.
    pub running: bool,
    /// Process ID if running.
    pub pid: Option<u32>,
    /// State description.
    pub state: String,
}

impl LaunchAgent {
    /// Load the LaunchAgent using launchctl.
    pub fn load(&self) -> Result<(), DaemonError> {
        let plist_path = self.plist_path();
        let uid = unsafe { libc::getuid() };
        let domain_target = format!("gui/{}", uid);

        let output = Command::new("launchctl")
            .args(["bootstrap", &domain_target, &plist_path.to_string_lossy()])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                tracing::info!("Loaded LaunchAgent: {}", self.config.label);
                Ok(())
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("already loaded") || stderr.contains("service already loaded") {
                    tracing::info!("LaunchAgent already loaded: {}", self.config.label);
                    Ok(())
                } else {
                    self.load_legacy()
                }
            }
            Err(_) => self.load_legacy(),
        }
    }

    /// Load using legacy launchctl load command.
    fn load_legacy(&self) -> Result<(), DaemonError> {
        let plist_path = self.plist_path();
        let output = Command::new("launchctl")
            .args(["load", "-w", &plist_path.to_string_lossy()])
            .output()
            .map_err(|e| DaemonError::Custom(format!("Failed to execute launchctl: {}", e)))?;

        if output.status.success() {
            tracing::info!("Loaded LaunchAgent (legacy): {}", self.config.label);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already loaded") {
                tracing::info!("LaunchAgent already loaded: {}", self.config.label);
                Ok(())
            } else {
                Err(DaemonError::Custom(format!(
                    "Failed to load LaunchAgent: {}",
                    stderr
                )))
            }
        }
    }

    /// Unload the LaunchAgent using launchctl.
    pub fn unload(&self) -> Result<(), DaemonError> {
        let uid = unsafe { libc::getuid() };
        let service_target = format!("gui/{}/{}", uid, self.config.label);

        let output = Command::new("launchctl")
            .args(["bootout", &service_target])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                tracing::info!("Unloaded LaunchAgent: {}", self.config.label);
                Ok(())
            }
            Ok(_) => self.unload_legacy(),
            Err(_) => self.unload_legacy(),
        }
    }

    /// Unload using legacy launchctl unload command.
    fn unload_legacy(&self) -> Result<(), DaemonError> {
        let plist_path = self.plist_path();
        let output = Command::new("launchctl")
            .args(["unload", "-w", &plist_path.to_string_lossy()])
            .output()
            .map_err(|e| DaemonError::Custom(format!("Failed to execute launchctl: {}", e)))?;

        if output.status.success() {
            tracing::info!("Unloaded LaunchAgent (legacy): {}", self.config.label);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not loaded") || stderr.contains("Could not find") {
                Ok(())
            } else {
                Err(DaemonError::Custom(format!(
                    "Failed to unload LaunchAgent: {}",
                    stderr
                )))
            }
        }
    }

    /// Start the LaunchAgent service.
    pub fn start(&self) -> Result<(), DaemonError> {
        let uid = unsafe { libc::getuid() };
        let service_target = format!("gui/{}/{}", uid, self.config.label);

        let output = Command::new("launchctl")
            .args(["kickstart", "-k", &service_target])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                tracing::info!("Started LaunchAgent: {}", self.config.label);
                Ok(())
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                Err(DaemonError::Custom(format!(
                    "Failed to start LaunchAgent: {}",
                    stderr
                )))
            }
            Err(e) => Err(DaemonError::Custom(format!(
                "Failed to execute launchctl: {}",
                e
            ))),
        }
    }

    /// Stop the LaunchAgent service.
    pub fn stop(&self) -> Result<(), DaemonError> {
        let uid = unsafe { libc::getuid() };
        let service_target = format!("gui/{}/{}", uid, self.config.label);

        let output = Command::new("launchctl")
            .args(["kill", "SIGTERM", &service_target])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                tracing::info!("Stopped LaunchAgent: {}", self.config.label);
                Ok(())
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                Err(DaemonError::Custom(format!(
                    "Failed to stop LaunchAgent: {}",
                    stderr
                )))
            }
            Err(e) => Err(DaemonError::Custom(format!(
                "Failed to execute launchctl: {}",
                e
            ))),
        }
    }

    /// Get the status of the LaunchAgent.
    pub fn status(&self) -> Result<LaunchAgentStatus, DaemonError> {
        let uid = unsafe { libc::getuid() };
        let service_target = format!("gui/{}/{}", uid, self.config.label);

        let output = Command::new("launchctl")
            .args(["print", &service_target])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let pid = parse_pid_from_launchctl_print(&stdout);
                let state = parse_state_from_launchctl_print(&stdout);
                Ok(LaunchAgentStatus {
                    loaded: true,
                    running: pid.is_some(),
                    pid,
                    state,
                })
            }
            Ok(_) => Ok(LaunchAgentStatus {
                loaded: false,
                running: false,
                pid: None,
                state: "not loaded".to_string(),
            }),
            Err(e) => Err(DaemonError::Custom(format!(
                "Failed to get LaunchAgent status: {}",
                e
            ))),
        }
    }
}

/// Parse PID from launchctl print output.
fn parse_pid_from_launchctl_print(output: &str) -> Option<u32> {
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("pid = ") {
            return line
                .strip_prefix("pid = ")
                .and_then(|s| s.parse().ok());
        }
    }
    None
}

/// Parse state from launchctl print output.
fn parse_state_from_launchctl_print(output: &str) -> String {
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("state = ") {
            return line
                .strip_prefix("state = ")
                .unwrap_or("unknown")
                .to_string();
        }
    }
    "unknown".to_string()
}
