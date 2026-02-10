//! SQLite FTS5 full-text search backend.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use parking_lot::RwLock;
use tokio_rusqlite::Connection;
use tracing::debug;

use autohands_protocols::error::MemoryError;
use autohands_protocols::memory::MemoryEntry;

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

        // Initialize FTS5 table + embeddings tables
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

                CREATE TABLE IF NOT EXISTS embeddings (
                    memory_id TEXT PRIMARY KEY,
                    vector BLOB NOT NULL,
                    model TEXT NOT NULL,
                    dimension INTEGER NOT NULL,
                    created_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS embedding_cache (
                    content_hash TEXT PRIMARY KEY,
                    provider TEXT NOT NULL,
                    model TEXT NOT NULL,
                    vector BLOB NOT NULL,
                    created_at TEXT NOT NULL
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

    // -----------------------------------------------------------------------
    // Embedding persistence
    // -----------------------------------------------------------------------

    /// Store an embedding vector for a memory entry.
    pub async fn store_embedding(
        &self,
        memory_id: &str,
        vector: &[f32],
        model: &str,
        dimension: usize,
    ) -> Result<(), MemoryError> {
        let id = memory_id.to_string();
        let blob = f32_vec_to_blob(vector);
        let model = model.to_string();
        let now = Utc::now().to_rfc3339();

        self.conn
            .call(move |conn| {
                conn.execute(
                    r#"INSERT OR REPLACE INTO embeddings (memory_id, vector, model, dimension, created_at)
                       VALUES (?, ?, ?, ?, ?)"#,
                    rusqlite::params![id, blob, model, dimension as i64, now],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| MemoryError::StorageError(format!("Failed to store embedding: {}", e)))
    }

    /// Load all stored embeddings. Returns Vec<(memory_id, vector)>.
    pub async fn load_embeddings(&self) -> Result<Vec<(String, Vec<f32>)>, MemoryError> {
        self.conn
            .call(|conn| {
                let mut stmt = conn.prepare("SELECT memory_id, vector FROM embeddings")?;
                let rows: Vec<(String, Vec<f32>)> = stmt
                    .query_map([], |row| {
                        let id: String = row.get(0)?;
                        let blob: Vec<u8> = row.get(1)?;
                        Ok((id, blob_to_f32_vec(&blob)))
                    })?
                    .filter_map(|r| r.ok())
                    .collect();
                Ok(rows)
            })
            .await
            .map_err(|e| MemoryError::StorageError(format!("Failed to load embeddings: {}", e)))
    }

    /// Remove an embedding by memory ID.
    pub async fn remove_embedding(&self, memory_id: &str) -> Result<(), MemoryError> {
        let id = memory_id.to_string();
        self.conn
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM embeddings WHERE memory_id = ?",
                    rusqlite::params![id],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| MemoryError::StorageError(format!("Failed to remove embedding: {}", e)))
    }

    /// Look up a cached embedding by content hash.
    pub async fn get_cached_embedding(&self, content_hash: &str) -> Result<Option<Vec<f32>>, MemoryError> {
        let hash = content_hash.to_string();
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT vector FROM embedding_cache WHERE content_hash = ?",
                )?;
                let result = stmt
                    .query_row(rusqlite::params![hash], |row| {
                        let blob: Vec<u8> = row.get(0)?;
                        Ok(blob_to_f32_vec(&blob))
                    })
                    .ok();
                Ok(result)
            })
            .await
            .map_err(|e| MemoryError::StorageError(format!("Failed to query embedding cache: {}", e)))
    }

    /// Cache an embedding.
    pub async fn cache_embedding(
        &self,
        content_hash: &str,
        provider: &str,
        model: &str,
        vector: &[f32],
    ) -> Result<(), MemoryError> {
        let hash = content_hash.to_string();
        let provider = provider.to_string();
        let model = model.to_string();
        let blob = f32_vec_to_blob(vector);
        let now = Utc::now().to_rfc3339();

        self.conn
            .call(move |conn| {
                conn.execute(
                    r#"INSERT OR REPLACE INTO embedding_cache (content_hash, provider, model, vector, created_at)
                       VALUES (?, ?, ?, ?, ?)"#,
                    rusqlite::params![hash, provider, model, blob, now],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| MemoryError::StorageError(format!("Failed to cache embedding: {}", e)))
    }
}

/// Convert f32 slice to byte blob for SQLite storage.
fn f32_vec_to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Convert byte blob back to f32 vector.
fn blob_to_f32_vec(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
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
#[path = "fts_tests.rs"]
mod tests;
