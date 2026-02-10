//! Kernel lifecycle management.
//!
//! Provides lifecycle management for kernel components including:
//! - Component startup/shutdown ordering via priority
//! - Graceful shutdown with timeout
//! - Integration points for daemon, scheduler, queue, and other 24/7 components

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, RwLock};
use tokio::time::timeout;
use tracing::{error, info, warn};

use autohands_protocols::error::ExtensionError;

#[path = "lifecycle_adapters.rs"]
mod lifecycle_adapters;
pub use lifecycle_adapters::*;

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;

/// Kernel state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum KernelState {
    /// Initial state, not started.
    Created = 0,
    /// Starting up.
    Starting = 1,
    /// Running and ready.
    Running = 2,
    /// Shutting down.
    ShuttingDown = 3,
    /// Stopped.
    Stopped = 4,
}

impl From<u8> for KernelState {
    fn from(v: u8) -> Self {
        match v {
            0 => KernelState::Created,
            1 => KernelState::Starting,
            2 => KernelState::Running,
            3 => KernelState::ShuttingDown,
            4 => KernelState::Stopped,
            _ => KernelState::Created,
        }
    }
}

/// Shutdown signal for graceful shutdown.
#[derive(Clone)]
pub struct ShutdownSignal {
    sender: broadcast::Sender<()>,
}

impl ShutdownSignal {
    /// Create a new shutdown signal.
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1);
        Self { sender }
    }

    /// Trigger shutdown.
    pub fn trigger(&self) {
        let _ = self.sender.send(());
    }

    /// Subscribe to shutdown signal.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.sender.subscribe()
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// Lifecycle hook trait for components.
#[async_trait::async_trait]
pub trait LifecycleHook: Send + Sync {
    /// Called during startup.
    async fn on_start(&self) -> Result<(), ExtensionError>;

    /// Called during shutdown.
    async fn on_stop(&self) -> Result<(), ExtensionError>;

    /// Priority for startup/shutdown ordering (higher = earlier start, later stop).
    fn priority(&self) -> i32 {
        0
    }
}

/// Lifecycle manager for kernel components.
pub struct LifecycleManager {
    state: AtomicU8,
    hooks: RwLock<Vec<Arc<dyn LifecycleHook>>>,
    shutdown_signal: ShutdownSignal,
    shutdown_timeout: Duration,
}

impl LifecycleManager {
    /// Create a new lifecycle manager.
    pub fn new(shutdown_timeout: Duration) -> Self {
        Self {
            state: AtomicU8::new(KernelState::Created as u8),
            hooks: RwLock::new(Vec::new()),
            shutdown_signal: ShutdownSignal::new(),
            shutdown_timeout,
        }
    }

    /// Get current state.
    pub fn state(&self) -> KernelState {
        KernelState::from(self.state.load(Ordering::SeqCst))
    }

    /// Register a lifecycle hook.
    pub async fn register_hook(&self, hook: Arc<dyn LifecycleHook>) {
        let mut hooks = self.hooks.write().await;
        hooks.push(hook);
        // Sort by priority (higher first for startup)
        hooks.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Get shutdown signal.
    pub fn shutdown_signal(&self) -> &ShutdownSignal {
        &self.shutdown_signal
    }

    /// Start all components.
    pub async fn start(&self) -> Result<(), ExtensionError> {
        let current = self.state.load(Ordering::SeqCst);
        if current != KernelState::Created as u8 {
            return Err(ExtensionError::Custom(format!(
                "Cannot start from state: {:?}",
                KernelState::from(current)
            )));
        }

        self.state
            .store(KernelState::Starting as u8, Ordering::SeqCst);
        info!("Kernel starting...");

        let hooks = self.hooks.read().await;
        for (i, hook) in hooks.iter().enumerate() {
            if let Err(e) = hook.on_start().await {
                error!("Failed to start hook {}: {}", i, e);
                // Rollback started hooks
                for started_hook in hooks.iter().take(i).rev() {
                    let _ = started_hook.on_stop().await;
                }
                self.state
                    .store(KernelState::Stopped as u8, Ordering::SeqCst);
                return Err(e);
            }
        }

        self.state
            .store(KernelState::Running as u8, Ordering::SeqCst);
        info!("Kernel started");
        Ok(())
    }

    /// Stop all components.
    pub async fn stop(&self) -> Result<(), ExtensionError> {
        let current = self.state.load(Ordering::SeqCst);
        if current != KernelState::Running as u8 {
            return Err(ExtensionError::Custom(format!(
                "Cannot stop from state: {:?}",
                KernelState::from(current)
            )));
        }

        self.state
            .store(KernelState::ShuttingDown as u8, Ordering::SeqCst);
        info!("Kernel shutting down...");

        // Signal shutdown
        self.shutdown_signal.trigger();

        // Stop hooks in reverse order
        let hooks = self.hooks.read().await;
        let mut errors = Vec::new();

        for hook in hooks.iter().rev() {
            match timeout(self.shutdown_timeout, hook.on_stop()).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    warn!("Hook stop error: {}", e);
                    errors.push(e);
                }
                Err(_) => {
                    warn!("Hook stop timeout");
                    errors.push(ExtensionError::Timeout);
                }
            }
        }

        self.state
            .store(KernelState::Stopped as u8, Ordering::SeqCst);
        info!("Kernel stopped");

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ExtensionError::Custom(format!(
                "{} hooks failed during shutdown",
                errors.len()
            )))
        }
    }

    /// Check if running.
    pub fn is_running(&self) -> bool {
        self.state.load(Ordering::SeqCst) == KernelState::Running as u8
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

/// Priority levels for lifecycle components (higher = starts earlier, stops later).
pub mod priority {
    pub const INFRASTRUCTURE: i32 = 1000;
    pub const DAEMON: i32 = 900;
    pub const RUNLOOP: i32 = 850;
    pub const MONITOR: i32 = 800;
    pub const QUEUE: i32 = 700;
    pub const SCHEDULER: i32 = 600;
    pub const TRIGGERS: i32 = 500;
    pub const ORCHESTRATOR: i32 = 400;
    pub const MEMORY: i32 = 300;
    pub const PROVIDERS: i32 = 200;
    pub const EXTENSIONS: i32 = 100;
    pub const DEFAULT: i32 = 0;
}
