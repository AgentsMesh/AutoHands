//! Lifecycle adapter implementations for kernel components.
//!
//! Adapters wrap various component types (scheduler, queue, triggers, monitor,
//! RunLoop) as LifecycleHook implementations for unified lifecycle management.

use std::sync::Arc;

use tracing::debug;

use autohands_protocols::error::ExtensionError;

use super::{priority, LifecycleHook};

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
