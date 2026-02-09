//! macOS LaunchAgent management.
//!
//! This module provides functionality to generate and manage macOS LaunchAgent
//! plist files for running AutoHands as a system service.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use autohands_daemon::launchd::{LaunchAgent, LaunchAgentConfig};
//!
//! let config = LaunchAgentConfig::default();
//! let agent = LaunchAgent::new(config);
//! agent.install()?;
//! ```

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::DaemonError;

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
    // Try to find the executable in common locations
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

/// macOS LaunchAgent manager.
#[derive(Debug)]
pub struct LaunchAgent {
    config: LaunchAgentConfig,
}

impl LaunchAgent {
    /// Create a new LaunchAgent manager.
    pub fn new(config: LaunchAgentConfig) -> Self {
        Self { config }
    }

    /// Get the plist file path.
    pub fn plist_path(&self) -> PathBuf {
        dirs::home_dir()
            .map(|h| h.join("Library").join("LaunchAgents").join(format!("{}.plist", self.config.label)))
            .unwrap_or_else(|| PathBuf::from(format!("/tmp/{}.plist", self.config.label)))
    }

    /// Generate the plist XML content.
    pub fn generate_plist(&self) -> String {
        let mut plist = String::new();
        plist.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        plist.push_str("<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n");
        plist.push_str("<plist version=\"1.0\">\n");
        plist.push_str("<dict>\n");

        // Label
        plist.push_str("    <key>Label</key>\n");
        plist.push_str(&format!("    <string>{}</string>\n", self.config.label));

        // Program
        plist.push_str("    <key>Program</key>\n");
        plist.push_str(&format!("    <string>{}</string>\n", self.config.program.display()));

        // ProgramArguments
        if !self.config.program_arguments.is_empty() {
            plist.push_str("    <key>ProgramArguments</key>\n");
            plist.push_str("    <array>\n");
            plist.push_str(&format!("        <string>{}</string>\n", self.config.program.display()));
            for arg in &self.config.program_arguments {
                plist.push_str(&format!("        <string>{}</string>\n", escape_xml(arg)));
            }
            plist.push_str("    </array>\n");
        }

        // WorkingDirectory
        if let Some(ref dir) = self.config.working_directory {
            plist.push_str("    <key>WorkingDirectory</key>\n");
            plist.push_str(&format!("    <string>{}</string>\n", dir.display()));
        }

        // RunAtLoad
        plist.push_str("    <key>RunAtLoad</key>\n");
        plist.push_str(&format!("    <{}/>", if self.config.run_at_load { "true" } else { "false" }));
        plist.push('\n');

        // KeepAlive
        plist.push_str("    <key>KeepAlive</key>\n");
        plist.push_str(&format!("    <{}/>", if self.config.keep_alive { "true" } else { "false" }));
        plist.push('\n');

        // StandardOutPath
        plist.push_str("    <key>StandardOutPath</key>\n");
        plist.push_str(&format!("    <string>{}</string>\n", self.config.standard_out_path.display()));

        // StandardErrorPath
        plist.push_str("    <key>StandardErrorPath</key>\n");
        plist.push_str(&format!("    <string>{}</string>\n", self.config.standard_error_path.display()));

        // ThrottleInterval
        plist.push_str("    <key>ThrottleInterval</key>\n");
        plist.push_str(&format!("    <integer>{}</integer>\n", self.config.throttle_interval));

        // ProcessType
        plist.push_str("    <key>ProcessType</key>\n");
        plist.push_str(&format!("    <string>{}</string>\n", self.config.process_type));

        // EnvironmentVariables
        if !self.config.environment_variables.is_empty() {
            plist.push_str("    <key>EnvironmentVariables</key>\n");
            plist.push_str("    <dict>\n");
            for (key, value) in &self.config.environment_variables {
                plist.push_str(&format!("        <key>{}</key>\n", escape_xml(key)));
                plist.push_str(&format!("        <string>{}</string>\n", escape_xml(value)));
            }
            plist.push_str("    </dict>\n");
        }

        // Nice
        if let Some(nice) = self.config.nice {
            plist.push_str("    <key>Nice</key>\n");
            plist.push_str(&format!("    <integer>{}</integer>\n", nice));
        }

        // LowPriorityIO
        if self.config.low_priority_io {
            plist.push_str("    <key>LowPriorityIO</key>\n");
            plist.push_str("    <true/>\n");
        }

        plist.push_str("</dict>\n");
        plist.push_str("</plist>\n");

        plist
    }

