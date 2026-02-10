//! Task queue with priority and delayed task support.

use std::collections::BinaryHeap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::debug;

use crate::config::TaskQueueConfig;
use crate::error::{RunLoopError, RunLoopResult};
use crate::task::{DelayedTask, PriorityTask, Task};
use crate::task_chain::TaskChainTracker;

/// Task queue with priority and delayed task support.
pub struct TaskQueue {
    /// Configuration.
    config: TaskQueueConfig,

    /// Immediate execution queue (priority sorted).
    immediate: RwLock<BinaryHeap<PriorityTask>>,

    /// Delayed tasks queue (by scheduled time).
    delayed: RwLock<BinaryHeap<DelayedTask>>,

    /// Task chain tracker.
    chain_tracker: Arc<TaskChainTracker>,
}

impl TaskQueue {
    /// Create a new task queue.
    pub fn new(config: TaskQueueConfig, max_tasks_per_chain: u32) -> Self {
        Self {
            config,
            immediate: RwLock::new(BinaryHeap::new()),
            delayed: RwLock::new(BinaryHeap::new()),
            chain_tracker: Arc::new(TaskChainTracker::new(max_tasks_per_chain)),
        }
    }

    /// Enqueue a task.
    pub async fn enqueue(&self, task: Task) -> RunLoopResult<()> {
        // Check chain limit if correlation ID exists
        if let Some(ref correlation_id) = task.correlation_id {
            self.chain_tracker.try_produce(correlation_id)?;
        }

        // Check queue size limit
        let immediate_len = self.immediate.read().await.len();
        let delayed_len = self.delayed.read().await.len();
        if immediate_len + delayed_len >= self.config.max_pending_tasks {
            return Err(RunLoopError::TaskProcessingError(
                "Task queue is full".to_string(),
            ));
        }

        // Route to appropriate queue
        if let Some(scheduled_at) = task.scheduled_at {
            if scheduled_at > Utc::now() {
                debug!(
                    "Task {} scheduled for {}",
                    task.id,
                    scheduled_at.to_rfc3339()
                );
                let mut delayed = self.delayed.write().await;
                delayed.push(DelayedTask {
                    task,
                    scheduled_at,
                });
                return Ok(());
            }
        }

        debug!(
            "Task {} enqueued (priority: {:?})",
            task.id, task.priority
        );
        let mut immediate = self.immediate.write().await;
        immediate.push(PriorityTask(task));

        Ok(())
    }

    /// Dequeue the highest priority ready task.
    pub async fn dequeue(&self) -> Option<Task> {
        let mut immediate = self.immediate.write().await;
        immediate.pop().map(|pt| {
            debug!("Task {} dequeued", pt.0.id);
            pt.0
        })
    }

    /// Promote delayed tasks that are now due.
    pub async fn promote_delayed(&self) {
        let now = Utc::now();
        let mut delayed = self.delayed.write().await;
        let mut immediate = self.immediate.write().await;

        while let Some(entry) = delayed.peek() {
            if entry.scheduled_at <= now {
                let entry = delayed.pop().unwrap();
                debug!(
                    "Promoting delayed task {} (scheduled: {})",
                    entry.task.id,
                    entry.scheduled_at.to_rfc3339()
                );
                immediate.push(PriorityTask(entry.task));
            } else {
                break;
            }
        }
    }

    /// Get the next delayed task's scheduled time.
    pub async fn next_delayed_time(&self) -> Option<DateTime<Utc>> {
        self.delayed.read().await.peek().map(|e| e.scheduled_at)
    }

    /// Get immediate queue length.
    pub async fn immediate_len(&self) -> usize {
        self.immediate.read().await.len()
    }

    /// Get delayed queue length.
    pub async fn delayed_len(&self) -> usize {
        self.delayed.read().await.len()
    }

    /// Get total queue length.
    pub async fn len(&self) -> usize {
        self.immediate_len().await + self.delayed_len().await
    }

    /// Check if queues are empty.
    pub async fn is_empty(&self) -> bool {
        self.immediate.read().await.is_empty() && self.delayed.read().await.is_empty()
    }

    /// Get the chain tracker.
    pub fn chain_tracker(&self) -> &Arc<TaskChainTracker> {
        &self.chain_tracker
    }

    /// Clear all tasks.
    pub async fn clear(&self) {
        self.immediate.write().await.clear();
        self.delayed.write().await.clear();
    }
}
