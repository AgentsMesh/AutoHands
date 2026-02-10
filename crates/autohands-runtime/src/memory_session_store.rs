//! In-memory session store implementation.

use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::RwLock;

use chrono::TimeZone;

use crate::session::Session;

use super::{PersistedSession, SessionStore, SessionStoreError};

/// In-memory session store.
pub struct MemorySessionStore {
    sessions: RwLock<std::collections::HashMap<String, PersistedSession>>,
}

impl MemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionStore for MemorySessionStore {
    async fn save(&self, session: &Session) -> Result<(), SessionStoreError> {
        let persisted = PersistedSession::from(session);
        self.sessions.write().await.insert(session.id.clone(), persisted);
        Ok(())
    }

    async fn load(&self, id: &str) -> Result<Option<Session>, SessionStoreError> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(id).map(|p| Session::from(p.clone())))
    }

    async fn delete(&self, id: &str) -> Result<(), SessionStoreError> {
        self.sessions.write().await.remove(id);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<String>, SessionStoreError> {
        let sessions = self.sessions.read().await;
        Ok(sessions.keys().cloned().collect())
    }

    async fn cleanup(&self, max_age: Duration) -> Result<usize, SessionStoreError> {
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::from_std(max_age).unwrap_or_default();

        let mut sessions = self.sessions.write().await;
        let to_remove: Vec<_> = sessions
            .iter()
            .filter(|(_, p)| {
                chrono::Utc
                    .timestamp_opt(p.last_active, 0)
                    .single()
                    .map(|t| t < cutoff)
                    .unwrap_or(true)
            })
            .map(|(id, _)| id.clone())
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            sessions.remove(&id);
        }

        Ok(count)
    }
}
