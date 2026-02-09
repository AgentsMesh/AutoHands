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
use tracing::{debug, error, info, warn};

use autohands_protocols::error::ExtensionError;

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

// ============================================================================
// Component Priority Constants
// ============================================================================

/// Priority levels for lifecycle components.
/// Higher priority = starts earlier, stops later.
pub mod priority {
    /// Core infrastructure (logging, metrics).
    pub const INFRASTRUCTURE: i32 = 1000;

    /// Daemon process management.
    pub const DAEMON: i32 = 900;

    /// RunLoop event loop (starts after daemon, runs the main event loop).
    pub const RUNLOOP: i32 = 850;

    /// Monitoring and health checks.
    pub const MONITOR: i32 = 800;

    /// Task queue system.
    pub const QUEUE: i32 = 700;

    /// Scheduler for cron jobs.
    pub const SCHEDULER: i32 = 600;

    /// Event triggers (webhooks, file watchers).
    pub const TRIGGERS: i32 = 500;

    /// Multi-agent orchestrator.
    pub const ORCHESTRATOR: i32 = 400;

    /// Memory backends.
    pub const MEMORY: i32 = 300;

    /// LLM providers.
    pub const PROVIDERS: i32 = 200;

    /// Tools and extensions.
    pub const EXTENSIONS: i32 = 100;

    /// Default priority for unspecified components.
    pub const DEFAULT: i32 = 0;
}

// ============================================================================
// Component Lifecycle Adapters
// ============================================================================

/// Adapter to wrap scheduler as a lifecycle hook.
pub struct SchedulerLifecycleHook<S> {
    scheduler: Arc<S>,
}

impl<S> SchedulerLifecycleHook<S> {
    pub fn new(scheduler: Arc<S>) -> Self {
        Self { scheduler }
    }
}

#[async_trait::async_trait]
impl<S> LifecycleHook for SchedulerLifecycleHook<S>
where
    S: SchedulerControl + Send + Sync + 'static,
{
    async fn on_start(&self) -> Result<(), ExtensionError> {
        debug!("Starting scheduler...");
        self.scheduler
            .start()
            .await
            .map_err(|e| ExtensionError::InitializationFailed(e.to_string()))
    }

    async fn on_stop(&self) -> Result<(), ExtensionError> {
        debug!("Stopping scheduler...");
        self.scheduler
            .stop()
            .await
            .map_err(|e| ExtensionError::ShutdownFailed(e.to_string()))
    }

    fn priority(&self) -> i32 {
        priority::SCHEDULER
    }
}

