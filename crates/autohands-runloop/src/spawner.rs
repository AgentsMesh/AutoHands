//! RunLoop-aware task spawner for unified async task management.
//!
//! This module provides a unified way to spawn async tasks that:
//! - Inherit correlation IDs for event tracing
//! - Respect RunLoop mode constraints
//! - Support unified cancellation
//! - Track spawned tasks for observability
//!
//! ## Why Use RunLoopSpawner?
//!
//! Direct `tokio::spawn` calls bypass RunLoop's event tracking and mode control.
//! Using `RunLoopSpawner` ensures:
//! 1. All async work is traceable through correlation IDs
//! 2. Tasks respect the current RunLoop mode
//! 3. Tasks can be cancelled when RunLoop stops
//! 4. Task metrics are collected
//!
//! ## Example
//!
//! ```rust,no_run
//! use autohands_runloop::RunLoopSpawner;
//!
//! async fn example() {
//!     let spawner = RunLoopSpawner::new();
//!
//!     // Spawn a task with correlation tracking
//!     let handle = spawner.spawn("fetch-data", async {
//!         // Task work here
//!         42
//!     }).await;
//!
//!     // Wait for result
//!     let result = handle.await.unwrap();
//! }
//! ```

use std::future::Future;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn, Instrument};
use uuid::Uuid;

use crate::mode::RunLoopState;

// Re-export types that were originally defined in this module so that
// `crate::spawner::TaskState` etc. paths continue to work.
pub use crate::correlation::CorrelationGuard;
pub use crate::spawner_types::{
    SpawnedTaskHandle, SpawnerInner, SpawnerMetrics, SpawnerStateProvider, TaskInfo, TaskState,
};

/// RunLoop-aware task spawner.
///
/// Provides unified async task management with:
/// - Correlation ID inheritance for event tracing
/// - RunLoop state awareness
/// - Unified cancellation support
/// - Task metrics collection
pub struct RunLoopSpawner {
    /// State provider for RunLoop state checks.
    state_provider: Option<Arc<dyn SpawnerStateProvider>>,
    /// Current correlation ID context (wrapped in Arc for Drop support).
    correlation_context: Arc<RwLock<Option<String>>>,
    /// Inner state.
    inner: Arc<SpawnerInner>,
}

impl Default for RunLoopSpawner {
    fn default() -> Self {
        Self::new()
    }
}

impl RunLoopSpawner {
    /// Create a new standalone spawner (without RunLoop state awareness).
    pub fn new() -> Self {
        Self {
            state_provider: None,
            correlation_context: Arc::new(RwLock::new(None)),
            inner: Arc::new(SpawnerInner::new()),
        }
    }

    /// Create a spawner with a state provider.
    pub fn with_state_provider(state_provider: Arc<dyn SpawnerStateProvider>) -> Self {
        Self {
            state_provider: Some(state_provider),
            correlation_context: Arc::new(RwLock::new(None)),
            inner: Arc::new(SpawnerInner::new()),
        }
    }

    /// Get the inner state (for sharing with RunLoop).
    pub fn inner(&self) -> Arc<SpawnerInner> {
        self.inner.clone()
    }

    /// Get Arc reference to correlation context (for CorrelationGuard).
    pub(crate) fn correlation_context_arc(&self) -> Arc<RwLock<Option<String>>> {
        self.correlation_context.clone()
    }

    /// Set the current correlation context.
    ///
    /// Tasks spawned after this call will inherit this correlation ID.
    pub async fn set_correlation_context(&self, correlation_id: Option<String>) {
        *self.correlation_context.write().await = correlation_id;
    }

    /// Get the current correlation context.
    pub async fn correlation_context(&self) -> Option<String> {
        self.correlation_context.read().await.clone()
    }

    /// Spawn a task with RunLoop awareness.
    ///
    /// The task will:
    /// - Inherit the current correlation ID
    /// - Be tracked for metrics
    /// - Respect RunLoop state (won't spawn if stopping)
    pub async fn spawn<F, T>(&self, name: impl Into<String>, future: F) -> SpawnedTaskHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let task_id = Uuid::new_v4();
        let task_name = name.into();
        let correlation_id = self.correlation_context.read().await.clone();

        // Check RunLoop state if provider is available
        if let Some(ref provider) = self.state_provider {
            let run_loop_state = provider.state();
            if run_loop_state == RunLoopState::Stopping || run_loop_state == RunLoopState::Stopped {
                warn!(
                    "Spawning task '{}' while RunLoop is {:?}",
                    task_name, run_loop_state
                );
            }
        }

        // Create task info
        let info = TaskInfo {
            id: task_id,
            name: task_name.clone(),
            correlation_id: correlation_id.clone(),
            parent_correlation_id: None,
            state: TaskState::Running,
            spawned_at: chrono::Utc::now(),
            cancellable: false,
        };

        self.inner.register_task(info, None);

        // Create tracing span
        let span = tracing::info_span!(
            "spawned_task",
            task_id = %task_id,
            task_name = %task_name,
            correlation_id = ?correlation_id,
        );

        debug!(
            task_id = %task_id,
            task_name = %task_name,
            correlation_id = ?correlation_id,
            "Spawning task"
        );

        // Spawn with instrumentation
        let handle = tokio::spawn(future.instrument(span));

