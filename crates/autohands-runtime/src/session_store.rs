//! Session persistence and cleanup.

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::session::Session;

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
            created_at: Utc.timestamp_opt(p.created_at, 0).unwrap(),
            last_active: Utc.timestamp_opt(p.last_active, 0).unwrap(),
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

/// File-based session store.
pub struct FileSessionStore {
    directory: PathBuf,
}

impl FileSessionStore {
    /// Create a new file session store.
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    fn session_path(&self, id: &str) -> PathBuf {
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

use chrono::TimeZone;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_session(id: &str) -> Session {
        Session {
            id: id.to_string(),
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
            data: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_memory_store_save_load() {
        let store = MemorySessionStore::new();
        let session = create_test_session("test-1");

        store.save(&session).await.unwrap();
        let loaded = store.load("test-1").await.unwrap();

        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, "test-1");
    }

    #[tokio::test]
    async fn test_memory_store_delete() {
        let store = MemorySessionStore::new();
        let session = create_test_session("test-1");

        store.save(&session).await.unwrap();
        store.delete("test-1").await.unwrap();

        assert!(store.load("test-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_memory_store_list() {
        let store = MemorySessionStore::new();

        store.save(&create_test_session("a")).await.unwrap();
        store.save(&create_test_session("b")).await.unwrap();

        let ids = store.list().await.unwrap();
        assert_eq!(ids.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_store_cleanup() {
        let store = MemorySessionStore::new();
        let mut old_session = create_test_session("old");
        old_session.last_active = chrono::Utc::now() - chrono::Duration::hours(2);

        store.save(&old_session).await.unwrap();
        store.save(&create_test_session("new")).await.unwrap();

        let cleaned = store.cleanup(Duration::from_secs(3600)).await.unwrap();
        assert_eq!(cleaned, 1);

        let remaining = store.list().await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0], "new");
    }

    #[tokio::test]
    async fn test_file_store_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());
        let session = create_test_session("file-test");

        store.save(&session).await.unwrap();
        let loaded = store.load("file-test").await.unwrap();

        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, "file-test");
    }

    #[tokio::test]
    async fn test_file_store_delete() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        store.save(&create_test_session("to-delete")).await.unwrap();
        store.delete("to-delete").await.unwrap();

        assert!(store.load("to-delete").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_file_store_list() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        store.save(&create_test_session("file-a")).await.unwrap();
        store.save(&create_test_session("file-b")).await.unwrap();

        let ids = store.list().await.unwrap();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_persisted_session_conversion() {
        let session = create_test_session("conv-test");
        let persisted = PersistedSession::from(&session);

        assert_eq!(persisted.id, session.id);

        let restored = Session::from(persisted);
        assert_eq!(restored.id, session.id);
    }

    #[test]
    fn test_session_store_error_display() {
        let err = SessionStoreError::NotFound("test".to_string());
        assert!(err.to_string().contains("Session not found"));
    }

    #[test]
    fn test_memory_store_default() {
        let store = MemorySessionStore::default();
        // Should be empty initially
        let rt = tokio::runtime::Runtime::new().unwrap();
        let ids = rt.block_on(store.list()).unwrap();
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn test_file_store_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        let result = store.load("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_file_store_delete_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        // Should not error
        let result = store.delete("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_file_store_list_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().join("sessions"));

        // Should return empty list for nonexistent directory
        let ids = store.list().await.unwrap();
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn test_file_store_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        let mut old_session = create_test_session("old");
        old_session.last_active = chrono::Utc::now() - chrono::Duration::hours(2);

        store.save(&old_session).await.unwrap();
        store.save(&create_test_session("new")).await.unwrap();

        let cleaned = store.cleanup(Duration::from_secs(3600)).await.unwrap();
        assert_eq!(cleaned, 1);

        let remaining = store.list().await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0], "new");
    }

    #[test]
    fn test_session_cleaner_creation() {
        let store = std::sync::Arc::new(MemorySessionStore::new());
        let _cleaner = SessionCleaner::new(
            store,
            Duration::from_secs(3600),
            Duration::from_secs(60),
        );
    }

    #[tokio::test]
    async fn test_memory_store_load_nonexistent() {
        let store = MemorySessionStore::new();
        let result = store.load("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_memory_store_delete_nonexistent() {
        let store = MemorySessionStore::new();
        // Should not error
        let result = store.delete("nonexistent").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_persisted_session_with_data() {
        let mut session = create_test_session("data-test");
        session.data.insert("key".to_string(), serde_json::json!("value"));

        let persisted = PersistedSession::from(&session);
        assert_eq!(persisted.data.get("key").unwrap(), &serde_json::json!("value"));

        let restored = Session::from(persisted);
        assert_eq!(restored.data.get("key").unwrap(), &serde_json::json!("value"));
    }

    #[test]
    fn test_session_store_error_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = SessionStoreError::from(io_err);
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_session_path() {
        let store = FileSessionStore::new(PathBuf::from("/tmp/sessions"));
        let path = store.session_path("test-id");
        assert_eq!(path, PathBuf::from("/tmp/sessions/test-id.json"));
    }

    #[test]
    fn test_session_store_error_serialization() {
        // Create a real serde_json::Error by parsing invalid JSON
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err = SessionStoreError::Serialization(json_err);
        assert!(err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_session_store_error_debug() {
        let err = SessionStoreError::NotFound("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
    }

    #[tokio::test]
    async fn test_memory_store_overwrite() {
        let store = MemorySessionStore::new();
        let mut session1 = create_test_session("test-id");
        session1.data.insert("key".to_string(), serde_json::json!("value1"));

        store.save(&session1).await.unwrap();

        let mut session2 = create_test_session("test-id");
        session2.data.insert("key".to_string(), serde_json::json!("value2"));

        store.save(&session2).await.unwrap();

        let loaded = store.load("test-id").await.unwrap().unwrap();
        assert_eq!(loaded.data.get("key").unwrap(), &serde_json::json!("value2"));
    }

    #[tokio::test]
    async fn test_memory_store_cleanup_no_expired() {
        let store = MemorySessionStore::new();
        store.save(&create_test_session("a")).await.unwrap();
        store.save(&create_test_session("b")).await.unwrap();

        let cleaned = store.cleanup(Duration::from_secs(3600)).await.unwrap();
        assert_eq!(cleaned, 0);

        let remaining = store.list().await.unwrap();
        assert_eq!(remaining.len(), 2);
    }

    #[tokio::test]
    async fn test_file_store_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        let mut session1 = create_test_session("overwrite-id");
        session1.data.insert("key".to_string(), serde_json::json!("v1"));
        store.save(&session1).await.unwrap();

        let mut session2 = create_test_session("overwrite-id");
        session2.data.insert("key".to_string(), serde_json::json!("v2"));
        store.save(&session2).await.unwrap();

        let loaded = store.load("overwrite-id").await.unwrap().unwrap();
        assert_eq!(loaded.data.get("key").unwrap(), &serde_json::json!("v2"));
    }

    #[test]
    fn test_persisted_session_timestamps() {
        let session = create_test_session("time-test");
        let persisted = PersistedSession::from(&session);

        assert!(persisted.created_at > 0);
        assert!(persisted.last_active > 0);
    }

    #[tokio::test]
    async fn test_file_store_cleanup_no_expired() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileSessionStore::new(temp_dir.path().to_path_buf());

        store.save(&create_test_session("a")).await.unwrap();
        store.save(&create_test_session("b")).await.unwrap();

        let cleaned = store.cleanup(Duration::from_secs(3600)).await.unwrap();
        assert_eq!(cleaned, 0);
    }
}
