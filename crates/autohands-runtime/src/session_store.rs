//! Session persistence and cleanup.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::time::interval;
use tracing::{info, warn};

use crate::session::Session;

#[path = "file_session_store.rs"]
mod file_session_store;
#[path = "memory_session_store.rs"]
mod memory_session_store;

pub use file_session_store::FileSessionStore;
pub use memory_session_store::MemorySessionStore;

#[cfg(test)]
#[path = "session_store_tests.rs"]
mod tests;

/// Session store error.
#[derive(Debug, thiserror::Error)]
pub enum SessionStoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Session not found: {0}")]
    NotFound(String),
}

/// Serializable session data for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub id: String,
    pub created_at: i64,
    pub last_active: i64,
    pub data: std::collections::HashMap<String, serde_json::Value>,
}

impl From<&Session> for PersistedSession {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.clone(),
            created_at: session.created_at.timestamp(),
            last_active: session.last_active.timestamp(),
            data: session.data.clone(),
        }
    }
}

impl From<PersistedSession> for Session {
    fn from(p: PersistedSession) -> Self {
        use chrono::{TimeZone, Utc};
        Self {
            id: p.id,
            created_at: Utc.timestamp_opt(p.created_at, 0).single().unwrap_or_else(Utc::now),
            last_active: Utc.timestamp_opt(p.last_active, 0).single().unwrap_or_else(Utc::now),
            data: p.data,
        }
    }
}

/// Session store trait.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Save a session.
    async fn save(&self, session: &Session) -> Result<(), SessionStoreError>;

    /// Load a session by ID.
    async fn load(&self, id: &str) -> Result<Option<Session>, SessionStoreError>;

    /// Delete a session.
    async fn delete(&self, id: &str) -> Result<(), SessionStoreError>;

    /// List all session IDs.
    async fn list(&self) -> Result<Vec<String>, SessionStoreError>;

    /// Clean up expired sessions.
    async fn cleanup(&self, max_age: Duration) -> Result<usize, SessionStoreError>;
}

/// Session cleanup task.
pub struct SessionCleaner {
    store: std::sync::Arc<dyn SessionStore>,
    max_age: Duration,
    interval: Duration,
}

impl SessionCleaner {
    /// Create a new session cleaner.
    pub fn new(
        store: std::sync::Arc<dyn SessionStore>,
        max_age: Duration,
        cleanup_interval: Duration,
    ) -> Self {
        Self {
            store,
            max_age,
            interval: cleanup_interval,
        }
    }

    /// Start the cleanup task.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = interval(self.interval);

            loop {
                ticker.tick().await;

                match self.store.cleanup(self.max_age).await {
                    Ok(count) if count > 0 => {
                        info!("Cleaned up {} expired sessions", count);
                    }
                    Ok(_) => {}
                    Err(e) => {
                        warn!("Session cleanup error: {}", e);
                    }
                }
            }
        })
    }
}