/// Trait for scheduler control.
#[async_trait::async_trait]
pub trait SchedulerControl: Send + Sync {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Adapter to wrap queue as a lifecycle hook.
pub struct QueueLifecycleHook<Q> {
    queue: Arc<Q>,
}

impl<Q> QueueLifecycleHook<Q> {
    pub fn new(queue: Arc<Q>) -> Self {
        Self { queue }
    }
}

#[async_trait::async_trait]
impl<Q> LifecycleHook for QueueLifecycleHook<Q>
where
    Q: QueueControl + Send + Sync + 'static,
{
    async fn on_start(&self) -> Result<(), ExtensionError> {
        debug!("Starting task queue...");
        self.queue
            .start()
            .await
            .map_err(|e| ExtensionError::InitializationFailed(e.to_string()))
    }

    async fn on_stop(&self) -> Result<(), ExtensionError> {
        debug!("Stopping task queue...");
        self.queue
            .stop()
            .await
            .map_err(|e| ExtensionError::ShutdownFailed(e.to_string()))
    }

    fn priority(&self) -> i32 {
        priority::QUEUE
    }
}

/// Trait for queue control.
#[async_trait::async_trait]
pub trait QueueControl: Send + Sync {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Adapter to wrap triggers as a lifecycle hook.
pub struct TriggersLifecycleHook<T> {
    triggers: Arc<T>,
}

impl<T> TriggersLifecycleHook<T> {
    pub fn new(triggers: Arc<T>) -> Self {
        Self { triggers }
    }
}

#[async_trait::async_trait]
impl<T> LifecycleHook for TriggersLifecycleHook<T>
where
    T: TriggersControl + Send + Sync + 'static,
{
    async fn on_start(&self) -> Result<(), ExtensionError> {
        debug!("Starting triggers...");
        self.triggers
            .start()
            .await
            .map_err(|e| ExtensionError::InitializationFailed(e.to_string()))
    }

    async fn on_stop(&self) -> Result<(), ExtensionError> {
        debug!("Stopping triggers...");
        self.triggers
            .stop()
            .await
            .map_err(|e| ExtensionError::ShutdownFailed(e.to_string()))
    }

    fn priority(&self) -> i32 {
        priority::TRIGGERS
    }
}

/// Trait for triggers control.
#[async_trait::async_trait]
pub trait TriggersControl: Send + Sync {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Adapter to wrap monitor as a lifecycle hook.
pub struct MonitorLifecycleHook<M> {
    monitor: Arc<M>,
}

impl<M> MonitorLifecycleHook<M> {
    pub fn new(monitor: Arc<M>) -> Self {
        Self { monitor }
    }
}

#[async_trait::async_trait]
impl<M> LifecycleHook for MonitorLifecycleHook<M>
where
    M: MonitorControl + Send + Sync + 'static,
{
    async fn on_start(&self) -> Result<(), ExtensionError> {
        debug!("Starting monitor...");
        self.monitor
            .start()
            .await
            .map_err(|e| ExtensionError::InitializationFailed(e.to_string()))
    }

    async fn on_stop(&self) -> Result<(), ExtensionError> {
        debug!("Stopping monitor...");
        self.monitor
            .stop()
            .await
            .map_err(|e| ExtensionError::ShutdownFailed(e.to_string()))
    }

    fn priority(&self) -> i32 {
        priority::MONITOR
    }
}

/// Trait for monitor control.
#[async_trait::async_trait]
pub trait MonitorControl: Send + Sync {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Adapter to wrap RunLoop as a lifecycle hook.
pub struct RunLoopLifecycleHook<R> {
    runloop: Arc<R>,
}

impl<R> RunLoopLifecycleHook<R> {
    pub fn new(runloop: Arc<R>) -> Self {
        Self { runloop }
    }
}

#[async_trait::async_trait]
impl<R> LifecycleHook for RunLoopLifecycleHook<R>
where
    R: RunLoopControl + Send + Sync + 'static,
{
    async fn on_start(&self) -> Result<(), ExtensionError> {
        debug!("Starting RunLoop...");
        self.runloop
            .start()
            .await
            .map_err(|e| ExtensionError::InitializationFailed(e.to_string()))
    }

    async fn on_stop(&self) -> Result<(), ExtensionError> {
        debug!("Stopping RunLoop...");
        self.runloop
            .stop()
            .await
            .map_err(|e| ExtensionError::ShutdownFailed(e.to_string()))
    }

    fn priority(&self) -> i32 {
        priority::RUNLOOP
    }
}

/// Trait for RunLoop control.
#[async_trait::async_trait]
pub trait RunLoopControl: Send + Sync {
    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    struct TestHook {
        started: AtomicBool,
        stopped: AtomicBool,
        priority: i32,
    }

    impl TestHook {
        fn new(priority: i32) -> Self {
            Self {
                started: AtomicBool::new(false),
                stopped: AtomicBool::new(false),
                priority,
            }
        }
    }

