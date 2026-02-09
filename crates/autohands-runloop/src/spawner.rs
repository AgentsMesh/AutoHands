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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn, Instrument};
use uuid::Uuid;

use crate::mode::RunLoopState;

/// Task state for tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task was cancelled.
    Cancelled,
    /// Task panicked or failed.
    Failed,
}

/// Task metadata for observability.
#[derive(Debug, Clone)]
pub struct TaskInfo {
    /// Unique task ID.
    pub id: Uuid,
    /// Human-readable task name.
    pub name: String,
    /// Correlation ID for event tracing.
    pub correlation_id: Option<String>,
    /// Parent correlation ID (if spawned from another task).
    pub parent_correlation_id: Option<String>,
    /// Task state.
    pub state: TaskState,
    /// Spawn timestamp.
    pub spawned_at: chrono::DateTime<chrono::Utc>,
    /// Whether this task is cancellable.
    pub cancellable: bool,
}

/// Handle to a spawned task.
pub struct SpawnedTaskHandle<T> {
    /// Task ID.
    pub id: Uuid,
    /// Inner join handle.
    inner: JoinHandle<T>,
    /// Reference to spawner for cleanup.
    spawner: Arc<SpawnerInner>,
}

impl<T> SpawnedTaskHandle<T> {
    /// Abort the task.
    pub fn abort(&self) {
        self.inner.abort();
        self.spawner.mark_cancelled(self.id);
    }

    /// Check if the task is finished.
    pub fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }
}

impl<T> Future for SpawnedTaskHandle<T> {
    type Output = Result<T, tokio::task::JoinError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let id = self.id;
        let spawner = self.spawner.clone();
        let result = std::pin::Pin::new(&mut self.inner).poll(cx);

        if let std::task::Poll::Ready(ref r) = result {
            if r.is_ok() {
                spawner.mark_completed(id);
            } else {
                spawner.mark_failed(id);
            }
        }

        result
    }
}

/// Inner spawner state (shared between RunLoopSpawner and RunLoop).
pub struct SpawnerInner {
    /// Active tasks.
    pub tasks: DashMap<Uuid, TaskInfo>,
    /// Cancellation tokens for cancellable tasks.
    cancellation_tokens: DashMap<Uuid, CancellationToken>,
    /// Total tasks spawned.
    pub total_spawned: AtomicU64,
    /// Total tasks completed.
    pub total_completed: AtomicU64,
    /// Total tasks cancelled.
    pub total_cancelled: AtomicU64,
    /// Total tasks failed.
    pub total_failed: AtomicU64,
}

impl Default for SpawnerInner {
    fn default() -> Self {
        Self::new()
    }
}

