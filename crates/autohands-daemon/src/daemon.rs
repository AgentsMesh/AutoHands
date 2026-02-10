//! Core daemon process management.

#[cfg(test)]
#[path = "daemon_tests.rs"]
mod tests;

use std::collections::VecDeque;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{broadcast, RwLock};

use crate::config::DaemonConfig;
use crate::error::{DaemonError, DaemonState as ErrorDaemonState};
use crate::health::HealthChecker;
use crate::pid::PidFile;
use crate::signal::SignalHandler;

pub use crate::error::DaemonState;

/// Daemon state as an atomic value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DaemonStateValue {
    Stopped = 0,
    Starting = 1,
    Running = 2,
    ShuttingDown = 3,
    Restarting = 4,
}

impl From<u8> for DaemonStateValue {
    fn from(v: u8) -> Self {
        match v {
            0 => DaemonStateValue::Stopped,
            1 => DaemonStateValue::Starting,
            2 => DaemonStateValue::Running,
            3 => DaemonStateValue::ShuttingDown,
            4 => DaemonStateValue::Restarting,
            _ => DaemonStateValue::Stopped,
        }
    }
}

impl From<DaemonStateValue> for ErrorDaemonState {
    fn from(v: DaemonStateValue) -> Self {
        match v {
            DaemonStateValue::Stopped => ErrorDaemonState::Stopped,
            DaemonStateValue::Starting => ErrorDaemonState::Starting,
            DaemonStateValue::Running => ErrorDaemonState::Running,
            DaemonStateValue::ShuttingDown => ErrorDaemonState::ShuttingDown,
            DaemonStateValue::Restarting => ErrorDaemonState::Restarting,
        }
    }
}

/// Restart tracking.
pub(crate) struct RestartTracker {
    /// Timestamps of recent restarts.
    restarts: VecDeque<Instant>,
    /// Configuration for restart limits.
    max_restarts: u32,
    /// Time window for counting restarts.
    window: std::time::Duration,
}

impl RestartTracker {
    pub(crate) fn new(config: &DaemonConfig) -> Self {
        Self {
            restarts: VecDeque::new(),
            max_restarts: config.max_restarts,
            window: config.restart_window(),
        }
    }

    /// Record a restart and check if limit exceeded.
    pub(crate) fn record_restart(&mut self) -> bool {
        let now = Instant::now();

        // Remove old restarts outside the window
        while let Some(front) = self.restarts.front() {
            if now.duration_since(*front) > self.window {
                self.restarts.pop_front();
            } else {
                break;
            }
        }

        // Record this restart
        self.restarts.push_back(now);

        // Check if we exceeded the limit
        self.restarts.len() as u32 > self.max_restarts
    }

    /// Get the number of recent restarts.
    pub(crate) fn count(&self) -> u32 {
        self.restarts.len() as u32
    }
}

/// The daemon process manager.
pub struct Daemon {
    pub(crate) config: DaemonConfig,
    pub(crate) state: AtomicU8,
    pub(crate) pid_file: RwLock<PidFile>,
    pub(crate) signal_handler: SignalHandler,
    pub(crate) health_checker: Arc<HealthChecker>,
    pub(crate) restart_tracker: RwLock<RestartTracker>,
    pub(crate) shutdown_sender: broadcast::Sender<()>,
}

impl Daemon {
    /// Create a new daemon instance.
    pub fn new(config: DaemonConfig) -> Result<Self, DaemonError> {
        config.validate().map_err(DaemonError::Config)?;

        let (shutdown_sender, _) = broadcast::channel(1);
        let pid_file = PidFile::new(&config.pid_file);
        let health_checker = Arc::new(HealthChecker::new(config.clone()));
        let restart_tracker = RestartTracker::new(&config);

        Ok(Self {
            config,
            state: AtomicU8::new(DaemonStateValue::Stopped as u8),
            pid_file: RwLock::new(pid_file),
            signal_handler: SignalHandler::new(),
            health_checker,
            restart_tracker: RwLock::new(restart_tracker),
            shutdown_sender,
        })
    }
}