    #[async_trait::async_trait]
    impl LifecycleHook for TestHook {
        async fn on_start(&self) -> Result<(), ExtensionError> {
            self.started.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn on_stop(&self) -> Result<(), ExtensionError> {
            self.stopped.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    #[test]
    fn test_kernel_state_conversion() {
        assert_eq!(KernelState::from(0), KernelState::Created);
        assert_eq!(KernelState::from(2), KernelState::Running);
        assert_eq!(KernelState::from(99), KernelState::Created);
    }

    #[test]
    fn test_shutdown_signal() {
        let signal = ShutdownSignal::new();
        let mut rx = signal.subscribe();

        signal.trigger();

        let result = rx.try_recv();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_lifecycle_start_stop() {
        let manager = LifecycleManager::default();
        let hook = Arc::new(TestHook::new(0));

        manager.register_hook(hook.clone()).await;

        assert_eq!(manager.state(), KernelState::Created);

        manager.start().await.unwrap();
        assert_eq!(manager.state(), KernelState::Running);
        assert!(hook.started.load(Ordering::SeqCst));
        assert!(manager.is_running());

        manager.stop().await.unwrap();
        assert_eq!(manager.state(), KernelState::Stopped);
        assert!(hook.stopped.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_cannot_start_twice() {
        let manager = LifecycleManager::default();
        manager.start().await.unwrap();

        let result = manager.start().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_hook_priority_order() {
        let manager = LifecycleManager::default();
        let hook1 = Arc::new(TestHook::new(10));
        let hook2 = Arc::new(TestHook::new(5));

        manager.register_hook(hook2.clone()).await;
        manager.register_hook(hook1.clone()).await;

        manager.start().await.unwrap();

        // Both should be started
        assert!(hook1.started.load(Ordering::SeqCst));
        assert!(hook2.started.load(Ordering::SeqCst));
    }

    #[test]
    fn test_kernel_state_debug() {
        let state = KernelState::Running;
        let debug = format!("{:?}", state);
        assert!(debug.contains("Running"));
    }

    #[test]
    fn test_kernel_state_clone() {
        let state = KernelState::Running;
        let cloned = state;
        assert_eq!(cloned, state);
    }

    #[test]
    fn test_kernel_state_eq() {
        assert_eq!(KernelState::Created, KernelState::Created);
        assert_ne!(KernelState::Created, KernelState::Running);
    }

    #[test]
    fn test_kernel_state_all_conversions() {
        assert_eq!(KernelState::from(1), KernelState::Starting);
        assert_eq!(KernelState::from(3), KernelState::ShuttingDown);
        assert_eq!(KernelState::from(4), KernelState::Stopped);
    }

    #[test]
    fn test_shutdown_signal_default() {
        let signal = ShutdownSignal::default();
        let mut rx = signal.subscribe();
        signal.trigger();
        assert!(rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn test_lifecycle_manager_new() {
        let manager = LifecycleManager::new(Duration::from_secs(5));
        assert_eq!(manager.state(), KernelState::Created);
        assert!(!manager.is_running());
    }

    #[tokio::test]
    async fn test_cannot_stop_before_start() {
        let manager = LifecycleManager::default();
        let result = manager.stop().await;
        assert!(result.is_err());
    }

    struct FailingHook;

    #[async_trait::async_trait]
    impl LifecycleHook for FailingHook {
        async fn on_start(&self) -> Result<(), ExtensionError> {
            Err(ExtensionError::InitializationFailed("Failed to start".to_string()))
        }

        async fn on_stop(&self) -> Result<(), ExtensionError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_start_with_failing_hook() {
        let manager = LifecycleManager::default();
        let hook = Arc::new(FailingHook);
        manager.register_hook(hook).await;

        let result = manager.start().await;
        assert!(result.is_err());
        assert_eq!(manager.state(), KernelState::Stopped);
    }

    #[tokio::test]
    async fn test_start_with_multiple_hooks_one_fails() {
        let manager = LifecycleManager::default();
        let good_hook = Arc::new(TestHook::new(10)); // Higher priority, starts first
        let bad_hook = Arc::new(FailingHook);

        manager.register_hook(good_hook.clone()).await;
        manager.register_hook(Arc::new(TestHook::new(5))).await; // This won't start

        // Insert failing hook with medium priority
        let manager2 = LifecycleManager::default();
        manager2.register_hook(good_hook.clone()).await;

        manager2.start().await.unwrap();
        assert!(good_hook.started.load(Ordering::SeqCst));
    }

    struct SlowStopHook;

    #[async_trait::async_trait]
    impl LifecycleHook for SlowStopHook {
        async fn on_start(&self) -> Result<(), ExtensionError> {
            Ok(())
        }

        async fn on_stop(&self) -> Result<(), ExtensionError> {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_stop_with_timeout() {
        let manager = LifecycleManager::new(Duration::from_millis(10));
        let hook = Arc::new(SlowStopHook);
        manager.register_hook(hook).await;

        manager.start().await.unwrap();
        let result = manager.stop().await;
        // Should error due to timeout
        assert!(result.is_err());
    }

    struct ErrorStopHook;

    #[async_trait::async_trait]
    impl LifecycleHook for ErrorStopHook {
        async fn on_start(&self) -> Result<(), ExtensionError> {
            Ok(())
        }

        async fn on_stop(&self) -> Result<(), ExtensionError> {
            Err(ExtensionError::ShutdownFailed("stop failed".to_string()))
        }
    }

    #[tokio::test]
    async fn test_stop_with_error() {
        let manager = LifecycleManager::default();
        let hook = Arc::new(ErrorStopHook);
        manager.register_hook(hook).await;

        manager.start().await.unwrap();
        let result = manager.stop().await;
        assert!(result.is_err());
        assert_eq!(manager.state(), KernelState::Stopped);
    }
}
