//! Checkpoint errors.

use thiserror::Error;

/// Checkpoint error types.
#[derive(Debug, Error)]
pub enum CheckpointError {
    /// Checkpoint not found.
    #[error("Checkpoint not found: {0}")]
    NotFound(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Recovery failed.
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),

    /// Invalid checkpoint data.
    #[error("Invalid checkpoint data: {0}")]
    InvalidData(String),

    /// Generic error.
    #[error("{0}")]
    Custom(String),
}
