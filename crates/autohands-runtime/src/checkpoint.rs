//! Checkpoint support for the agent loop.

use autohands_protocols::types::Message;

/// Checkpoint support trait (optional integration).
#[async_trait::async_trait]
pub trait CheckpointSupport: Send + Sync {
    /// Check if a checkpoint should be created at this turn.
    fn should_checkpoint(&self, turn: u32) -> bool;

    /// Create a checkpoint with the current state.
    async fn create_checkpoint(
        &self,
        session_id: &str,
        turn: u32,
        messages: &[Message],
        context: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get the latest checkpoint for recovery.
    async fn get_latest_checkpoint(
        &self,
        session_id: &str,
    ) -> Result<Option<CheckpointData>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Checkpoint data for recovery.
#[derive(Debug, Clone)]
pub struct CheckpointData {
    /// Turn number when checkpoint was created.
    pub turn: u32,
    /// Serialized messages.
    pub messages: Vec<Message>,
    /// Serialized context.
    pub context: serde_json::Value,
}
