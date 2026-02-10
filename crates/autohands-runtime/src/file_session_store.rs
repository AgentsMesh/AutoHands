//! File-based session store implementation.

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, info};

use crate::session::Session;

use super::{PersistedSession, SessionStore, SessionStoreError};

/// File-based session store.
pub struct FileSessionStore {
    directory: PathBuf,
}

impl FileSessionStore {
    /// Create a new file session store.
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    pub(crate) fn session_path(&self, id: &str) -> PathBuf {
        self.directory.join(format!("{}.json", id))
    }
}

#[async_trait]
impl SessionStore for FileSessionStore {
    async fn save(&self, session: &Session) -> Result<(), SessionStoreError> {
        tokio::fs::create_dir_all(&self.directory).await?;

        let persisted = PersistedSession::from(session);
        let json = serde_json::to_string_pretty(&persisted)?;
        let path = self.session_path(&session.id);

        tokio::fs::write(&path, json).await?;
        debug!("Saved session {} to {:?}", session.id, path);
        Ok(())
    }

    async fn load(&self, id: &str) -> Result<Option<Session>, SessionStoreError> {
        let path = self.session_path(id);

        if !path.exists() {
            return Ok(None);
        }

        let json = tokio::fs::read_to_string(&path).await?;
        let persisted: PersistedSession = serde_json::from_str(&json)?;
        Ok(Some(Session::from(persisted)))
    }

    async fn delete(&self, id: &str) -> Result<(), SessionStoreError> {
        let path = self.session_path(id);

        if path.exists() {
            tokio::fs::remove_file(&path).await?;
            debug!("Deleted session file: {:?}", path);
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<String>, SessionStoreError> {
        if !self.directory.exists() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.directory).await?;

        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(id) = name.strip_suffix(".json") {
                    ids.push(id.to_string());
                }
            }
        }

        Ok(ids)
    }

    async fn cleanup(&self, max_age: Duration) -> Result<usize, SessionStoreError> {
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::from_std(max_age).unwrap_or_default();
        let mut cleaned = 0;

        for id in self.list().await? {
            if let Some(session) = self.load(&id).await? {
                if session.last_active < cutoff {
                    self.delete(&id).await?;
                    cleaned += 1;
                    info!("Cleaned up expired session: {}", id);
                }
            }
        }

        Ok(cleaned)
    }
}
