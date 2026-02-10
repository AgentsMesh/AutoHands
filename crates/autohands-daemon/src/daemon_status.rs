//! Daemon status information.

pub use crate::error::DaemonState;

/// Daemon status information.
#[derive(Debug, Clone)]
pub struct DaemonStatus {
    /// Current daemon state.
    pub state: DaemonState,
    /// PID if running.
    pub pid: Option<u32>,
    /// Total health checks performed.
    pub health_checks: u64,
    /// Failed health checks.
    pub health_failures: u64,
}

impl std::fmt::Display for DaemonStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "State: {}", self.state)?;
        if let Some(pid) = self.pid {
            write!(f, ", PID: {}", pid)?;
        }
        write!(
            f,
            ", Health: {}/{}",
            self.health_checks - self.health_failures,
            self.health_checks
        )
    }
}
