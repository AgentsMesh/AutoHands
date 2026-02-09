//! Signal handling for daemon processes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::{debug, info};

use crate::error::DaemonError;

/// Signal type for daemon control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonSignal {
    /// Graceful shutdown (SIGTERM, SIGINT).
    Shutdown,
    /// Reload configuration (SIGHUP).
    Reload,
    /// Force terminate (for internal use).
    Terminate,
}

impl std::fmt::Display for DaemonSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonSignal::Shutdown => write!(f, "SHUTDOWN"),
            DaemonSignal::Reload => write!(f, "RELOAD"),
            DaemonSignal::Terminate => write!(f, "TERMINATE"),
        }
    }
}

/// Signal handler for managing daemon lifecycle signals.
#[derive(Clone)]
pub struct SignalHandler {
    sender: broadcast::Sender<DaemonSignal>,
    shutdown_requested: Arc<AtomicBool>,
    reload_requested: Arc<AtomicBool>,
}

impl SignalHandler {
    /// Create a new signal handler.
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(16);
        Self {
            sender,
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            reload_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Subscribe to signals.
    pub fn subscribe(&self) -> broadcast::Receiver<DaemonSignal> {
        self.sender.subscribe()
    }

    /// Send a signal.
    pub fn send(&self, signal: DaemonSignal) {
        debug!("Sending signal: {}", signal);

        match signal {
            DaemonSignal::Shutdown | DaemonSignal::Terminate => {
                self.shutdown_requested.store(true, Ordering::SeqCst);
            }
            DaemonSignal::Reload => {
                self.reload_requested.store(true, Ordering::SeqCst);
            }
        }

        let _ = self.sender.send(signal);
    }

    /// Request shutdown.
    pub fn request_shutdown(&self) {
        self.send(DaemonSignal::Shutdown);
    }

    /// Request configuration reload.
    pub fn request_reload(&self) {
        self.send(DaemonSignal::Reload);
    }

    /// Check if shutdown has been requested.
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }

    /// Check if reload has been requested.
    pub fn is_reload_requested(&self) -> bool {
        self.reload_requested.load(Ordering::SeqCst)
    }

    /// Clear the reload flag after handling.
    pub fn clear_reload_flag(&self) {
        self.reload_requested.store(false, Ordering::SeqCst);
    }

    /// Set up OS signal handlers (Unix only).
    #[cfg(unix)]
    pub async fn setup_os_signals(&self) -> Result<(), DaemonError> {
        use tokio::signal::unix::{signal, SignalKind};

        let handler = self.clone();

        // SIGTERM handler
        let mut sigterm =
            signal(SignalKind::terminate()).map_err(|e| DaemonError::SignalSetup(e.to_string()))?;

        let sigterm_handler = handler.clone();
        tokio::spawn(async move {
            while sigterm.recv().await.is_some() {
                info!("Received SIGTERM");
                sigterm_handler.request_shutdown();
            }
        });

        // SIGINT handler
        let mut sigint =
            signal(SignalKind::interrupt()).map_err(|e| DaemonError::SignalSetup(e.to_string()))?;

        let sigint_handler = handler.clone();
        tokio::spawn(async move {
            while sigint.recv().await.is_some() {
                info!("Received SIGINT");
                sigint_handler.request_shutdown();
            }
        });

        // SIGHUP handler (reload config)
        let mut sighup =
            signal(SignalKind::hangup()).map_err(|e| DaemonError::SignalSetup(e.to_string()))?;

        let sighup_handler = handler.clone();
        tokio::spawn(async move {
            while sighup.recv().await.is_some() {
                info!("Received SIGHUP - requesting config reload");
                sighup_handler.request_reload();
            }
        });

        info!("OS signal handlers installed (SIGTERM, SIGINT, SIGHUP)");
        Ok(())
    }

    /// Set up OS signal handlers (non-Unix fallback).
    #[cfg(not(unix))]
    pub async fn setup_os_signals(&self) -> Result<(), DaemonError> {
        let handler = self.clone();

        // Only Ctrl+C is available on non-Unix
        tokio::spawn(async move {
            if let Ok(()) = tokio::signal::ctrl_c().await {
                info!("Received Ctrl+C");
                handler.request_shutdown();
            }
        });

        info!("OS signal handlers installed (Ctrl+C only)");
        Ok(())
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Send a signal to a running daemon process.
#[cfg(unix)]
pub fn send_signal_to_pid(pid: u32, signal: DaemonSignal) -> Result<(), DaemonError> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    let nix_signal = match signal {
        DaemonSignal::Shutdown => Signal::SIGTERM,
        DaemonSignal::Reload => Signal::SIGHUP,
        DaemonSignal::Terminate => Signal::SIGKILL,
    };

    kill(Pid::from_raw(pid as i32), nix_signal).map_err(|e| {
        DaemonError::Custom(format!("Failed to send {} to PID {}: {}", signal, pid, e))
    })?;

    info!("Sent {} to PID {}", signal, pid);
    Ok(())
}

#[cfg(not(unix))]
pub fn send_signal_to_pid(_pid: u32, _signal: DaemonSignal) -> Result<(), DaemonError> {
    Err(DaemonError::Custom(
        "Signal sending not supported on this platform".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_display() {
        assert_eq!(DaemonSignal::Shutdown.to_string(), "SHUTDOWN");
        assert_eq!(DaemonSignal::Reload.to_string(), "RELOAD");
        assert_eq!(DaemonSignal::Terminate.to_string(), "TERMINATE");
    }

    #[test]
    fn test_signal_handler_new() {
        let handler = SignalHandler::new();
        assert!(!handler.is_shutdown_requested());
        assert!(!handler.is_reload_requested());
    }

    #[test]
    fn test_request_shutdown() {
        let handler = SignalHandler::new();
        handler.request_shutdown();
        assert!(handler.is_shutdown_requested());
    }

    #[test]
    fn test_request_reload() {
        let handler = SignalHandler::new();
        handler.request_reload();
        assert!(handler.is_reload_requested());
    }

    #[test]
    fn test_clear_reload_flag() {
        let handler = SignalHandler::new();
        handler.request_reload();
        assert!(handler.is_reload_requested());

        handler.clear_reload_flag();
        assert!(!handler.is_reload_requested());
    }

    #[tokio::test]
    async fn test_signal_subscription() {
        let handler = SignalHandler::new();
        let mut rx = handler.subscribe();

        handler.send(DaemonSignal::Shutdown);

        let received = rx.recv().await.unwrap();
        assert_eq!(received, DaemonSignal::Shutdown);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let handler = SignalHandler::new();
        let mut rx1 = handler.subscribe();
        let mut rx2 = handler.subscribe();

        handler.send(DaemonSignal::Reload);

        assert_eq!(rx1.recv().await.unwrap(), DaemonSignal::Reload);
        assert_eq!(rx2.recv().await.unwrap(), DaemonSignal::Reload);
    }

    #[test]
    fn test_signal_eq() {
        assert_eq!(DaemonSignal::Shutdown, DaemonSignal::Shutdown);
        assert_ne!(DaemonSignal::Shutdown, DaemonSignal::Reload);
    }

    #[test]
    fn test_signal_clone() {
        let signal = DaemonSignal::Shutdown;
        let cloned = signal;
        assert_eq!(signal, cloned);
    }

    #[test]
    fn test_handler_clone() {
        let handler = SignalHandler::new();
        let cloned = handler.clone();

        handler.request_shutdown();
        // Cloned handler shares the same state
        assert!(cloned.is_shutdown_requested());
    }
}
