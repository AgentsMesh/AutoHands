//! Manager for multiple file watcher triggers.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::error;

use super::file_watcher::FileWatcherTrigger;
use super::trigger_types::{FileWatcherConfig, Trigger, TriggerError};

/// Manager for multiple file watcher triggers.
pub struct FileWatcherManager {
    triggers: RwLock<HashMap<String, Arc<FileWatcherTrigger>>>,
}

impl FileWatcherManager {
    /// Create a new file watcher manager.
    pub fn new() -> Self {
        Self {
            triggers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a file watcher trigger.
    pub async fn register(
        &self,
        config: FileWatcherConfig,
    ) -> Result<Arc<FileWatcherTrigger>, TriggerError> {
        let id = config.id.clone();
        let trigger = Arc::new(FileWatcherTrigger::new(config));

        let mut triggers = self.triggers.write().await;
        if triggers.contains_key(&id) {
            return Err(TriggerError::AlreadyExists(id));
        }

        triggers.insert(id, trigger.clone());
        Ok(trigger)
    }

    /// Unregister a file watcher trigger.
    pub async fn unregister(&self, id: &str) -> Result<(), TriggerError> {
        let mut triggers = self.triggers.write().await;

        if let Some(trigger) = triggers.remove(id) {
            trigger.stop().await?;
            Ok(())
        } else {
            Err(TriggerError::NotFound(id.to_string()))
        }
    }

    /// Get a trigger by ID.
    pub async fn get(&self, id: &str) -> Option<Arc<FileWatcherTrigger>> {
        let triggers = self.triggers.read().await;
        triggers.get(id).cloned()
    }

    /// List all triggers.
    pub async fn list(&self) -> Vec<Arc<FileWatcherTrigger>> {
        let triggers = self.triggers.read().await;
        triggers.values().cloned().collect()
    }

    /// Start all triggers.
    pub async fn start_all(&self) -> Result<(), TriggerError> {
        let triggers = self.triggers.read().await;
        for trigger in triggers.values() {
            if let Err(e) = trigger.start().await {
                error!("Failed to start trigger {}: {}", trigger.id(), e);
            }
        }
        Ok(())
    }

    /// Stop all triggers.
    pub async fn stop_all(&self) -> Result<(), TriggerError> {
        let triggers = self.triggers.read().await;
        for trigger in triggers.values() {
            if let Err(e) = trigger.stop().await {
                error!("Failed to stop trigger {}: {}", trigger.id(), e);
            }
        }
        Ok(())
    }
}

impl Default for FileWatcherManager {
    fn default() -> Self {
        Self::new()
    }
}
