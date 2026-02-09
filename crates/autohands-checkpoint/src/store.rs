//! Checkpoint storage.

use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::checkpoint::Checkpoint;
use crate::error::CheckpointError;

/// Checkpoint storage trait.
#[async_trait]
pub trait CheckpointStore: Send + Sync {
    /// Save a checkpoint.
    async fn save(&self, checkpoint: &Checkpoint) -> Result<(), CheckpointError>;

    /// Get a checkpoint by ID.
    async fn get(&self, id: &Uuid) -> Result<Option<Checkpoint>, CheckpointError>;

    /// Get the latest checkpoint for a session.
    async fn get_latest(&self, session_id: &str) -> Result<Option<Checkpoint>, CheckpointError>;

    /// List all checkpoints for a session (ordered by turn, oldest first).
    async fn list(&self, session_id: &str) -> Result<Vec<Checkpoint>, CheckpointError>;

    /// Delete a checkpoint.
    async fn delete(&self, id: &Uuid) -> Result<(), CheckpointError>;

    /// Delete all checkpoints for a session.
    async fn delete_session(&self, session_id: &str) -> Result<(), CheckpointError>;
}

/// In-memory checkpoint store for testing.
pub struct MemoryCheckpointStore {
    checkpoints: tokio::sync::RwLock<std::collections::HashMap<Uuid, Checkpoint>>,
}