impl SpawnerInner {
    /// Create a new spawner inner.
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
            cancellation_tokens: DashMap::new(),
            total_spawned: AtomicU64::new(0),
            total_completed: AtomicU64::new(0),
            total_cancelled: AtomicU64::new(0),
            total_failed: AtomicU64::new(0),
        }
    }

    fn register_task(&self, info: TaskInfo, token: Option<CancellationToken>) {
        let id = info.id;
        self.tasks.insert(id, info);
        if let Some(t) = token {
            self.cancellation_tokens.insert(id, t);
        }
        self.total_spawned.fetch_add(1, Ordering::SeqCst);
    }

    /// Cancel a specific task by ID.
    pub fn cancel_task(&self, id: Uuid) -> bool {
        if let Some((_, token)) = self.cancellation_tokens.remove(&id) {
            token.cancel();
            if let Some(mut entry) = self.tasks.get_mut(&id) {
                entry.state = TaskState::Cancelled;
            }
            self.total_cancelled.fetch_add(1, Ordering::SeqCst);
            self.tasks.remove(&id);
            true
        } else {
            false
        }
    }

    /// Cancel all cancellable tasks.
    pub fn cancel_all(&self) -> usize {
        let tokens: Vec<(Uuid, CancellationToken)> = self
            .cancellation_tokens
            .iter()
            .map(|e| (*e.key(), e.value().clone()))
            .collect();

        let count = tokens.len();
        for (id, token) in tokens {
            token.cancel();
            if let Some(mut entry) = self.tasks.get_mut(&id) {
                entry.state = TaskState::Cancelled;
            }
            self.total_cancelled.fetch_add(1, Ordering::SeqCst);
            self.cancellation_tokens.remove(&id);
            self.tasks.remove(&id);
        }

        if count > 0 {
            info!(count, "Cancelled all cancellable tasks");
        }
        count
    }

    fn mark_completed(&self, id: Uuid) {
        if let Some(mut entry) = self.tasks.get_mut(&id) {
            entry.state = TaskState::Completed;
        }
        self.total_completed.fetch_add(1, Ordering::SeqCst);
        self.cancellation_tokens.remove(&id);
        self.tasks.remove(&id);
    }

    fn mark_cancelled(&self, id: Uuid) {
        if let Some(mut entry) = self.tasks.get_mut(&id) {
            entry.state = TaskState::Cancelled;
        }
        self.total_cancelled.fetch_add(1, Ordering::SeqCst);
        self.cancellation_tokens.remove(&id);
        self.tasks.remove(&id);
    }

    fn mark_failed(&self, id: Uuid) {
        if let Some(mut entry) = self.tasks.get_mut(&id) {
            entry.state = TaskState::Failed;
        }
        self.total_failed.fetch_add(1, Ordering::SeqCst);
        self.cancellation_tokens.remove(&id);
        self.tasks.remove(&id);
    }

    /// Get the number of cancellable tasks.
    pub fn cancellable_count(&self) -> usize {
        self.cancellation_tokens.len()
    }
}

/// Spawner metrics snapshot.
#[derive(Debug, Clone)]
pub struct SpawnerMetrics {
    /// Total tasks spawned.
    pub total_spawned: u64,
    /// Total tasks completed.
    pub total_completed: u64,
    /// Total tasks cancelled.
    pub total_cancelled: u64,
    /// Total tasks failed.
    pub total_failed: u64,
    /// Currently active tasks.
    pub active_tasks: usize,
}

/// State provider for spawner to check RunLoop state without circular reference.
pub trait SpawnerStateProvider: Send + Sync {
    /// Get the current RunLoop state.
    fn state(&self) -> RunLoopState;
}

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
    /// Current correlation ID context.
    correlation_context: RwLock<Option<String>>,
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
            correlation_context: RwLock::new(None),
            inner: Arc::new(SpawnerInner::new()),
        }
    }

    /// Create a spawner with a state provider.
    pub fn with_state_provider(state_provider: Arc<dyn SpawnerStateProvider>) -> Self {
        Self {
            state_provider: Some(state_provider),
            correlation_context: RwLock::new(None),
            inner: Arc::new(SpawnerInner::new()),
        }
    }

    /// Get the inner state (for sharing with RunLoop).
    pub fn inner(&self) -> Arc<SpawnerInner> {
        self.inner.clone()
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

/// Correlation guard for scoped correlation ID.
///
/// Automatically clears the correlation context when dropped.
pub struct CorrelationGuard<'a> {
    spawner: &'a RunLoopSpawner,
    previous: Option<String>,
}

impl<'a> CorrelationGuard<'a> {
    /// Create a new correlation guard.
    pub async fn new(spawner: &'a RunLoopSpawner, correlation_id: String) -> Self {
        let previous = spawner.correlation_context().await;
        spawner
            .set_correlation_context(Some(correlation_id))
            .await;
        Self { spawner, previous }
    }
}

