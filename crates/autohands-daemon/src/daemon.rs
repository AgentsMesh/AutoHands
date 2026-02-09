//! Core daemon process management.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};

use crate::config::DaemonConfig;
use crate::error::{DaemonError, DaemonState as ErrorDaemonState};
use crate::health::{HealthCheckable, HealthChecker, LivenessCheck};
use crate::pid::PidFile;
use crate::signal::{DaemonSignal, SignalHandler};

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
struct RestartTracker {
    /// Timestamps of recent restarts.
    restarts: VecDeque<Instant>,
    /// Configuration for restart limits.
    max_restarts: u32,
    /// Time window for counting restarts.
    window: std::time::Duration,
}

impl RestartTracker {
    fn new(config: &DaemonConfig) -> Self {
        Self {
            restarts: VecDeque::new(),
            max_restarts: config.max_restarts,
            window: config.restart_window(),
        }
    }

    /// Record a restart and check if limit exceeded.
    fn record_restart(&mut self) -> bool {
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
    fn count(&self) -> u32 {
        self.restarts.len() as u32
    }
}

/// The daemon process manager.
pub struct Daemon {
    config: DaemonConfig,
    state: AtomicU8,
    pid_file: RwLock<PidFile>,
    signal_handler: SignalHandler,
    health_checker: Arc<HealthChecker>,
    restart_tracker: RwLock<RestartTracker>,
    shutdown_sender: broadcast::Sender<()>,
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

    /// Get the current daemon state.
    pub fn state(&self) -> DaemonState {
        DaemonStateValue::from(self.state.load(Ordering::SeqCst)).into()
    }

    /// Check if daemon is running.
    pub fn is_running(&self) -> bool {
        self.state.load(Ordering::SeqCst) == DaemonStateValue::Running as u8
    }

    /// Get the signal handler.
    pub fn signal_handler(&self) -> &SignalHandler {
        &self.signal_handler
    }

    /// Get the health checker.
    pub fn health_checker(&self) -> &Arc<HealthChecker> {
        &self.health_checker
    }

    /// Subscribe to shutdown notifications.
    pub fn shutdown_receiver(&self) -> broadcast::Receiver<()> {
        self.shutdown_sender.subscribe()
    }

    /// Register a health check component.
    pub async fn register_health_check(&self, component: Arc<dyn HealthCheckable>) {
        self.health_checker.register(component).await;
    }

    /// Start the daemon.
    pub async fn start(&self) -> Result<(), DaemonError> {
        let current = self.state.load(Ordering::SeqCst);
        if current != DaemonStateValue::Stopped as u8 {
            return Err(DaemonError::InvalidStateTransition {
                from: DaemonStateValue::from(current).into(),
                to: ErrorDaemonState::Starting,
            });
        }

        self.state
            .store(DaemonStateValue::Starting as u8, Ordering::SeqCst);
        info!("Daemon starting...");

        // Try to acquire PID file lock
        {
            let mut pid_file = self.pid_file.write().await;
            pid_file.try_acquire()?;
        }

        // Set up signal handlers
        self.signal_handler.setup_os_signals().await?;

        // Register default health checks
        self.health_checker
            .register(Arc::new(LivenessCheck))
            .await;

        // Daemonize if configured (Unix only)
        #[cfg(unix)]
        if self.config.daemonize {
            self.daemonize()?;
        }

        self.state
            .store(DaemonStateValue::Running as u8, Ordering::SeqCst);
        info!("Daemon started (PID: {})", std::process::id());

        Ok(())
    }

    /// Stop the daemon gracefully.
    pub async fn stop(&self) -> Result<(), DaemonError> {
        let current = self.state.load(Ordering::SeqCst);
        if current != DaemonStateValue::Running as u8
            && current != DaemonStateValue::Restarting as u8
        {
            return Err(DaemonError::InvalidStateTransition {
                from: DaemonStateValue::from(current).into(),
                to: ErrorDaemonState::ShuttingDown,
            });
        }

        self.state
            .store(DaemonStateValue::ShuttingDown as u8, Ordering::SeqCst);
        info!("Daemon shutting down...");

        // Send shutdown signal
        let _ = self.shutdown_sender.send(());

        // Remove PID file
        {
            let mut pid_file = self.pid_file.write().await;
            pid_file.remove()?;
        }

        self.state
            .store(DaemonStateValue::Stopped as u8, Ordering::SeqCst);
        info!("Daemon stopped");

        Ok(())
    }