    /// Install the LaunchAgent.
    ///
    /// This will:
    /// 1. Create the plist file
    /// 2. Create necessary log directories
    /// 3. Load the agent using launchctl
    pub fn install(&self) -> Result<(), DaemonError> {
        // Ensure log directories exist
        if let Some(parent) = self.config.standard_out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                DaemonError::Custom(format!("Failed to create log directory: {}", e))
            })?;
        }
        if let Some(parent) = self.config.standard_error_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                DaemonError::Custom(format!("Failed to create log directory: {}", e))
            })?;
        }

        // Ensure LaunchAgents directory exists
        let plist_path = self.plist_path();
        if let Some(parent) = plist_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                DaemonError::Custom(format!("Failed to create LaunchAgents directory: {}", e))
            })?;
        }

        // Write plist file
        let content = self.generate_plist();
        let mut file = fs::File::create(&plist_path).map_err(|e| {
            DaemonError::Custom(format!("Failed to create plist file: {}", e))
        })?;
        file.write_all(content.as_bytes()).map_err(|e| {
            DaemonError::Custom(format!("Failed to write plist file: {}", e))
        })?;

        tracing::info!("Created LaunchAgent plist at: {}", plist_path.display());

        // Load the agent
        self.load()?;

        Ok(())
    }

    /// Uninstall the LaunchAgent.
    ///
    /// This will:
    /// 1. Unload the agent using launchctl
    /// 2. Remove the plist file
    pub fn uninstall(&self) -> Result<(), DaemonError> {
        // Unload the agent (ignore errors if not loaded)
        let _ = self.unload();

        // Remove plist file
        let plist_path = self.plist_path();
        if plist_path.exists() {
            fs::remove_file(&plist_path).map_err(|e| {
                DaemonError::Custom(format!("Failed to remove plist file: {}", e))
            })?;
            tracing::info!("Removed LaunchAgent plist: {}", plist_path.display());
        }

        Ok(())
    }

    /// Load the LaunchAgent using launchctl.
    pub fn load(&self) -> Result<(), DaemonError> {
        let plist_path = self.plist_path();

        // Use launchctl bootstrap for modern macOS (10.10+)
        // Fall back to launchctl load for older systems
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
                // If already loaded, that's fine
                if stderr.contains("already loaded") || stderr.contains("service already loaded") {
                    tracing::info!("LaunchAgent already loaded: {}", self.config.label);
                    Ok(())
                } else {
                    // Try legacy load command
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
            // If not loaded, that's fine
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

    /// Check if the LaunchAgent is installed.
    pub fn is_installed(&self) -> bool {
        self.plist_path().exists()
    }
}

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

/// Escape special characters for XML.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LaunchAgentConfig::default();
        assert_eq!(config.label, "com.autohands.agent");
        assert!(config.run_at_load);
        assert!(config.keep_alive);
    }

    #[test]
    fn test_config_builder() {
        let config = LaunchAgentConfig::with_label("com.test.service")
            .program("/usr/bin/test")
            .working_directory("/tmp")
            .env("FOO", "bar");

        assert_eq!(config.label, "com.test.service");
        assert_eq!(config.program, PathBuf::from("/usr/bin/test"));
        assert_eq!(config.working_directory, Some(PathBuf::from("/tmp")));
        assert_eq!(config.environment_variables.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_generate_plist() {
        let config = LaunchAgentConfig::with_label("com.test.agent")
            .program("/usr/local/bin/test")
            .program_arguments(vec!["--daemon".to_string()])
            .env("PATH", "/usr/local/bin");

        let agent = LaunchAgent::new(config);
        let plist = agent.generate_plist();

        assert!(plist.contains("<key>Label</key>"));
        assert!(plist.contains("<string>com.test.agent</string>"));
        assert!(plist.contains("<key>Program</key>"));
        assert!(plist.contains("<string>/usr/local/bin/test</string>"));
        assert!(plist.contains("<key>EnvironmentVariables</key>"));
        assert!(plist.contains("<key>PATH</key>"));
    }

    #[test]
    fn test_plist_path() {
        let config = LaunchAgentConfig::with_label("com.test.agent");
        let agent = LaunchAgent::new(config);
        let path = agent.plist_path();

        assert!(path.to_string_lossy().contains("LaunchAgents"));
        assert!(path.to_string_lossy().contains("com.test.agent.plist"));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("foo & bar"), "foo &amp; bar");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }
}
