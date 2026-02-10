//! Spawner lifecycle observer for RunLoop.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tracing::debug;

use crate::mode::RunLoopPhase;
use crate::spawner::SpawnerInner;
use crate::RunLoop;

use super::RunLoopObserver;

/// Spawner lifecycle observer.
///
/// Manages spawned task lifecycle in coordination with RunLoop:
/// - BeforeWaiting: Log active task count, check for stale tasks
/// - Exit: Cancel all active tasks for graceful shutdown
pub struct SpawnerObserver {
    /// Shared spawner state.
    spawner_inner: Arc<SpawnerInner>,
    /// Task timeout duration (tasks running longer than this are considered stale).
    pub(super) task_timeout: Option<Duration>,
    /// Whether to cancel tasks on Exit.
    pub(super) cancel_on_exit: bool,
}

impl SpawnerObserver {
    /// Create a new spawner observer.
    pub fn new(spawner_inner: Arc<SpawnerInner>) -> Self {
        Self {
            spawner_inner,
            task_timeout: None,
            cancel_on_exit: true,
        }
    }

    /// Set task timeout for stale task detection.
    pub fn with_task_timeout(mut self, timeout: Duration) -> Self {
        self.task_timeout = Some(timeout);
        self
    }

    /// Configure whether to cancel tasks on Exit phase.
    pub fn with_cancel_on_exit(mut self, cancel: bool) -> Self {
        self.cancel_on_exit = cancel;
        self
    }

    /// Check for stale tasks (running longer than timeout).
    fn check_stale_tasks(&self) {
        if let Some(timeout) = self.task_timeout {
            let now = chrono::Utc::now();
            let mut stale_count = 0;

            for entry in self.spawner_inner.tasks.iter() {
                let task = entry.value();
                let duration = now - task.spawned_at;
                if duration > chrono::Duration::from_std(timeout).unwrap_or(chrono::Duration::MAX) {
                    tracing::warn!(
                        task_id = %task.id,
                        task_name = %task.name,
                        duration_secs = duration.num_seconds(),
                        "Stale task detected"
                    );
                    stale_count += 1;
                }
            }

            if stale_count > 0 {
                tracing::warn!(stale_count, "Found stale tasks");
            }
        }
    }

    /// Cancel all active tasks.
    ///
    /// Uses the SpawnerInner's cancel_all method which properly triggers
    /// CancellationTokens for cancellable tasks.
    fn cancel_all_tasks(&self) {
        let cancellable_count = self.spawner_inner.cancellable_count();
        let total_count = self.spawner_inner.tasks.len();

        if total_count > 0 {
            tracing::info!(
                total_count,
                cancellable_count,
                "Cancelling all active tasks on RunLoop exit"
            );

            // Cancel all cancellable tasks (triggers their CancellationTokens)
            let cancelled = self.spawner_inner.cancel_all();

            // Mark remaining non-cancellable tasks as cancelled
            let remaining_ids: Vec<uuid::Uuid> = self
                .spawner_inner
                .tasks
                .iter()
                .map(|e| *e.key())
                .collect();

            for id in remaining_ids {
                if let Some(mut entry) = self.spawner_inner.tasks.get_mut(&id) {
                    entry.state = crate::spawner::TaskState::Cancelled;
                }
                self.spawner_inner
                    .total_cancelled
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                self.spawner_inner.tasks.remove(&id);
            }

            tracing::info!(cancelled, "Cancelled cancellable tasks with tokens");
        }
    }
}

#[async_trait]
impl RunLoopObserver for SpawnerObserver {
    fn activities(&self) -> u32 {
        RunLoopPhase::BeforeWaiting as u32 | RunLoopPhase::Exit as u32
    }

    fn priority(&self) -> i32 {
        50 // Run relatively early
    }

    async fn on_phase(&self, phase: RunLoopPhase, run_loop: &RunLoop) {
        match phase {
            RunLoopPhase::BeforeWaiting => {
                let active_tasks = self.spawner_inner.tasks.len();
                if active_tasks > 0 {
                    debug!(
                        active_tasks,
                        "Spawner status at BeforeWaiting"
                    );
                }

                // Check for stale tasks
                self.check_stale_tasks();

                // Update metrics
                let metrics = run_loop.metrics();
                metrics.set_active_tasks(active_tasks as u64);
            }
            RunLoopPhase::Exit => {
                if self.cancel_on_exit {
                    self.cancel_all_tasks();
                }
            }
            _ => {}
        }
    }
}