    /// Run the daemon main loop.
    pub async fn run<F, Fut>(&self, main_fn: F) -> Result<(), DaemonError>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<(), DaemonError>> + Send,
    {
        self.start().await?;

        // Start health check loop
        let health_checker = self.health_checker.clone();
        let shutdown_rx = self.shutdown_receiver();
        tokio::spawn(async move {
            health_checker.start_loop(shutdown_rx).await;
        });

        // Main loop with restart support
        loop {
            let mut signal_rx = self.signal_handler.subscribe();

            tokio::select! {
                result = main_fn() => {
                    match result {
                        Ok(()) => {
                            info!("Main function completed normally");
                            break;
                        }
                        Err(e) => {
                            error!("Main function error: {}", e);

                            if self.config.auto_restart {
                                if self.should_restart().await? {
                                    warn!("Restarting after error...");
                                    tokio::time::sleep(self.config.restart_delay()).await;
                                    continue;
                                } else {
                                    return Err(e);
                                }
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
                signal = signal_rx.recv() => {
                    match signal {
                        Ok(DaemonSignal::Shutdown) | Ok(DaemonSignal::Terminate) => {
                            info!("Received shutdown signal");
                            break;
                        }
                        Ok(DaemonSignal::Reload) => {
                            info!("Received reload signal - config reload not yet implemented");
                            self.signal_handler.clear_reload_flag();
                        }
                        Err(_) => {
                            // Channel closed, exit
                            break;
                        }
                    }
                }
            }
        }

        self.stop().await
    }

    /// Check if we should restart based on restart limits.
    async fn should_restart(&self) -> Result<bool, DaemonError> {
        let mut tracker = self.restart_tracker.write().await;
        if tracker.record_restart() {
            error!(
                "Maximum restarts ({}) exceeded in {:?}",
                self.config.max_restarts,
                self.config.restart_window()
            );
            return Err(DaemonError::MaxRestartsExceeded {
                max: self.config.max_restarts,
            });
        }

        info!(
            "Restart {}/{} in current window",
            tracker.count(),
            self.config.max_restarts
        );
        Ok(true)
    }

    /// Daemonize the process (Unix only).
    #[cfg(unix)]
    fn daemonize(&self) -> Result<(), DaemonError> {
        use nix::unistd::{chdir, dup2, fork, setsid, ForkResult};
        use std::os::unix::io::AsRawFd;

        info!("Daemonizing process...");

        // First fork
        match unsafe { fork() } {
            Ok(ForkResult::Parent { .. }) => {
                // Parent exits
                std::process::exit(0);
            }
            Ok(ForkResult::Child) => {
                // Child continues
            }
            Err(e) => {
                return Err(DaemonError::ForkFailed(e.to_string()));
            }
        }

        // Create new session
        setsid().map_err(|e| DaemonError::ForkFailed(format!("setsid failed: {}", e)))?;

        // Second fork to prevent acquiring a controlling terminal
        match unsafe { fork() } {
            Ok(ForkResult::Parent { .. }) => {
                std::process::exit(0);
            }
            Ok(ForkResult::Child) => {
                // Grandchild continues as daemon
            }
            Err(e) => {
                return Err(DaemonError::ForkFailed(e.to_string()));
            }
        }

        // Change to work directory
        if let Some(ref work_dir) = self.config.work_dir {
            chdir(work_dir.as_path())
                .map_err(|e| DaemonError::ForkFailed(format!("chdir failed: {}", e)))?;
        }

        // Redirect standard streams to /dev/null
        let dev_null = std::fs::File::open("/dev/null")
            .map_err(|e| DaemonError::ForkFailed(format!("Failed to open /dev/null: {}", e)))?;

        let fd = dev_null.as_raw_fd();
        dup2(fd, 0).ok(); // stdin
        dup2(fd, 1).ok(); // stdout
        dup2(fd, 2).ok(); // stderr

        info!("Process daemonized (PID: {})", std::process::id());
        Ok(())
    }

    /// Get the PID of a running daemon, if any.
    pub async fn get_running_pid(&self) -> Result<Option<u32>, DaemonError> {
        let pid_file = self.pid_file.read().await;
        if let Some(pid) = pid_file.read_pid()? {
            if PidFile::is_process_running(pid) {
                return Ok(Some(pid));
            }
        }
        Ok(None)
    }

    /// Check the status of the daemon.
    pub async fn status(&self) -> DaemonStatus {
        let state = self.state();
        let pid = self.get_running_pid().await.ok().flatten();

        DaemonStatus {
            state,
            pid,
            health_checks: self.health_checker.check_count(),
            health_failures: self.health_checker.failure_count(),
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_state_conversion() {
        assert_eq!(DaemonStateValue::from(0), DaemonStateValue::Stopped);
        assert_eq!(DaemonStateValue::from(1), DaemonStateValue::Starting);
        assert_eq!(DaemonStateValue::from(2), DaemonStateValue::Running);
        assert_eq!(DaemonStateValue::from(3), DaemonStateValue::ShuttingDown);
        assert_eq!(DaemonStateValue::from(4), DaemonStateValue::Restarting);
        assert_eq!(DaemonStateValue::from(99), DaemonStateValue::Stopped);
    }

    #[test]
    fn test_daemon_new() {
        let config = DaemonConfig::default();
        let daemon = Daemon::new(config).unwrap();
        assert_eq!(daemon.state(), DaemonState::Stopped);
        assert!(!daemon.is_running());
    }

    #[test]
    fn test_daemon_invalid_config() {
        let mut config = DaemonConfig::default();
        config.restart_window_secs = 0;
        let result = Daemon::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_restart_tracker() {
        let config = DaemonConfig {
            max_restarts: 3,
            restart_window_secs: 60,
            ..Default::default()
        };
        let mut tracker = RestartTracker::new(&config);

        // First 3 restarts should be OK
        assert!(!tracker.record_restart());
        assert!(!tracker.record_restart());
        assert!(!tracker.record_restart());
        assert_eq!(tracker.count(), 3);

        // 4th restart should exceed limit
        assert!(tracker.record_restart());
    }

    #[tokio::test]
    async fn test_daemon_status() {
        let config = DaemonConfig::default();
        let daemon = Daemon::new(config).unwrap();
        let status = daemon.status().await;

        assert_eq!(status.state, DaemonState::Stopped);
        assert!(status.pid.is_none());
    }

    #[test]
    fn test_daemon_status_display() {
        let status = DaemonStatus {
            state: DaemonState::Running,
            pid: Some(12345),
            health_checks: 100,
            health_failures: 5,
        };

        let display = status.to_string();
        assert!(display.contains("running"));
        assert!(display.contains("12345"));
        assert!(display.contains("95/100"));
    }
}
