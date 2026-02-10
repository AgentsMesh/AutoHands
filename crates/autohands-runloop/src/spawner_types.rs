//! Spawner types for unified async task management.
//!
//! This module contains the core types used by [`RunLoopSpawner`](crate::spawner::RunLoopSpawner):
//! - [`TaskState`] / [`TaskInfo`]: Task metadata for observability
//! - [`SpawnedTaskHandle`]: Handle to a spawned task (implements `Future`)
//! - [`SpawnerInner`]: Shared inner state for task tracking and cancellation
//! - [`SpawnerMetrics`]: Metrics snapshot
//! - [`SpawnerStateProvider`]: Trait for RunLoop state awareness

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::info;

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
    pub id: uuid::Uuid,
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
    pub id: uuid::Uuid,
    /// Inner join handle.
    pub(crate) inner: JoinHandle<T>,
    /// Reference to spawner for cleanup.
    pub(crate) spawner: Arc<SpawnerInner>,
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
    pub tasks: DashMap<uuid::Uuid, TaskInfo>,
    /// Cancellation tokens for cancellable tasks.
    cancellation_tokens: DashMap<uuid::Uuid, CancellationToken>,
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

    pub(crate) fn register_task(&self, info: TaskInfo, token: Option<CancellationToken>) {
        let id = info.id;
        self.tasks.insert(id, info);
        if let Some(t) = token {
            self.cancellation_tokens.insert(id, t);
        }
        self.total_spawned.fetch_add(1, Ordering::SeqCst);
    }

    /// Cancel a specific task by ID.
    pub fn cancel_task(&self, id: uuid::Uuid) -> bool {
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
        let tokens: Vec<(uuid::Uuid, CancellationToken)> = self
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

    pub(crate) fn mark_completed(&self, id: uuid::Uuid) {
        if let Some(mut entry) = self.tasks.get_mut(&id) {
            entry.state = TaskState::Completed;
        }
        self.total_completed.fetch_add(1, Ordering::SeqCst);
        self.cancellation_tokens.remove(&id);
        self.tasks.remove(&id);
    }

    pub(crate) fn mark_cancelled(&self, id: uuid::Uuid) {
        if let Some(mut entry) = self.tasks.get_mut(&id) {
            entry.state = TaskState::Cancelled;
        }
        self.total_cancelled.fetch_add(1, Ordering::SeqCst);
        self.cancellation_tokens.remove(&id);
        self.tasks.remove(&id);
    }

    pub(crate) fn mark_failed(&self, id: uuid::Uuid) {
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
