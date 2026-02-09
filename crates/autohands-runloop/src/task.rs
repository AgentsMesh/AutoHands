//! RunLoop task definitions and task queue.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::config::TaskQueueConfig;
use crate::error::{TaskChainError, RunLoopError, RunLoopResult};

/// Task priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TaskPriority {
    /// Low priority (background tasks).
    Low = 0,
    /// Normal priority.
    Normal = 1,
    /// High priority.
    High = 2,
    /// Critical priority (system tasks).
    Critical = 3,
    /// System priority (shutdown, reload).
    System = 4,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// Task source identification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskSource {
    /// User input (WebSocket, HTTP).
    User,
    /// Scheduler (cron jobs).
    Scheduler,
    /// File watcher trigger.
    FileWatcher,
    /// Webhook trigger.
    Webhook,
    /// WebSocket connection.
    WebSocket,
    /// Agent self-generated task.
    Agent,
    /// System task (shutdown, reload).
    System,
    /// Timer task.
    Timer,
    /// Custom source.
    Custom(String),
}

impl Default for TaskSource {
    fn default() -> Self {
        TaskSource::User
    }
}

/// A RunLoop task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task ID.
    pub id: Uuid,

    /// Task type (e.g., "agent:execute", "scheduler:job:due").
    pub task_type: String,

    /// Task payload.
    pub payload: serde_json::Value,

    /// Task priority.
    pub priority: TaskPriority,

    /// Task source.
    pub source: TaskSource,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Scheduled execution time.
    /// None = immediate execution, Some = delayed execution.
    pub scheduled_at: Option<DateTime<Utc>>,

    /// Correlation ID for task chains.
    pub correlation_id: Option<String>,

    /// Parent task ID (for tracing).
    pub parent_id: Option<Uuid>,

    /// Task metadata.
    pub metadata: HashMap<String, serde_json::Value>,

    /// Retry count.
    pub retry_count: u32,

    /// Maximum retries.
    pub max_retries: u32,
}

impl Task {
    /// Create a new task.
    pub fn new(task_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type: task_type.into(),
            payload,
            priority: TaskPriority::Normal,
            source: TaskSource::User,
            created_at: Utc::now(),
            scheduled_at: None,
            correlation_id: None,
            parent_id: None,
            metadata: HashMap::new(),
            retry_count: 0,
            max_retries: 3,
        }
    }

    /// Set task priority.
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set task source.
    pub fn with_source(mut self, source: TaskSource) -> Self {
        self.source = source;
        self
    }

    /// Set scheduled execution time.
    pub fn with_scheduled_at(mut self, time: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(time);
        self
    }

    /// Set correlation ID.
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Set parent task ID.
    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set max retries.
    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Check if the task is ready to execute.
    pub fn is_ready(&self) -> bool {
        match self.scheduled_at {
            Some(scheduled) => scheduled <= Utc::now(),
            None => true,
        }
    }

    /// Check if the task can be retried.
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Increment retry count.
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// Get or create correlation ID.
    pub fn ensure_correlation_id(&mut self) -> String {
        if self.correlation_id.is_none() {
            self.correlation_id = Some(Uuid::new_v4().to_string());
        }
        self.correlation_id.clone().unwrap()
    }
}

/// Wrapper for priority queue ordering.
#[derive(Clone)]
pub struct PriorityTask(pub Task);

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for PriorityTask {}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then earlier creation time
        match self.0.priority.cmp(&other.0.priority) {
            Ordering::Equal => other.0.created_at.cmp(&self.0.created_at),
            ord => ord,
        }
    }
}

/// Delayed task entry.
#[derive(Clone)]
struct DelayedTask {
    task: Task,
    scheduled_at: DateTime<Utc>,
}

impl PartialEq for DelayedTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.id == other.task.id
    }
}

impl Eq for DelayedTask {}

impl PartialOrd for DelayedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DelayedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Earlier scheduled time has higher priority (reverse for min-heap)
        other.scheduled_at.cmp(&self.scheduled_at)
    }
}

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

/// Task chain tracker.
///
/// Tracks task chains by correlation ID and enforces limits.
pub struct TaskChainTracker {
    /// correlation_id -> task count
    chains: DashMap<String, AtomicU32>,

    /// Maximum tasks per chain.
    max_tasks_per_chain: u32,
}

impl TaskChainTracker {
    /// Create a new chain tracker.
    pub fn new(max_tasks_per_chain: u32) -> Self {
        Self {
            chains: DashMap::new(),
            max_tasks_per_chain,
        }
    }

    /// Try to produce a new task in a chain.
    pub fn try_produce(&self, correlation_id: &str) -> RunLoopResult<()> {
        let count = self
            .chains
            .entry(correlation_id.to_string())
            .or_insert(AtomicU32::new(0));

        let current = count.fetch_add(1, AtomicOrdering::SeqCst);

        if current >= self.max_tasks_per_chain {
            warn!(
                "Task chain {} exceeded limit ({})",
                correlation_id, current
            );
            return Err(RunLoopError::TaskProcessingError(
                TaskChainError::LimitExceeded {
                    correlation_id: correlation_id.to_string(),
                    count: current,
                    limit: self.max_tasks_per_chain,
                }
                .to_string(),
            ));
        }

        Ok(())
    }

