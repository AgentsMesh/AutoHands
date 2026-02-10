//! Daemon implementation methods.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use tracing::{error, info, warn};

use crate::error::{DaemonError, DaemonState as ErrorDaemonState};
use crate::health::{HealthCheckable, LivenessCheck};
use crate::pid::PidFile;
use crate::signal::DaemonSignal;

use crate::daemon::{Daemon, DaemonStateValue};
use crate::daemon_status::DaemonStatus;

impl Daemon {
    /// Get the current daemon state.
    pub fn state(&self) -> crate::daemon_status::DaemonState {
        DaemonStateValue::from(self.state.load(Ordering::SeqCst)).into()
    }

    /// Check if daemon is running.
    pub fn is_running(&self) -> bool {
        self.state.load(Ordering::SeqCst) == DaemonStateValue::Running as u8
    }

    /// Get the signal handler.
    pub fn signal_handler(&self) -> &crate::signal::SignalHandler {
        &self.signal_handler
    }

    /// Get the health checker.
    pub fn health_checker(&self) -> &Arc<crate::health::HealthChecker> {
        &self.health_checker
    }

    /// Subscribe to shutdown notifications.
    pub fn shutdown_receiver(&self) -> tokio::sync::broadcast::Receiver<()> {
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
