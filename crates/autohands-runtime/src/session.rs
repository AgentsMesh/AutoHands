//! Session management.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

/// Session data.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_active: chrono::DateTime<chrono::Utc>,
    pub data: HashMap<String, serde_json::Value>,
}

impl Session {
    pub fn new(id: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: id.into(),
            created_at: now,
            last_active: now,
            data: HashMap::new(),
        }
    }
}

/// Session manager.
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a session.
    pub fn get_or_create(&self, id: &str) -> Session {
        let mut sessions = self.sessions.write();
        sessions
            .entry(id.to_string())
            .or_insert_with(|| Session::new(id))
            .clone()
    }

    /// Get a session by ID.
    pub fn get(&self, id: &str) -> Option<Session> {
        self.sessions.read().get(id).cloned()
    }

    /// Remove a session.
    pub fn remove(&self, id: &str) -> Option<Session> {
        self.sessions.write().remove(id)
    }

    /// Update last active time.
    pub fn touch(&self, id: &str) {
        if let Some(session) = self.sessions.write().get_mut(id) {
            session.last_active = chrono::Utc::now();
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-id");
        assert_eq!(session.id, "test-id");
        assert!(session.data.is_empty());
    }

    #[test]
    fn test_session_manager_creation() {
        let manager = SessionManager::new();
        assert!(manager.get("nonexistent").is_none());
    }

    #[test]
    fn test_session_manager_default() {
        let manager = SessionManager::default();
        assert!(manager.get("nonexistent").is_none());
    }

    #[test]
    fn test_get_or_create() {
        let manager = SessionManager::new();

        // First call creates
        let session1 = manager.get_or_create("test-id");
        assert_eq!(session1.id, "test-id");

        // Second call returns existing
        let session2 = manager.get_or_create("test-id");
        assert_eq!(session2.created_at, session1.created_at);
    }

    #[test]
    fn test_get() {
        let manager = SessionManager::new();

        assert!(manager.get("test-id").is_none());

        manager.get_or_create("test-id");
        assert!(manager.get("test-id").is_some());
    }

    #[test]
    fn test_remove() {
        let manager = SessionManager::new();
        manager.get_or_create("test-id");

        let removed = manager.remove("test-id");
        assert!(removed.is_some());
        assert!(manager.get("test-id").is_none());
    }

    #[test]
    fn test_remove_nonexistent() {
        let manager = SessionManager::new();
        let removed = manager.remove("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn test_touch() {
        let manager = SessionManager::new();
        let session = manager.get_or_create("test-id");
        let original_time = session.last_active;

        std::thread::sleep(std::time::Duration::from_millis(10));
        manager.touch("test-id");

        let updated = manager.get("test-id").unwrap();
        assert!(updated.last_active > original_time);
    }

    #[test]
    fn test_touch_nonexistent() {
        let manager = SessionManager::new();
        // Should not panic
        manager.touch("nonexistent");
    }
}
