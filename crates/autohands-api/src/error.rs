//! Interface error types.

use thiserror::Error;

/// Interface error types.
#[derive(Debug, Error)]
pub enum InterfaceError {
    /// Workflow not found.
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),

    /// Agent not found.
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Step execution failed.
    #[error("Step execution failed: {step} - {reason}")]
    StepFailed { step: String, reason: String },

    /// Workflow already running.
    #[error("Workflow already running: {0}")]
    AlreadyRunning(String),

    /// Invalid workflow definition.
    #[error("Invalid workflow definition: {0}")]
    InvalidWorkflow(String),

    /// Handoff failed.
    #[error("Handoff failed: {0}")]
    HandoffFailed(String),

    /// Timeout.
    #[error("Timeout")]
    Timeout,

    /// RunLoop injection failed.
    #[error("Failed to inject event into RunLoop: {0}")]
    RunLoopInjectionFailed(String),

    /// WebSocket error.
    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    /// Webhook error.
    #[error("Webhook error: {0}")]
    WebhookError(String),

    /// Generic error.
    #[error("{0}")]
    Custom(String),
}
