//! Daemon configuration.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Daemon configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Whether daemon mode is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Path to PID file.
    #[serde(default = "default_pid_file")]
    pub pid_file: PathBuf,

    /// Whether to automatically restart on crash.
    #[serde(default = "default_auto_restart")]
    pub auto_restart: bool,

    /// Maximum number of restarts before giving up.
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,

    /// Time window for counting restarts (in seconds).
    #[serde(default = "default_restart_window")]
    pub restart_window_secs: u64,

    /// Delay between restarts (in seconds).
    #[serde(default = "default_restart_delay")]
    pub restart_delay_secs: u64,

    /// Health check interval (in seconds).
    #[serde(default = "default_health_interval")]
    pub health_check_interval_secs: u64,

    /// Graceful shutdown timeout (in seconds).
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,

    /// Working directory for the daemon.
    #[serde(default)]
    pub work_dir: Option<PathBuf>,

    /// Log file path (when running as daemon).
    #[serde(default)]
    pub log_file: Option<PathBuf>,

    /// Whether to daemonize (fork to background).
    /// Set to false for debugging or running in containers.
    #[serde(default = "default_daemonize")]
    pub daemonize: bool,
}

fn default_enabled() -> bool {
    true
}

fn default_pid_file() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".autohands").join("autohands.pid"))
        .unwrap_or_else(|| PathBuf::from("/tmp/autohands.pid"))
}

fn default_auto_restart() -> bool {
    true
}

fn default_max_restarts() -> u32 {
    10
}

fn default_restart_window() -> u64 {
    300 // 5 minutes
}

fn default_restart_delay() -> u64 {
    5
}

fn default_health_interval() -> u64 {
    30
}

fn default_shutdown_timeout() -> u64 {
    30
}

fn default_daemonize() -> bool {
    true
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            pid_file: default_pid_file(),
            auto_restart: default_auto_restart(),
            max_restarts: default_max_restarts(),
            restart_window_secs: default_restart_window(),
            restart_delay_secs: default_restart_delay(),
            health_check_interval_secs: default_health_interval(),
            shutdown_timeout_secs: default_shutdown_timeout(),
            work_dir: None,
            log_file: None,
            daemonize: default_daemonize(),
        }
    }
}

impl DaemonConfig {
    /// Create a new daemon config with the given PID file path.
    pub fn with_pid_file(pid_file: PathBuf) -> Self {
        Self {
            pid_file,
            ..Default::default()
        }
    }

    /// Get the restart window as a Duration.
    pub fn restart_window(&self) -> Duration {
        Duration::from_secs(self.restart_window_secs)
    }

    /// Get the restart delay as a Duration.
    pub fn restart_delay(&self) -> Duration {
        Duration::from_secs(self.restart_delay_secs)
    }

    /// Get the health check interval as a Duration.
    pub fn health_check_interval(&self) -> Duration {
        Duration::from_secs(self.health_check_interval_secs)
    }

    /// Get the shutdown timeout as a Duration.
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.shutdown_timeout_secs)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_restarts == 0 && self.auto_restart {
            return Err("max_restarts must be > 0 when auto_restart is enabled".to_string());
        }

        if self.restart_window_secs == 0 {
            return Err("restart_window_secs must be > 0".to_string());
        }

        if self.health_check_interval_secs == 0 {
            return Err("health_check_interval_secs must be > 0".to_string());
        }

        if self.shutdown_timeout_secs == 0 {
            return Err("shutdown_timeout_secs must be > 0".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DaemonConfig::default();
        assert!(config.enabled);
        assert!(config.auto_restart);
        assert_eq!(config.max_restarts, 10);
        assert!(config.daemonize);
    }

    #[test]
    fn test_config_with_pid_file() {
        let config = DaemonConfig::with_pid_file(PathBuf::from("/tmp/test.pid"));
        assert_eq!(config.pid_file, PathBuf::from("/tmp/test.pid"));
    }

    #[test]
    fn test_duration_getters() {
        let config = DaemonConfig::default();
        assert_eq!(config.restart_window(), Duration::from_secs(300));
        assert_eq!(config.restart_delay(), Duration::from_secs(5));
        assert_eq!(config.health_check_interval(), Duration::from_secs(30));
        assert_eq!(config.shutdown_timeout(), Duration::from_secs(30));
    }

    #[test]
    fn test_validate_valid_config() {
        let config = DaemonConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_max_restarts() {
        let mut config = DaemonConfig::default();
        config.max_restarts = 0;
        config.auto_restart = true;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_restart_window() {
        let mut config = DaemonConfig::default();
        config.restart_window_secs = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_serialization() {
        let config = DaemonConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("enabled"));
        assert!(json.contains("auto_restart"));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{"enabled": false, "max_restarts": 5}"#;
        let config: DaemonConfig = serde_json::from_str(json).unwrap();
        assert!(!config.enabled);
        assert_eq!(config.max_restarts, 5);
    }
}