    /// Get the current count for a chain.
    pub fn get_count(&self, correlation_id: &str) -> u32 {
        self.chains
            .get(correlation_id)
            .map(|c| c.load(AtomicOrdering::SeqCst))
            .unwrap_or(0)
    }

    /// Clean up old chains (call periodically).
    pub fn cleanup(&self) {
        // Remove chains with 0 count (already completed)
        self.chains.retain(|_, count| count.load(AtomicOrdering::SeqCst) > 0);
    }

    /// Reset a chain (call when chain completes).
    pub fn reset_chain(&self, correlation_id: &str) {
        self.chains.remove(correlation_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_new() {
        let task = Task::new("test:task", serde_json::json!({"key": "value"}));
        assert_eq!(task.task_type, "test:task");
        assert_eq!(task.priority, TaskPriority::Normal);
        assert!(task.is_ready());
    }

    #[test]
    fn test_task_builder() {
        let task = Task::new("test", serde_json::Value::Null)
            .with_priority(TaskPriority::High)
            .with_source(TaskSource::Agent)
            .with_correlation_id("chain-1")
            .with_max_retries(5);

        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.source, TaskSource::Agent);
        assert_eq!(task.correlation_id, Some("chain-1".to_string()));
        assert_eq!(task.max_retries, 5);
    }

    #[test]
    fn test_task_delayed() {
        let future = Utc::now() + chrono::Duration::hours(1);
        let task = Task::new("test", serde_json::Value::Null).with_scheduled_at(future);

        assert!(!task.is_ready());
    }

    #[test]
    fn test_priority_ordering() {
        assert!(TaskPriority::System > TaskPriority::Critical);
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    #[tokio::test]
    async fn test_task_queue_basic() {
        let config = TaskQueueConfig::default();
        let queue = TaskQueue::new(config, 100);

        let task = Task::new("test", serde_json::Value::Null);
        queue.enqueue(task.clone()).await.unwrap();

        assert_eq!(queue.len().await, 1);

        let dequeued = queue.dequeue().await;
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().task_type, "test");
        assert_eq!(queue.len().await, 0);
    }

    #[tokio::test]
    async fn test_task_queue_priority() {
        let config = TaskQueueConfig::default();
        let queue = TaskQueue::new(config, 100);

        let low = Task::new("low", serde_json::Value::Null).with_priority(TaskPriority::Low);
        let high =
            Task::new("high", serde_json::Value::Null).with_priority(TaskPriority::High);
        let normal = Task::new("normal", serde_json::Value::Null);

        queue.enqueue(low).await.unwrap();
        queue.enqueue(normal).await.unwrap();
        queue.enqueue(high).await.unwrap();

        assert_eq!(queue.dequeue().await.unwrap().task_type, "high");
        assert_eq!(queue.dequeue().await.unwrap().task_type, "normal");
        assert_eq!(queue.dequeue().await.unwrap().task_type, "low");
    }

    #[tokio::test]
    async fn test_task_queue_delayed() {
        let config = TaskQueueConfig::default();
        let queue = TaskQueue::new(config, 100);

        let future = Utc::now() + chrono::Duration::hours(1);
        let task =
            Task::new("delayed", serde_json::Value::Null).with_scheduled_at(future);

        queue.enqueue(task).await.unwrap();

        // Should be in delayed queue
        assert_eq!(queue.immediate_len().await, 0);
        assert_eq!(queue.delayed_len().await, 1);

        // Should not be dequeued
        assert!(queue.dequeue().await.is_none());
    }

    #[test]
    fn test_chain_tracker() {
        let tracker = TaskChainTracker::new(3);

        // First 3 should succeed
        assert!(tracker.try_produce("chain-1").is_ok());
        assert!(tracker.try_produce("chain-1").is_ok());
        assert!(tracker.try_produce("chain-1").is_ok());

        // 4th should fail
        assert!(tracker.try_produce("chain-1").is_err());

        // Different chain should work
        assert!(tracker.try_produce("chain-2").is_ok());
    }

    #[test]
    fn test_chain_tracker_reset() {
        let tracker = TaskChainTracker::new(2);

        tracker.try_produce("chain-1").unwrap();
        tracker.try_produce("chain-1").unwrap();
        assert!(tracker.try_produce("chain-1").is_err());

        tracker.reset_chain("chain-1");
        assert!(tracker.try_produce("chain-1").is_ok());
    }

    #[test]
    fn test_task_retry() {
        let mut task = Task::new("test", serde_json::Value::Null).with_max_retries(2);

        assert!(task.can_retry());
        task.increment_retry();
        assert!(task.can_retry());
        task.increment_retry();
        assert!(!task.can_retry());
    }

    #[test]
    fn test_ensure_correlation_id() {
        let mut task = Task::new("test", serde_json::Value::Null);
        assert!(task.correlation_id.is_none());

        let id1 = task.ensure_correlation_id();
        let id2 = task.ensure_correlation_id();
        assert_eq!(id1, id2);
    }
}
