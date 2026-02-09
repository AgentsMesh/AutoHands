//! Daemon-related errors.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during daemon operations.
#[derive(Debug, Error)]
pub enum DaemonError {
    /// PID file already exists and daemon is running.
    #[error("Daemon already running (PID file: {path}, PID: {pid})")]
    AlreadyRunning { path: PathBuf, pid: u32 },

    /// PID file exists but process is not running (stale).
    #[error("Stale PID file found: {0}")]
    StalePidFile(PathBuf),

    /// Failed to create PID file.
    #[error("Failed to create PID file at {path}: {reason}")]
    PidFileCreation { path: PathBuf, reason: String },

    /// Failed to read PID file.
    #[error("Failed to read PID file at {path}: {reason}")]
    PidFileRead { path: PathBuf, reason: String },

    /// Failed to remove PID file.
    #[error("Failed to remove PID file at {path}: {reason}")]
    PidFileRemoval { path: PathBuf, reason: String },

    /// Process fork failed.
    #[error("Failed to fork process: {0}")]
    ForkFailed(String),

    /// Failed to set up signal handlers.
    #[error("Failed to set up signal handlers: {0}")]
    SignalSetup(String),

    /// Daemon is not running.
    #[error("Daemon is not running")]
    NotRunning,

    /// Invalid daemon state transition.
    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition { from: DaemonState, to: DaemonState },

    /// Health check failed.
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    /// Maximum restart count exceeded.
    #[error("Maximum restart count ({max}) exceeded")]
    MaxRestartsExceeded { max: u32 },

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Generic daemon error.
    #[error("{0}")]
    Custom(String),
}

/// Daemon state for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonState {
    /// Initial state.
    Stopped,
    /// Starting up.
    Starting,
    /// Running normally.
    Running,
    /// Shutting down.
    ShuttingDown,
    /// Restarting.
    Restarting,
}

impl std::fmt::Display for DaemonState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonState::Stopped => write!(f, "stopped"),
            DaemonState::Starting => write!(f, "starting"),
            DaemonState::Running => write!(f, "running"),
            DaemonState::ShuttingDown => write!(f, "shutting_down"),
            DaemonState::Restarting => write!(f, "restarting"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_already_running_error() {
        let err = DaemonError::AlreadyRunning {
            path: PathBuf::from("/tmp/test.pid"),
            pid: 12345,
        };
        let msg = err.to_string();
        assert!(msg.contains("already running"));
        assert!(msg.contains("12345"));
    }

    #[test]
    fn test_stale_pid_file_error() {
        let err = DaemonError::StalePidFile(PathBuf::from("/tmp/test.pid"));
        assert!(err.to_string().contains("Stale"));
    }

    #[test]
    fn test_max_restarts_exceeded() {
        let err = DaemonError::MaxRestartsExceeded { max: 10 };
        let msg = err.to_string();
        assert!(msg.contains("10"));
        assert!(msg.contains("exceeded"));
    }

    #[test]
    fn test_daemon_state_display() {
        assert_eq!(DaemonState::Stopped.to_string(), "stopped");
        assert_eq!(DaemonState::Starting.to_string(), "starting");
        assert_eq!(DaemonState::Running.to_string(), "running");
        assert_eq!(DaemonState::ShuttingDown.to_string(), "shutting_down");
        assert_eq!(DaemonState::Restarting.to_string(), "restarting");
    }

    #[test]
    fn test_invalid_state_transition() {
        let err = DaemonError::InvalidStateTransition {
            from: DaemonState::Stopped,
            to: DaemonState::ShuttingDown,
        };
        let msg = err.to_string();
        assert!(msg.contains("Stopped"));
        assert!(msg.contains("ShuttingDown"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let daemon_err: DaemonError = io_err.into();
        assert!(daemon_err.to_string().contains("file not found"));
    }
}
