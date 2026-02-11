//! Task chain tracker for enforcing chain limits.

use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

use dashmap::DashMap;
use tracing::warn;

use crate::error::{RunLoopError, RunLoopResult, TaskChainError};

/// Task chain tracker.
///
/// Tracks task chains by correlation ID and enforces limits.
pub struct TaskChainTracker {
    /// correlation_id -> task count
    chains: DashMap<String, AtomicU32>,

    /// Maximum tasks per chain.
    max_tasks_per_chain: u32,
}

impl TaskChainTracker {
    /// Create a new chain tracker.
    pub fn new(max_tasks_per_chain: u32) -> Self {
        Self {
            chains: DashMap::new(),
            max_tasks_per_chain,
        }
    }

    /// Try to produce a new task in a chain.
    pub fn try_produce(&self, correlation_id: &str) -> RunLoopResult<()> {
        let count = self
            .chains
            .entry(correlation_id.to_string())
            .or_insert(AtomicU32::new(0));

        let current = count.fetch_add(1, AtomicOrdering::SeqCst);

        if current >= self.max_tasks_per_chain {
            count.fetch_sub(1, AtomicOrdering::SeqCst);
            warn!(
                "Task chain {} exceeded limit ({})",
                correlation_id, current
            );
            return Err(RunLoopError::TaskProcessingError(
                TaskChainError::LimitExceeded {
                    correlation_id: correlation_id.to_string(),
                    count: current,
                    limit: self.max_tasks_per_chain,
                }
                .to_string(),
            ));
        }

        Ok(())
    }

    /// Get the current count for a chain.
    pub fn get_count(&self, correlation_id: &str) -> u32 {
        self.chains
            .get(correlation_id)
            .map(|c| c.load(AtomicOrdering::SeqCst))
            .unwrap_or(0)
    }

    /// Clean up old chains (call periodically).
    pub fn cleanup(&self) {
        // Remove chains with 0 count (already completed)
        self.chains
            .retain(|_, count| count.load(AtomicOrdering::SeqCst) > 0);
    }

    /// Reset a chain (call when chain completes).
    pub fn reset_chain(&self, correlation_id: &str) {
        self.chains.remove(correlation_id);
    }
}