impl MemoryCheckpointStore {
    /// Create a new memory store.
    pub fn new() -> Self {
        Self {
            checkpoints: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MemoryCheckpointStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CheckpointStore for MemoryCheckpointStore {
    async fn save(&self, checkpoint: &Checkpoint) -> Result<(), CheckpointError> {
        let mut store = self.checkpoints.write().await;
        store.insert(checkpoint.id, checkpoint.clone());
        Ok(())
    }

    async fn get(&self, id: &Uuid) -> Result<Option<Checkpoint>, CheckpointError> {
        let store = self.checkpoints.read().await;
        Ok(store.get(id).cloned())
    }

    async fn get_latest(&self, session_id: &str) -> Result<Option<Checkpoint>, CheckpointError> {
        let store = self.checkpoints.read().await;
        let latest = store
            .values()
            .filter(|cp| cp.session_id == session_id)
            .max_by_key(|cp| cp.turn)
            .cloned();
        Ok(latest)
    }

    async fn list(&self, session_id: &str) -> Result<Vec<Checkpoint>, CheckpointError> {
        let store = self.checkpoints.read().await;
        let mut checkpoints: Vec<_> = store
            .values()
            .filter(|cp| cp.session_id == session_id)
            .cloned()
            .collect();
        checkpoints.sort_by_key(|cp| cp.turn);
        Ok(checkpoints)
    }

    async fn delete(&self, id: &Uuid) -> Result<(), CheckpointError> {
        let mut store = self.checkpoints.write().await;
        store.remove(id);
        Ok(())
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), CheckpointError> {
        let mut store = self.checkpoints.write().await;
        store.retain(|_, cp| cp.session_id != session_id);
        Ok(())
    }
}

/// File system based checkpoint store for persistence.
///
/// Checkpoints are stored as individual JSON files organized by session:
/// ```text
/// {storage_path}/
/// └── checkpoints/
///     └── {session_id}/
///         ├── {uuid}_turn_{turn}.json
///         ├── {uuid}_turn_{turn}.json
///         └── ...
/// ```
pub struct FileCheckpointStore {
    /// Base storage path.
    storage_path: PathBuf,
}

impl FileCheckpointStore {
    /// Create a new file-based checkpoint store.
    ///
    /// # Arguments
    /// * `storage_path` - Base directory for storing checkpoint files
    pub async fn new(storage_path: impl Into<PathBuf>) -> Result<Self, CheckpointError> {
        let storage_path = storage_path.into();
        let checkpoints_dir = storage_path.join("checkpoints");

        // Ensure base directory exists
        fs::create_dir_all(&checkpoints_dir).await?;

        debug!("FileCheckpointStore initialized at {:?}", storage_path);

        Ok(Self { storage_path })
    }

    /// Get the checkpoints directory path.
    fn checkpoints_dir(&self) -> PathBuf {
        self.storage_path.join("checkpoints")
    }

    /// Get the session directory path.
    fn session_dir(&self, session_id: &str) -> PathBuf {
        let sanitized = Self::sanitize_session_id(session_id);
        self.checkpoints_dir().join(sanitized)
    }

    /// Get the file path for a checkpoint.
    fn checkpoint_path(&self, session_id: &str, id: &Uuid, turn: u32) -> PathBuf {
        self.session_dir(session_id)
            .join(format!("{}_turn_{:06}.json", id, turn))
    }

    /// Sanitize session ID for use as directory name.
    fn sanitize_session_id(session_id: &str) -> String {
        session_id
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect()
    }

    /// Ensure session directory exists.
    async fn ensure_session_dir(&self, session_id: &str) -> Result<PathBuf, CheckpointError> {
        let dir = self.session_dir(session_id);
        fs::create_dir_all(&dir).await?;
        Ok(dir)
    }

    /// Parse checkpoint ID and turn from filename.
    fn parse_filename(filename: &str) -> Option<(Uuid, u32)> {
        // Format: {uuid}_turn_{turn}.json
        let stem = filename.strip_suffix(".json")?;
        let parts: Vec<&str> = stem.split("_turn_").collect();
        if parts.len() != 2 {
            return None;
        }
        let uuid = Uuid::parse_str(parts[0]).ok()?;
        let turn = parts[1].parse().ok()?;
        Some((uuid, turn))
    }

    /// Read all checkpoints in a session directory.
    async fn read_session_checkpoints(&self, session_id: &str) -> Result<Vec<Checkpoint>, CheckpointError> {
        let session_dir = self.session_dir(session_id);

        if !session_dir.exists() {
            return Ok(Vec::new());
        }

        let mut checkpoints = Vec::new();
        let mut entries = fs::read_dir(&session_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                match fs::read_to_string(&path).await {
                    Ok(content) => {
                        match serde_json::from_str::<Checkpoint>(&content) {
                            Ok(checkpoint) => checkpoints.push(checkpoint),
                            Err(e) => {
                                warn!("Failed to deserialize checkpoint from {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read checkpoint file {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(checkpoints)
    }

    /// Find a checkpoint file by ID.
    async fn find_checkpoint_file(&self, id: &Uuid) -> Result<Option<PathBuf>, CheckpointError> {
        let checkpoints_dir = self.checkpoints_dir();

        if !checkpoints_dir.exists() {
            return Ok(None);
        }

        // Search through all session directories
        let mut sessions = fs::read_dir(&checkpoints_dir).await?;

        while let Some(session_entry) = sessions.next_entry().await? {
            let session_path = session_entry.path();
            if !session_path.is_dir() {
                continue;
            }

            let mut files = fs::read_dir(&session_path).await?;
            while let Some(file_entry) = files.next_entry().await? {
                let file_path = file_entry.path();
                if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
                    if let Some((file_id, _)) = Self::parse_filename(filename) {
                        if file_id == *id {
                            return Ok(Some(file_path));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

#[async_trait]
impl CheckpointStore for FileCheckpointStore {
    async fn save(&self, checkpoint: &Checkpoint) -> Result<(), CheckpointError> {
        self.ensure_session_dir(&checkpoint.session_id).await?;

        let path = self.checkpoint_path(&checkpoint.session_id, &checkpoint.id, checkpoint.turn);

        let content = serde_json::to_string_pretty(checkpoint).map_err(|e| {
            CheckpointError::Serialization(format!("Failed to serialize checkpoint: {}", e))
        })?;

        fs::write(&path, content).await?;

        debug!(
            "Saved checkpoint '{}' for session '{}' at turn {} to {:?}",
            checkpoint.id, checkpoint.session_id, checkpoint.turn, path
        );
        Ok(())
    }

    async fn get(&self, id: &Uuid) -> Result<Option<Checkpoint>, CheckpointError> {
        let Some(path) = self.find_checkpoint_file(id).await? else {
            return Ok(None);
        };

        let content = fs::read_to_string(&path).await?;

        let checkpoint: Checkpoint = serde_json::from_str(&content).map_err(|e| {
            CheckpointError::Serialization(format!("Failed to deserialize checkpoint: {}", e))
        })?;

        Ok(Some(checkpoint))
    }

    async fn get_latest(&self, session_id: &str) -> Result<Option<Checkpoint>, CheckpointError> {
        let checkpoints = self.read_session_checkpoints(session_id).await?;

        Ok(checkpoints.into_iter().max_by_key(|cp| cp.turn))
    }

    async fn list(&self, session_id: &str) -> Result<Vec<Checkpoint>, CheckpointError> {
        let mut checkpoints = self.read_session_checkpoints(session_id).await?;
        checkpoints.sort_by_key(|cp| cp.turn);
        Ok(checkpoints)
    }

    async fn delete(&self, id: &Uuid) -> Result<(), CheckpointError> {
        if let Some(path) = self.find_checkpoint_file(id).await? {
            fs::remove_file(&path).await?;
            debug!("Deleted checkpoint '{}'", id);
        }
        Ok(())
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), CheckpointError> {
        let session_dir = self.session_dir(session_id);

        if session_dir.exists() {
            fs::remove_dir_all(&session_dir).await?;
            debug!("Deleted all checkpoints for session '{}'", session_id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_checkpoint_store_save_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileCheckpointStore::new(temp_dir.path()).await.unwrap();

        let checkpoint = Checkpoint::new(
            "session1",
            5,
            serde_json::json!([{"role": "user", "content": "hello"}]),
            serde_json::json!({"key": "value"}),
        );
        let cp_id = checkpoint.id;

        // Save
        store.save(&checkpoint).await.unwrap();

        // Get by ID
        let loaded = store.get(&cp_id).await.unwrap();
        assert!(loaded.is_some());
        let loaded_cp = loaded.unwrap();
        assert_eq!(loaded_cp.session_id, "session1");
        assert_eq!(loaded_cp.turn, 5);
    }

    #[tokio::test]
    async fn test_file_checkpoint_store_get_latest() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileCheckpointStore::new(temp_dir.path()).await.unwrap();

        // Create multiple checkpoints for same session
        for turn in [1, 5, 3, 10, 7] {
            let cp = Checkpoint::new(
                "session1",
                turn,
                serde_json::json!([]),
                serde_json::json!({}),
            );
            store.save(&cp).await.unwrap();
        }

        // Get latest should return turn 10
        let latest = store.get_latest("session1").await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().turn, 10);
    }

    #[tokio::test]
    async fn test_file_checkpoint_store_list() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileCheckpointStore::new(temp_dir.path()).await.unwrap();

        // Create checkpoints in random order
        for turn in [5, 1, 3] {
            let cp = Checkpoint::new("session1", turn, serde_json::json!([]), serde_json::json!({}));
            store.save(&cp).await.unwrap();
        }

        // List should return sorted by turn
        let list = store.list("session1").await.unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].turn, 1);
        assert_eq!(list[1].turn, 3);
        assert_eq!(list[2].turn, 5);
    }

    #[tokio::test]
    async fn test_file_checkpoint_store_delete() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileCheckpointStore::new(temp_dir.path()).await.unwrap();

        let checkpoint = Checkpoint::new("session1", 5, serde_json::json!([]), serde_json::json!({}));
        let cp_id = checkpoint.id;

        store.save(&checkpoint).await.unwrap();
        assert!(store.get(&cp_id).await.unwrap().is_some());

        store.delete(&cp_id).await.unwrap();
        assert!(store.get(&cp_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_file_checkpoint_store_delete_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileCheckpointStore::new(temp_dir.path()).await.unwrap();

        // Create checkpoints for two sessions
        for turn in 1..=3 {
            let cp1 = Checkpoint::new("session1", turn, serde_json::json!([]), serde_json::json!({}));
            let cp2 = Checkpoint::new("session2", turn, serde_json::json!([]), serde_json::json!({}));
            store.save(&cp1).await.unwrap();
            store.save(&cp2).await.unwrap();
        }

        assert_eq!(store.list("session1").await.unwrap().len(), 3);
        assert_eq!(store.list("session2").await.unwrap().len(), 3);

        // Delete session1
        store.delete_session("session1").await.unwrap();

        assert_eq!(store.list("session1").await.unwrap().len(), 0);
        assert_eq!(store.list("session2").await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn test_file_checkpoint_store_get_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileCheckpointStore::new(temp_dir.path()).await.unwrap();

        let result = store.get(&Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_filename() {
        let uuid = Uuid::new_v4();
        let filename = format!("{}_turn_000005.json", uuid);
        let parsed = FileCheckpointStore::parse_filename(&filename);
        assert!(parsed.is_some());
        let (parsed_uuid, parsed_turn) = parsed.unwrap();
        assert_eq!(parsed_uuid, uuid);
        assert_eq!(parsed_turn, 5);
    }

    #[test]
    fn test_sanitize_session_id() {
        assert_eq!(FileCheckpointStore::sanitize_session_id("simple-session"), "simple-session");
        assert_eq!(FileCheckpointStore::sanitize_session_id("session/with/slashes"), "session_with_slashes");
        assert_eq!(FileCheckpointStore::sanitize_session_id("session:with:colons"), "session_with_colons");
    }
}
