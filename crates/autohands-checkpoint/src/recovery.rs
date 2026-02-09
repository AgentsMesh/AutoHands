//! Recovery from checkpoints.

use std::sync::Arc;
use tracing::{info, warn};

use crate::checkpoint::{Checkpoint, CheckpointManager};
use crate::config::CheckpointConfig;
use crate::error::CheckpointError;
use crate::store::CheckpointStore;

/// Recovery manager for restoring from checkpoints.
pub struct RecoveryManager {
    checkpoint_manager: CheckpointManager,
    config: CheckpointConfig,
}

impl RecoveryManager {
    /// Create a new recovery manager.
    pub fn new(config: CheckpointConfig, store: Arc<dyn CheckpointStore>) -> Self {
        let checkpoint_manager = CheckpointManager::new(config.clone(), store);
        Self {
            checkpoint_manager,
            config,
        }
    }

    /// Attempt to recover a session from its latest checkpoint.
    pub async fn recover(&self, session_id: &str) -> Result<Option<RecoveryResult>, CheckpointError> {
        if !self.config.auto_recover {
            return Ok(None);
        }

        let checkpoint = self.checkpoint_manager.get_latest(session_id).await?;

        match checkpoint {
            Some(cp) => {
                info!(
                    "Recovering session {} from checkpoint at turn {}",
                    session_id, cp.turn
                );
                Ok(Some(RecoveryResult {
                    checkpoint: cp,
                    recovered_at: chrono::Utc::now(),
                }))
            }
            None => {
                warn!("No checkpoint found for session {}", session_id);
                Ok(None)
            }
        }
    }

    /// List all recoverable sessions.
    pub async fn list_recoverable(&self) -> Result<Vec<String>, CheckpointError> {
        // This would need a store method to list all unique session IDs
        // For now, return empty - full implementation would query the store
        Ok(Vec::new())
    }

    /// Get the checkpoint manager.
    pub fn checkpoint_manager(&self) -> &CheckpointManager {
        &self.checkpoint_manager
    }
}

/// Result of a recovery operation.
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    /// The checkpoint that was recovered.
    pub checkpoint: Checkpoint,
    /// When recovery was performed.
    pub recovered_at: chrono::DateTime<chrono::Utc>,
}

impl RecoveryResult {
    /// Get the messages from the checkpoint.
    pub fn messages(&self) -> &serde_json::Value {
        &self.checkpoint.messages
    }

    /// Get the context from the checkpoint.
    pub fn context(&self) -> &serde_json::Value {
        &self.checkpoint.context
    }

    /// Get the turn number to resume from.
    pub fn resume_turn(&self) -> u32 {
        self.checkpoint.turn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryCheckpointStore;

    #[tokio::test]
    async fn test_recovery_no_checkpoint() {
        let config = CheckpointConfig::default();
        let store = Arc::new(MemoryCheckpointStore::new());
        let manager = RecoveryManager::new(config, store);

        let result = manager.recover("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_recovery_with_checkpoint() {
        let config = CheckpointConfig::default();
        let store = Arc::new(MemoryCheckpointStore::new());
        let manager = RecoveryManager::new(config, store);

        // Create a checkpoint
        manager
            .checkpoint_manager()
            .create("session1", 10, serde_json::json!(["msg1"]), serde_json::json!({"key": "value"}))
            .await
            .unwrap();

        let result = manager.recover("session1").await.unwrap();
        assert!(result.is_some());

        let recovery = result.unwrap();
        assert_eq!(recovery.resume_turn(), 10);
        assert_eq!(recovery.messages(), &serde_json::json!(["msg1"]));
    }

    #[tokio::test]
    async fn test_recovery_disabled() {
        let config = CheckpointConfig {
            auto_recover: false,
            ..Default::default()
        };
        let store = Arc::new(MemoryCheckpointStore::new());
        let manager = RecoveryManager::new(config, store);

        // Even with a checkpoint, recovery should return None
        manager
            .checkpoint_manager()
            .create("session1", 10, serde_json::json!([]), serde_json::json!({}))
            .await
            .unwrap();

        let result = manager.recover("session1").await.unwrap();
        assert!(result.is_none());
    }
}
