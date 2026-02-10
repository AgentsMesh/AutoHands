//! Checkpoint integration with RunLoop.
//!
//! Provides an Observer that creates checkpoints at BeforeWaiting phase,
//! similar to CATransaction commit in iOS.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::mode::{RunLoopMode, RunLoopPhase};
use crate::observer::RunLoopObserver;
use crate::RunLoop;

/// RunLoop checkpoint data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLoopCheckpoint {
    /// Checkpoint ID.
    pub id: Uuid,

    /// Current mode.
    pub mode: RunLoopMode,

    /// Pending event count.
    pub pending_events: usize,

    /// Metrics snapshot.
    pub metrics: CheckpointMetrics,

    /// Timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Metrics included in checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetrics {
    pub iterations: u64,
    pub events_processed: u64,
    pub events_enqueued: u64,
    pub wakeups: u64,
    pub uptime_secs: u64,
}

/// Checkpoint manager trait.
///
/// Implement this to persist checkpoints.
#[async_trait]
pub trait CheckpointManager: Send + Sync {
    /// Save a RunLoop checkpoint.
    async fn save_runloop_checkpoint(&self, checkpoint: &RunLoopCheckpoint) -> Result<(), CheckpointError>;

    /// Load the latest RunLoop checkpoint.
    async fn load_latest_checkpoint(&self) -> Result<Option<RunLoopCheckpoint>, CheckpointError>;

    /// List all checkpoints.
    async fn list_checkpoints(&self) -> Result<Vec<Uuid>, CheckpointError>;

    /// Delete a checkpoint.
    async fn delete_checkpoint(&self, id: &Uuid) -> Result<(), CheckpointError>;
}

/// Checkpoint error.
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Checkpoint not found: {0}")]
    NotFound(Uuid),
}

/// In-memory checkpoint manager for testing.
pub struct MemoryCheckpointManager {
    checkpoints: parking_lot::RwLock<Vec<RunLoopCheckpoint>>,
    max_checkpoints: usize,
}

impl MemoryCheckpointManager {
    /// Create a new memory checkpoint manager.
    pub fn new(max_checkpoints: usize) -> Self {
        Self {
            checkpoints: parking_lot::RwLock::new(Vec::new()),
            max_checkpoints,
        }
    }

    /// Get the number of stored checkpoints.
    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.read().len()
    }
}

impl Default for MemoryCheckpointManager {
    fn default() -> Self {
        Self::new(10)
    }
}

#[async_trait]
impl CheckpointManager for MemoryCheckpointManager {
    async fn save_runloop_checkpoint(&self, checkpoint: &RunLoopCheckpoint) -> Result<(), CheckpointError> {
        let mut checkpoints = self.checkpoints.write();

        // Remove old checkpoints if at capacity
        while checkpoints.len() >= self.max_checkpoints {
            checkpoints.remove(0);
        }

        checkpoints.push(checkpoint.clone());
        Ok(())
    }

    async fn load_latest_checkpoint(&self) -> Result<Option<RunLoopCheckpoint>, CheckpointError> {
        let checkpoints = self.checkpoints.read();
        Ok(checkpoints.last().cloned())
    }

    async fn list_checkpoints(&self) -> Result<Vec<Uuid>, CheckpointError> {
        let checkpoints = self.checkpoints.read();
        Ok(checkpoints.iter().map(|c| c.id).collect())
    }

    async fn delete_checkpoint(&self, id: &Uuid) -> Result<(), CheckpointError> {
        let mut checkpoints = self.checkpoints.write();
        let len_before = checkpoints.len();
        checkpoints.retain(|c| c.id != *id);

        if checkpoints.len() == len_before {
            return Err(CheckpointError::NotFound(*id));
        }

        Ok(())
    }
}

/// Checkpoint Observer.
///
/// Creates checkpoints at BeforeWaiting phase.
pub struct CheckpointObserver {
    manager: Arc<dyn CheckpointManager>,
    /// Minimum interval between checkpoints (seconds).
    min_interval_secs: u64,
    /// Last checkpoint time.
    last_checkpoint: parking_lot::RwLock<Option<DateTime<Utc>>>,
}

impl CheckpointObserver {
    /// Create a new checkpoint observer.
    pub fn new(manager: Arc<dyn CheckpointManager>) -> Self {
        Self {
            manager,
            min_interval_secs: 60, // Default: 1 minute
            last_checkpoint: parking_lot::RwLock::new(None),
        }
    }

    /// Set minimum interval between checkpoints.
    pub fn with_interval(mut self, interval_secs: u64) -> Self {
        self.min_interval_secs = interval_secs;
        self
    }

    /// Check if enough time has passed since last checkpoint.
    fn should_checkpoint(&self) -> bool {
        let last = self.last_checkpoint.read();
        match *last {
            Some(time) => {
                let elapsed = (Utc::now() - time).num_seconds();
                elapsed >= self.min_interval_secs as i64
            }
            None => true,
        }
    }

    /// Update last checkpoint time.
    fn mark_checkpointed(&self) {
        *self.last_checkpoint.write() = Some(Utc::now());
    }
}

#[async_trait]
impl RunLoopObserver for CheckpointObserver {
    fn activities(&self) -> u32 {
        RunLoopPhase::BeforeWaiting as u32
    }

    fn priority(&self) -> i32 {
        -50 // Run after most observers but before metrics
    }

    async fn on_phase(&self, _phase: RunLoopPhase, run_loop: &RunLoop) {
        if !self.should_checkpoint() {
            return;
        }

        let metrics = run_loop.metrics();
        let snapshot = metrics.snapshot();

        let checkpoint = RunLoopCheckpoint {
            id: Uuid::new_v4(),
            mode: run_loop.current_mode().await,
            pending_events: run_loop.pending_task_count().await,
            metrics: CheckpointMetrics {
                iterations: snapshot.iterations,
                events_processed: snapshot.events_processed,
                events_enqueued: snapshot.events_enqueued,
                wakeups: snapshot.wakeups,
                uptime_secs: snapshot.uptime_secs,
            },
            timestamp: Utc::now(),
        };

        debug!("Creating checkpoint: {}", checkpoint.id);

        if let Err(e) = self.manager.save_runloop_checkpoint(&checkpoint).await {
            warn!("Failed to save RunLoop checkpoint: {}", e);
        } else {
            self.mark_checkpointed();
        }
    }
}

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod tests;