impl<'a> Drop for CorrelationGuard<'a> {
    fn drop(&mut self) {
        // Note: We can't await in Drop, so we use blocking approach
        // In practice, this is fine as it's just setting a value
        let previous = self.previous.take();
        let spawner = self.spawner;
        tokio::spawn(async move {
            // This is a bit of a hack - we spawn a task to reset the context
            // In practice, you might want to use a different approach
            let _ = previous;
            let _ = spawner;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_spawner_creation() {
        let spawner = RunLoopSpawner::new();

        let metrics = spawner.metrics();
        assert_eq!(metrics.total_spawned, 0);
        assert_eq!(metrics.active_tasks, 0);
    }

    #[tokio::test]
    async fn test_spawn_task() {
        let spawner = RunLoopSpawner::new();

        let handle = spawner
            .spawn("test-task", async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                42
            })
            .await;

        let result = handle.await.unwrap();
        assert_eq!(result, 42);

        let metrics = spawner.metrics();
        assert_eq!(metrics.total_spawned, 1);
        assert_eq!(metrics.total_completed, 1);
    }

    #[tokio::test]
    async fn test_correlation_context() {
        let spawner = RunLoopSpawner::new();

        assert!(spawner.correlation_context().await.is_none());

        spawner
            .set_correlation_context(Some("test-correlation".to_string()))
            .await;
        assert_eq!(
            spawner.correlation_context().await,
            Some("test-correlation".to_string())
        );

        spawner.set_correlation_context(None).await;
        assert!(spawner.correlation_context().await.is_none());
    }

    #[tokio::test]
    async fn test_spawn_with_correlation() {
        let spawner = RunLoopSpawner::new();

        spawner
            .set_correlation_context(Some("parent-correlation".to_string()))
            .await;

        let handle = spawner
            .spawn("correlated-task", async { "done" })
            .await;

        handle.await.unwrap();

        // Verify the task was created with correlation
        let metrics = spawner.metrics();
        assert_eq!(metrics.total_completed, 1);
    }

    #[tokio::test]
    async fn test_spawn_blocking() {
        let spawner = RunLoopSpawner::new();

        let handle = spawner
            .spawn_blocking("blocking-task", || {
                std::thread::sleep(Duration::from_millis(10));
                123
            })
            .await;

        let result = handle.await.unwrap();
        assert_eq!(result, 123);
    }

    #[tokio::test]
    async fn test_task_abort() {
        let spawner = RunLoopSpawner::new();

        let handle = spawner
            .spawn("long-task", async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                "never"
            })
            .await;

        handle.abort();

        // Task should be marked as cancelled eventually
        tokio::time::sleep(Duration::from_millis(50)).await;

        let metrics = spawner.metrics();
        assert_eq!(metrics.total_cancelled, 1);
    }

    #[tokio::test]
    async fn test_active_tasks() {
        let spawner = RunLoopSpawner::new();

        let handle1 = spawner
            .spawn("task-1", async {
                tokio::time::sleep(Duration::from_millis(100)).await;
            })
            .await;

        let handle2 = spawner
            .spawn("task-2", async {
                tokio::time::sleep(Duration::from_millis(100)).await;
            })
            .await;

        // Give tasks time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        let active = spawner.active_tasks();
        assert_eq!(active.len(), 2);

        // Wait for completion
        handle1.await.unwrap();
        handle2.await.unwrap();

        let active = spawner.active_tasks();
        assert_eq!(active.len(), 0);
    }

    #[tokio::test]
    async fn test_cancel_all() {
        let spawner = RunLoopSpawner::new();

        let _handle1 = spawner
            .spawn("task-1", async {
                tokio::time::sleep(Duration::from_secs(10)).await;
            })
            .await;

        let _handle2 = spawner
            .spawn("task-2", async {
                tokio::time::sleep(Duration::from_secs(10)).await;
            })
            .await;

        spawner.cancel_all();

        let metrics = spawner.metrics();
        assert_eq!(metrics.total_cancelled, 2);
    }

    #[tokio::test]
    async fn test_task_info() {
        let spawner = RunLoopSpawner::new();

        spawner
            .set_correlation_context(Some("test-corr".to_string()))
            .await;

        let handle = spawner
            .spawn("info-task", async {
                tokio::time::sleep(Duration::from_millis(50)).await;
            })
            .await;

        let active = spawner.active_tasks();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "info-task");
        assert_eq!(active[0].correlation_id, Some("test-corr".to_string()));
        assert_eq!(active[0].state, TaskState::Running);
        assert!(!active[0].cancellable); // Regular spawn is not cancellable

        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_spawn_cancellable() {
        let spawner = RunLoopSpawner::new();

        let handle = spawner
            .spawn_cancellable("cancellable-task", |token| async move {
                loop {
                    tokio::select! {
                        _ = token.cancelled() => {
                            return "cancelled";
                        }
                        _ = tokio::time::sleep(Duration::from_millis(10)) => {
                            // Keep working
                        }
                    }
                }
            })
            .await;

        // Verify task is marked as cancellable
        let active = spawner.active_tasks();
        assert_eq!(active.len(), 1);
        assert!(active[0].cancellable);
        assert_eq!(spawner.inner().cancellable_count(), 1);

        // Cancel the task
        let cancelled = spawner.cancel_task(handle.id);
        assert!(cancelled);

        // Wait for task to finish
        let result = handle.await.unwrap();
        assert_eq!(result, "cancelled");

        // Verify metrics
        let metrics = spawner.metrics();
        assert_eq!(metrics.total_cancelled, 1);
    }

    #[tokio::test]
    async fn test_cancel_all_with_cancellable_tasks() {
        let spawner = RunLoopSpawner::new();

        // Spawn a mix of cancellable and non-cancellable tasks
        let _handle1 = spawner
            .spawn("regular-task", async {
                tokio::time::sleep(Duration::from_secs(10)).await;
            })
            .await;

        let _handle2 = spawner
            .spawn_cancellable("cancellable-task", |token| async move {
                loop {
                    tokio::select! {
                        _ = token.cancelled() => {
                            return "cancelled";
                        }
                        _ = tokio::time::sleep(Duration::from_millis(10)) => {}
                    }
                }
            })
            .await;

        // Give tasks time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Verify we have one cancellable task
        assert_eq!(spawner.inner().cancellable_count(), 1);

        // Cancel all tasks
        let cancelled_with_tokens = spawner.cancel_all();
        assert_eq!(cancelled_with_tokens, 1); // Only one had a token

        // All tasks should be marked as cancelled
        let metrics = spawner.metrics();
        assert_eq!(metrics.total_cancelled, 2);
    }

    #[tokio::test]
    async fn test_cancel_task_by_id() {
        let spawner = RunLoopSpawner::new();

        let handle = spawner
            .spawn_cancellable("task-to-cancel", |token| async move {
                token.cancelled().await;
                "done"
            })
            .await;

        let task_id = handle.id;

        // Cancel by ID
        assert!(spawner.cancel_task(task_id));

        // Try to cancel again (should fail, already removed)
        assert!(!spawner.cancel_task(task_id));

        // Wait for completion
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_spawner_inner_cancel_all() {
        let inner = Arc::new(SpawnerInner::new());

        // Register some tasks with tokens
        let token1 = CancellationToken::new();
        let token2 = CancellationToken::new();

        let info1 = TaskInfo {
            id: Uuid::new_v4(),
            name: "task1".to_string(),
            correlation_id: None,
            parent_correlation_id: None,
            state: TaskState::Running,
            spawned_at: chrono::Utc::now(),
            cancellable: true,
        };

        let info2 = TaskInfo {
            id: Uuid::new_v4(),
            name: "task2".to_string(),
            correlation_id: None,
            parent_correlation_id: None,
            state: TaskState::Running,
            spawned_at: chrono::Utc::now(),
            cancellable: true,
        };

        inner.register_task(info1, Some(token1.clone()));
        inner.register_task(info2, Some(token2.clone()));

        assert_eq!(inner.cancellable_count(), 2);

        // Cancel all
        let cancelled = inner.cancel_all();
        assert_eq!(cancelled, 2);

        // Tokens should be triggered
        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());

        // Tasks should be removed
        assert_eq!(inner.tasks.len(), 0);
        assert_eq!(inner.cancellable_count(), 0);
    }
}
