//! RunLoop task definitions.

#[cfg(test)]
#[path = "task_tests.rs"]
mod tests;

use std::cmp::Ordering;
use std::collections::HashMap;

use autohands_protocols::channel::ReplyAddress;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    /// Reply address for routing responses back to the source channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<ReplyAddress>,
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
            reply_to: None,
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

    /// Set reply address for routing responses back to the source channel.
    pub fn with_reply_to(mut self, reply_to: ReplyAddress) -> Self {
        self.reply_to = Some(reply_to);
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
        self.correlation_id
            .get_or_insert_with(|| Uuid::new_v4().to_string())
            .clone()
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
pub(crate) struct DelayedTask {
    pub task: Task,
    pub scheduled_at: DateTime<Utc>,
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
