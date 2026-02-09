//! SQLite FTS5 full-text search backend.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use parking_lot::RwLock;
use tokio_rusqlite::Connection;
use tracing::{debug, error};

use autohands_protocols::error::MemoryError;
use autohands_protocols::memory::{MemoryEntry, MemoryQuery, MemorySearchResult};

/// FTS5 full-text search backend.
pub struct FTSBackend {
    conn: Arc<Connection>,
    entries: RwLock<HashMap<String, MemoryEntry>>,
}

impl FTSBackend {
    /// Create a new FTS backend with in-memory database.
    pub async fn new() -> Result<Self, MemoryError> {
        Self::with_path(":memory:").await
    }

    /// Create a new FTS backend with file database.
    pub async fn with_path(path: impl Into<PathBuf>) -> Result<Self, MemoryError> {
        let path: PathBuf = path.into();
        let path_str = path.to_string_lossy().to_string();

        let conn = Connection::open(path_str)
            .await
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        // Initialize FTS5 table
        conn.call(|conn| {
            conn.execute_batch(
                r#"
                CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
                    id,
                    content,
                    memory_type,
                    tags,
                    tokenize='porter unicode61'
                );
                "#,
            )?;
            Ok(())
        })
        .await
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        Ok(Self {
            conn: Arc::new(conn),
            entries: RwLock::new(HashMap::new()),
        })
    }

    /// Index a memory entry for FTS.
    pub async fn index(&self, entry: &MemoryEntry) -> Result<(), MemoryError> {
        let id = entry
            .id
            .clone()
            .ok_or_else(|| MemoryError::StorageError("Entry missing ID".to_string()))?;

        let content = entry.content.clone();
        let memory_type = entry.memory_type.clone();
        let tags = entry.tags.join(" ");

        // Remove existing if present
        let id_clone = id.clone();
        self.conn
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM memory_fts WHERE id = ?",
                    rusqlite::params![id_clone],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        // Insert new entry
        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO memory_fts (id, content, memory_type, tags) VALUES (?, ?, ?, ?)",
                    rusqlite::params![id, content, memory_type, tags],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        // Store entry in memory for retrieval
        self.entries
            .write()
            .insert(entry.id.clone().unwrap(), entry.clone());

        debug!("Indexed entry for FTS: {}", entry.id.as_ref().unwrap());
        Ok(())
    }

    /// Remove an entry from the FTS index.
    pub async fn remove(&self, id: &str) -> Result<(), MemoryError> {
        let id_owned = id.to_string();
        self.conn
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM memory_fts WHERE id = ?",
                    rusqlite::params![id_owned],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        self.entries.write().remove(id);
        Ok(())
    }

    /// Search using FTS5.
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(String, f32)>, MemoryError> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }

        // Escape special FTS5 characters and prepare query
        let escaped_query = escape_fts_query(query);

        let search_limit = limit * 2; // Get more results to allow for filtering
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT id, bm25(memory_fts) as score
                    FROM memory_fts
                    WHERE memory_fts MATCH ?
                    ORDER BY score
                    LIMIT ?
                    "#,
                )?;

                let results: Vec<(String, f32)> = stmt
                    .query_map(rusqlite::params![escaped_query, search_limit as i64], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)? as f32))
                    })?
                    .filter_map(|r| r.ok())
                    .collect();

                Ok(results)
            })
            .await
            .map_err(|e| MemoryError::QueryError(format!("FTS search failed: {}", e)))
    }

    /// Get an entry by ID.
    pub fn get_entry(&self, id: &str) -> Option<MemoryEntry> {
        self.entries.read().get(id).cloned()
    }
}

/// Escape special FTS5 query characters.
fn escape_fts_query(query: &str) -> String {
    // Split into words and join with OR for broader matching
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }

    // Use simple word matching with OR
    words
        .iter()
        .map(|w| format!("\"{}\"", w.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fts_backend_creation() {
        let backend = FTSBackend::new().await.unwrap();
        assert!(backend.entries.read().is_empty());
    }

    #[tokio::test]
    async fn test_fts_index_and_search() {
        let backend = FTSBackend::new().await.unwrap();

        let mut entry = MemoryEntry::new("Hello world test content", "fact");
        entry.id = Some("entry-1".to_string());

        backend.index(&entry).await.unwrap();

        let results = backend.search("hello", 10).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "entry-1");
    }

    #[tokio::test]
    async fn test_fts_remove() {
        let backend = FTSBackend::new().await.unwrap();

        let mut entry = MemoryEntry::new("Test content", "fact");
        entry.id = Some("entry-1".to_string());

        backend.index(&entry).await.unwrap();
        backend.remove("entry-1").await.unwrap();

        let results = backend.search("test", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_fts_empty_query() {
        let backend = FTSBackend::new().await.unwrap();
        let results = backend.search("", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_escape_fts_query() {
        let escaped = escape_fts_query("hello world");
        assert!(escaped.contains("hello"));
        assert!(escaped.contains("world"));
        assert!(escaped.contains("OR"));
    }

    #[test]
    fn test_escape_fts_query_with_quotes() {
        let escaped = escape_fts_query("hello \"world\"");
        // Quotes should be stripped
        assert!(!escaped.contains("\\\""));
    }

    #[tokio::test]
    async fn test_get_entry() {
        let backend = FTSBackend::new().await.unwrap();

        let mut entry = MemoryEntry::new("Test", "fact");
        entry.id = Some("entry-1".to_string());

        backend.index(&entry).await.unwrap();

        let retrieved = backend.get_entry("entry-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test");
    }

    #[tokio::test]
    async fn test_fts_multiple_entries() {
        let backend = FTSBackend::new().await.unwrap();

        for i in 0..5 {
            let mut entry = MemoryEntry::new(format!("Entry {} content about Rust", i), "fact");
            entry.id = Some(format!("entry-{}", i));
            backend.index(&entry).await.unwrap();
        }

        let results = backend.search("Rust", 10).await.unwrap();
        assert_eq!(results.len(), 5);
    }
}
