//! SQLite memory backend implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::params;
use std::collections::HashMap;
use std::path::Path;
use tokio_rusqlite::Connection;

use autohands_protocols::error::MemoryError;
use autohands_protocols::memory::{MemoryBackend, MemoryEntry, MemoryQuery, MemorySearchResult};

use crate::schema::init_schema;

#[path = "backend_search.rs"]
mod backend_search;
use backend_search::{search_with_fts, search_without_fts};

#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;

/// SQLite-based memory backend.
pub struct SqliteMemoryBackend {
    conn: Connection,
}

impl SqliteMemoryBackend {
    /// Create a new in-memory database.
    pub async fn in_memory() -> Result<Self, MemoryError> {
        let conn = Connection::open_in_memory()
            .await
            .map_err(|e| MemoryError::ConnectionError(e.to_string()))?;

        conn.call(|conn| Ok(init_schema(conn)?))
            .await
            .map_err(|e| MemoryError::QueryError(e.to_string()))?;

        Ok(Self { conn })
    }

    /// Create a new file-backed database.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let path = path.as_ref().to_path_buf();
        let conn = Connection::open(path)
            .await
            .map_err(|e| MemoryError::ConnectionError(e.to_string()))?;

        conn.call(|conn| Ok(init_schema(conn)?))
            .await
            .map_err(|e| MemoryError::QueryError(e.to_string()))?;

        Ok(Self { conn })
    }
}

#[async_trait]
impl MemoryBackend for SqliteMemoryBackend {
    fn id(&self) -> &str {
        "sqlite"
    }

    async fn store(&self, entry: MemoryEntry) -> Result<String, MemoryError> {
        let id = entry.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let now = Utc::now().to_rfc3339();
        let created = entry.created_at.map(|t| t.to_rfc3339()).unwrap_or_else(|| now.clone());
        let metadata = serde_json::to_string(&entry.metadata).unwrap_or_else(|_| "{}".to_string());
        let tags = entry.tags.clone();

        let id_clone = id.clone();
        self.conn
            .call(move |conn| {
                let tx = conn.transaction()?;

                tx.execute(
                    "INSERT INTO memories (id, content, memory_type, importance, created_at, updated_at, metadata)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![id_clone, entry.content, entry.memory_type, entry.importance, created, now, metadata],
                )?;

                for tag in tags {
                    tx.execute(
                        "INSERT INTO memory_tags (memory_id, tag) VALUES (?1, ?2)",
                        params![id_clone, tag],
                    )?;
                }

                tx.commit()?;
                Ok(())
            })
            .await
            .map_err(|e| MemoryError::QueryError(e.to_string()))?;

        Ok(id)
    }

    async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let id = id.to_string();
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, content, memory_type, importance, created_at, metadata
                     FROM memories WHERE id = ?1",
                )?;

                let entry = stmt.query_row([&id], |row| {
                    let id: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    let memory_type: String = row.get(2)?;
                    let importance: Option<f32> = row.get(3)?;
                    let created_str: String = row.get(4)?;
                    let metadata_str: String = row.get(5)?;

                    let created_at = DateTime::parse_from_rfc3339(&created_str)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc));
                    let metadata: HashMap<String, serde_json::Value> =
                        serde_json::from_str(&metadata_str).unwrap_or_default();

                    Ok((id, content, memory_type, importance, created_at, metadata))
                });

                match entry {
                    Ok((id, content, memory_type, importance, created_at, metadata)) => {
                        // Get tags
                        let mut tag_stmt = conn.prepare(
                            "SELECT tag FROM memory_tags WHERE memory_id = ?1"
                        )?;
                        let tags: Vec<String> = tag_stmt
                            .query_map([&id], |row| row.get(0))?
                            .filter_map(|r| r.ok())
                            .collect();

                        Ok(Some(MemoryEntry {
                            id: Some(id),
                            content,
                            memory_type,
                            tags,
                            created_at,
                            importance,
                            metadata,
                        }))
                    }
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.into()),
                }
            })
            .await
            .map_err(|e| MemoryError::QueryError(e.to_string()))
    }

    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemorySearchResult>, MemoryError> {
        let limit = query.limit;
        self.conn
            .call(move |conn| {
                let results = if let Some(text) = &query.text {
                    search_with_fts(conn, text, &query, limit)?
                } else {
                    search_without_fts(conn, &query, limit)?
                };
                Ok(results)
            })
            .await
            .map_err(|e| MemoryError::QueryError(e.to_string()))
    }

    async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        let id = id.to_string();
        self.conn
            .call(move |conn| {
                conn.execute("DELETE FROM memories WHERE id = ?1", [&id])?;
                Ok(())
            })
            .await
            .map_err(|e| MemoryError::QueryError(e.to_string()))
    }

    async fn update(&self, id: &str, entry: MemoryEntry) -> Result<(), MemoryError> {
        let id = id.to_string();
        let now = Utc::now().to_rfc3339();
        let metadata = serde_json::to_string(&entry.metadata).unwrap_or_else(|_| "{}".to_string());
        let tags = entry.tags.clone();

        self.conn
            .call(move |conn| {
                let tx = conn.transaction()?;

                tx.execute(
                    "UPDATE memories SET content = ?1, memory_type = ?2, importance = ?3,
                     updated_at = ?4, metadata = ?5 WHERE id = ?6",
                    params![entry.content, entry.memory_type, entry.importance, now, metadata, id],
                )?;

                // Update tags
                tx.execute("DELETE FROM memory_tags WHERE memory_id = ?1", [&id])?;
                for tag in tags {
                    tx.execute(
                        "INSERT INTO memory_tags (memory_id, tag) VALUES (?1, ?2)",
                        params![id, tag],
                    )?;
                }

                tx.commit()?;
                Ok(())
            })
            .await
            .map_err(|e| MemoryError::QueryError(e.to_string()))
    }
}
