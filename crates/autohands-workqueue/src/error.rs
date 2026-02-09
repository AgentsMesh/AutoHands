//! Queue errors.

use thiserror::Error;

/// Queue error types.
#[derive(Debug, Error)]
pub enum QueueError {
    /// Task not found.
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    /// Queue is full.
    #[error("Queue is full")]
    QueueFull,

    /// Queue is empty.
    #[error("Queue is empty")]
    QueueEmpty,

    /// Worker error.
    #[error("Worker error: {0}")]
    WorkerError(String),

    /// Database error.
    #[error("Database error: {0}")]
    Database(String),

    /// Task execution failed.
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    /// Generic error.
    #[error("{0}")]
    Custom(String),
}
