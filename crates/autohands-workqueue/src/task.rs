//! Task definition and status.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    /// Low priority.
    Low = 0,
    /// Normal priority.
    Normal = 1,
    /// High priority.
    High = 2,
    /// Critical priority.
    Critical = 3,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// Task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Waiting in queue.
    Pending,
    /// Currently being processed.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed (may be retried).
    Failed,
    /// Moved to dead letter queue.
    DeadLetter,
    /// Cancelled by user.
    Cancelled,
}

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::Pending
    }
}

/// A task in the queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task ID.
    pub id: Uuid,
    /// Task name/type.
    pub name: String,
    /// Agent to execute the task.
    pub agent: String,
    /// Task payload/prompt.
    pub payload: String,
    /// Task priority.
    pub priority: TaskPriority,
    /// Current status.
    pub status: TaskStatus,
    /// Creation time.
    pub created_at: DateTime<Utc>,
    /// Last update time.
    pub updated_at: DateTime<Utc>,
    /// Scheduled execution time (None = immediate).
    pub scheduled_at: Option<DateTime<Utc>>,
    /// Number of retry attempts.
    pub retry_count: u32,
    /// Maximum retries allowed.
    pub max_retries: u32,
    /// Last error message.
    pub last_error: Option<String>,
    /// Metadata.
    pub metadata: serde_json::Value,
}

impl Task {
    /// Create a new task.
    pub fn new(name: impl Into<String>, agent: impl Into<String>, payload: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            agent: agent.into(),
            payload: payload.into(),
            priority: TaskPriority::Normal,
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            scheduled_at: None,
            retry_count: 0,
            max_retries: 3,
            last_error: None,
            metadata: serde_json::Value::Null,
        }
    }

    /// Set task priority.
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set scheduled execution time.
    pub fn with_scheduled_at(mut self, time: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(time);
        self
    }

    /// Set maximum retries.
    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Check if task can be retried.
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Check if task is ready to run.
    pub fn is_ready(&self) -> bool {
        if self.status != TaskStatus::Pending {
            return false;
        }

        match self.scheduled_at {
            Some(scheduled) => scheduled <= Utc::now(),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_new() {
        let task = Task::new("test", "general", "test payload");
        assert_eq!(task.name, "test");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, TaskPriority::Normal);
    }

    #[test]
    fn test_task_priority_order() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    #[test]
    fn test_can_retry() {
        let mut task = Task::new("test", "general", "test");
        task.max_retries = 3;

        assert!(task.can_retry());
        task.retry_count = 3;
        assert!(!task.can_retry());
    }

    #[test]
    fn test_is_ready() {
        let task = Task::new("test", "general", "test");
        assert!(task.is_ready());

        let future_task = Task::new("test", "general", "test")
            .with_scheduled_at(Utc::now() + chrono::Duration::hours(1));
        assert!(!future_task.is_ready());
    }
}
