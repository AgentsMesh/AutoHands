//! Error types for the RunLoop module.

use thiserror::Error;

use crate::mode::RunLoopMode;

/// Errors that can occur in the RunLoop.
#[derive(Debug, Error)]
pub enum RunLoopError {
    /// RunLoop is already running.
    #[error("RunLoop is already running")]
    AlreadyRunning,

    /// RunLoop is not running.
    #[error("RunLoop is not running")]
    NotRunning,

    /// Invalid state transition.
    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: crate::RunLoopState,
        to: crate::RunLoopState,
    },

    /// Mode not found.
    #[error("Mode not found: {0:?}")]
    ModeNotFound(RunLoopMode),

    /// Source error.
    #[error("Source error: {0}")]
    SourceError(String),

    /// Observer error.
    #[error("Observer error: {0}")]
    ObserverError(String),

    /// Task processing error.
    #[error("Task processing error: {0}")]
    TaskProcessingError(String),

    /// Channel closed.
    #[error("Channel closed")]
    ChannelClosed,

    /// Timeout.
    #[error("Operation timed out")]
    Timeout,

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Agent execution error.
    #[error("Agent execution error: {0}")]
    AgentError(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for RunLoop operations.
pub type RunLoopResult<T> = Result<T, RunLoopError>;

/// Errors related to task chains.
#[derive(Debug, Error)]
pub enum TaskChainError {
    /// Task chain limit exceeded.
    #[error("Task chain {correlation_id} exceeded limit: {count}/{limit}")]
    LimitExceeded {
        correlation_id: String,
        count: u32,
        limit: u32,
    },
}
