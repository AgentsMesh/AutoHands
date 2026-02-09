//! Checkpoint data structures and manager.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::Arc;

use crate::config::CheckpointConfig;
use crate::error::CheckpointError;
use crate::store::CheckpointStore;

/// A checkpoint of execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique checkpoint ID.
    pub id: Uuid,
    /// Session ID this checkpoint belongs to.
    pub session_id: String,
    /// Turn number when checkpoint was created.
    pub turn: u32,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Serialized conversation messages.
    pub messages: serde_json::Value,
    /// Serialized context/state.
    pub context: serde_json::Value,
    /// Metadata.
    pub metadata: serde_json::Value,
}

impl Checkpoint {
    /// Create a new checkpoint.
    pub fn new(
        session_id: impl Into<String>,
        turn: u32,
        messages: serde_json::Value,
        context: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: session_id.into(),
            turn,
            created_at: Utc::now(),
            messages,
            context,
            metadata: serde_json::Value::Null,
        }
    }

    /// Add metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Checkpoint manager for creating and managing checkpoints.
pub struct CheckpointManager {
    config: CheckpointConfig,
    store: Arc<dyn CheckpointStore>,
}

impl CheckpointManager {
    /// Create a new checkpoint manager.
    pub fn new(config: CheckpointConfig, store: Arc<dyn CheckpointStore>) -> Self {
        Self { config, store }
    }

    /// Check if a checkpoint should be created at this turn.
    pub fn should_checkpoint(&self, turn: u32) -> bool {
        self.config.enabled && turn > 0 && turn % self.config.interval_turns == 0
    }

    /// Create a checkpoint.
    pub async fn create(
        &self,
        session_id: &str,
        turn: u32,
        messages: serde_json::Value,
        context: serde_json::Value,
    ) -> Result<Checkpoint, CheckpointError> {
        let checkpoint = Checkpoint::new(session_id, turn, messages, context);
        self.store.save(&checkpoint).await?;

        // Cleanup old checkpoints
        self.cleanup(session_id).await?;

        Ok(checkpoint)
    }

    /// Get the latest checkpoint for a session.
    pub async fn get_latest(&self, session_id: &str) -> Result<Option<Checkpoint>, CheckpointError> {
        self.store.get_latest(session_id).await
    }

    /// Get a specific checkpoint by ID.
    pub async fn get(&self, id: &Uuid) -> Result<Option<Checkpoint>, CheckpointError> {
        self.store.get(id).await
    }

    /// List checkpoints for a session.
    pub async fn list(&self, session_id: &str) -> Result<Vec<Checkpoint>, CheckpointError> {
        self.store.list(session_id).await
    }

    /// Delete a checkpoint.
    pub async fn delete(&self, id: &Uuid) -> Result<(), CheckpointError> {
        self.store.delete(id).await
    }

    /// Cleanup old checkpoints, keeping only the most recent ones.
    async fn cleanup(&self, session_id: &str) -> Result<(), CheckpointError> {
        let checkpoints = self.store.list(session_id).await?;

        if checkpoints.len() > self.config.max_checkpoints as usize {
            let to_delete = checkpoints.len() - self.config.max_checkpoints as usize;
            for checkpoint in checkpoints.iter().take(to_delete) {
                self.store.delete(&checkpoint.id).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryCheckpointStore;

    #[test]
    fn test_checkpoint_new() {
        let cp = Checkpoint::new("session1", 5, serde_json::json!([]), serde_json::json!({}));
        assert_eq!(cp.session_id, "session1");
        assert_eq!(cp.turn, 5);
    }

    #[test]
    fn test_should_checkpoint() {
        let config = CheckpointConfig {
            enabled: true,
            interval_turns: 5,
            ..Default::default()
        };
        let store = Arc::new(MemoryCheckpointStore::new());
        let manager = CheckpointManager::new(config, store);

        assert!(!manager.should_checkpoint(0));
        assert!(!manager.should_checkpoint(3));
        assert!(manager.should_checkpoint(5));
        assert!(manager.should_checkpoint(10));
    }

    #[tokio::test]
    async fn test_create_checkpoint() {
        let config = CheckpointConfig::default();
        let store = Arc::new(MemoryCheckpointStore::new());
        let manager = CheckpointManager::new(config, store);

        let cp = manager
            .create("session1", 5, serde_json::json!([]), serde_json::json!({}))
            .await
            .unwrap();

        assert_eq!(cp.session_id, "session1");
        assert_eq!(cp.turn, 5);

        let latest = manager.get_latest("session1").await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().id, cp.id);
    }
}