        SpawnedTaskHandle {
            id: task_id,
            inner: handle,
            spawner: self.inner.clone(),
        }
    }

    /// Spawn a blocking task.
    pub async fn spawn_blocking<F, T>(&self, name: impl Into<String>, f: F) -> SpawnedTaskHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let task_id = Uuid::new_v4();
        let task_name = name.into();
        let correlation_id = self.correlation_context.read().await.clone();

        let info = TaskInfo {
            id: task_id,
            name: task_name.clone(),
            correlation_id: correlation_id.clone(),
            parent_correlation_id: None,
            state: TaskState::Running,
            spawned_at: chrono::Utc::now(),
            cancellable: false,
        };

        self.inner.register_task(info, None);

        debug!(
            task_id = %task_id,
            task_name = %task_name,
            "Spawning blocking task"
        );

        let handle = tokio::task::spawn_blocking(f);

        SpawnedTaskHandle {
            id: task_id,
            inner: handle,
            spawner: self.inner.clone(),
        }
    }

    /// Spawn a cancellable task with cooperative cancellation support.
    ///
    /// The task receives a `CancellationToken` that it should check periodically.
    /// When cancelled, the token will be triggered and the task can clean up gracefully.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use autohands_runloop::RunLoopSpawner;
    /// use tokio_util::sync::CancellationToken;
    ///
    /// async fn example() {
    ///     let spawner = RunLoopSpawner::new();
    ///
    ///     let handle = spawner.spawn_cancellable("long-task", |token| async move {
    ///         loop {
    ///             tokio::select! {
    ///                 _ = token.cancelled() => {
    ///                     println!("Task cancelled, cleaning up...");
    ///                     break;
    ///                 }
    ///                 _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
    ///                     println!("Working...");
    ///                 }
    ///             }
    ///         }
    ///         "done"
    ///     }).await;
    ///
    ///     // Later: cancel the task
    ///     // spawner.inner().cancel_task(handle.id);
    /// }
    /// ```
    pub async fn spawn_cancellable<F, Fut, T>(
        &self,
        name: impl Into<String>,
        f: F,
    ) -> SpawnedTaskHandle<T>
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let task_id = Uuid::new_v4();
        let task_name = name.into();
        let correlation_id = self.correlation_context.read().await.clone();
        let cancellation_token = CancellationToken::new();

        // Check RunLoop state if provider is available
        if let Some(ref provider) = self.state_provider {
            let run_loop_state = provider.state();
            if run_loop_state == RunLoopState::Stopping || run_loop_state == RunLoopState::Stopped {
                warn!(
                    "Spawning cancellable task '{}' while RunLoop is {:?}",
                    task_name, run_loop_state
                );
            }
        }

        // Create task info
        let info = TaskInfo {
            id: task_id,
            name: task_name.clone(),
            correlation_id: correlation_id.clone(),
            parent_correlation_id: None,
            state: TaskState::Running,
            spawned_at: chrono::Utc::now(),
            cancellable: true,
        };

        self.inner
            .register_task(info, Some(cancellation_token.clone()));

        // Create tracing span
        let span = tracing::info_span!(
            "spawned_cancellable_task",
            task_id = %task_id,
            task_name = %task_name,
            correlation_id = ?correlation_id,
        );

        debug!(
            task_id = %task_id,
            task_name = %task_name,
            correlation_id = ?correlation_id,
            "Spawning cancellable task"
        );

        // Create the future with the cancellation token
        let future = f(cancellation_token);

        // Spawn with instrumentation
        let handle = tokio::spawn(future.instrument(span));

        SpawnedTaskHandle {
            id: task_id,
            inner: handle,
            spawner: self.inner.clone(),
        }
    }

    /// Get current metrics.
    pub fn metrics(&self) -> SpawnerMetrics {
        SpawnerMetrics {
            total_spawned: self.inner.total_spawned.load(Ordering::SeqCst),
            total_completed: self.inner.total_completed.load(Ordering::SeqCst),
            total_cancelled: self.inner.total_cancelled.load(Ordering::SeqCst),
            total_failed: self.inner.total_failed.load(Ordering::SeqCst),
            active_tasks: self.inner.tasks.len(),
        }
    }

    /// Get list of active tasks.
    pub fn active_tasks(&self) -> Vec<TaskInfo> {
        self.inner
            .tasks
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Cancel all cancellable tasks.
    ///
    /// This method:
    /// 1. Triggers the CancellationToken for all cancellable tasks
    /// 2. Marks them as cancelled in our tracking
    ///
    /// Non-cancellable tasks will only be marked as cancelled.
    /// Returns the number of cancellable tasks that were cancelled.
    pub fn cancel_all(&self) -> usize {
        // First cancel all cancellable tasks (with tokens)
        let cancelled_count = self.inner.cancel_all();

        // Then mark remaining non-cancellable tasks as cancelled
        let remaining_ids: Vec<Uuid> = self.inner.tasks.iter().map(|e| *e.key()).collect();
        for id in remaining_ids {
            self.inner.mark_cancelled(id);
        }

        cancelled_count
    }

    /// Cancel a specific task by ID.
    ///
    /// If the task is cancellable (spawned via `spawn_cancellable`), its
    /// CancellationToken will be triggered for cooperative cancellation.
    ///
    /// Returns true if the task was found and cancelled.
    pub fn cancel_task(&self, id: Uuid) -> bool {
        self.inner.cancel_task(id)
    }
}

#[cfg(test)]
#[path = "spawner_tests.rs"]
mod tests;
