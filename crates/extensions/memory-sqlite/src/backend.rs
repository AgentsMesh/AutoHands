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

fn search_with_fts(
    conn: &rusqlite::Connection,
    text: &str,
    query: &MemoryQuery,
    limit: usize,
) -> Result<Vec<MemorySearchResult>, rusqlite::Error> {
    let sql = build_fts_query(query);
    let mut stmt = conn.prepare(&sql)?;
    execute_search(&mut stmt, text, query, limit)
}

fn search_without_fts(
    conn: &rusqlite::Connection,
    query: &MemoryQuery,
    limit: usize,
) -> Result<Vec<MemorySearchResult>, rusqlite::Error> {
    let sql = build_basic_query(query);
    let mut stmt = conn.prepare(&sql)?;
    execute_basic_search(&mut stmt, query, limit)
}

fn build_fts_query(query: &MemoryQuery) -> String {
    let mut sql = String::from(
        "SELECT m.id, m.content, m.memory_type, m.importance, m.created_at, m.metadata,
         bm25(memories_fts) as score
         FROM memories m
         JOIN memories_fts ON m.rowid = memories_fts.rowid
         WHERE memories_fts MATCH ?1"
    );

    if query.memory_type.is_some() {
        sql.push_str(" AND m.memory_type = ?2");
    }
    if !query.tags.is_empty() {
        sql.push_str(" AND EXISTS (SELECT 1 FROM memory_tags t WHERE t.memory_id = m.id AND t.tag IN (");
        sql.push_str(&query.tags.iter().map(|_| "?").collect::<Vec<_>>().join(","));
        sql.push_str("))");
    }

    sql.push_str(" ORDER BY score LIMIT ?");
    sql
}

fn build_basic_query(query: &MemoryQuery) -> String {
    let mut sql = String::from(
        "SELECT m.id, m.content, m.memory_type, m.importance, m.created_at, m.metadata,
         1.0 as score FROM memories m WHERE 1=1"
    );

    if query.memory_type.is_some() {
        sql.push_str(" AND m.memory_type = ?1");
    }
    if !query.tags.is_empty() {
        sql.push_str(" AND EXISTS (SELECT 1 FROM memory_tags t WHERE t.memory_id = m.id AND t.tag IN (");
        sql.push_str(&query.tags.iter().map(|_| "?").collect::<Vec<_>>().join(","));
        sql.push_str("))");
    }

    sql.push_str(" ORDER BY m.created_at DESC LIMIT ?");
    sql
}

fn execute_search(
    stmt: &mut rusqlite::Statement,
    text: &str,
    query: &MemoryQuery,
    limit: usize,
) -> Result<Vec<MemorySearchResult>, rusqlite::Error> {
    let mut idx = 1;
    stmt.raw_bind_parameter(idx, text)?;
    idx += 1;

    if let Some(ref mem_type) = query.memory_type {
        stmt.raw_bind_parameter(idx, mem_type)?;
        idx += 1;
    }

    for tag in &query.tags {
        stmt.raw_bind_parameter(idx, tag)?;
        idx += 1;
    }

    stmt.raw_bind_parameter(idx, limit as i64)?;

    collect_results(stmt, query.min_relevance)
}

fn execute_basic_search(
    stmt: &mut rusqlite::Statement,
    query: &MemoryQuery,
    limit: usize,
) -> Result<Vec<MemorySearchResult>, rusqlite::Error> {
    let mut idx = 1;

    if let Some(ref mem_type) = query.memory_type {
        stmt.raw_bind_parameter(idx, mem_type)?;
        idx += 1;
    }

    for tag in &query.tags {
        stmt.raw_bind_parameter(idx, tag)?;
        idx += 1;
    }

    stmt.raw_bind_parameter(idx, limit as i64)?;

    collect_results(stmt, query.min_relevance)
}

fn collect_results(
    stmt: &mut rusqlite::Statement,
    min_relevance: Option<f32>,
) -> Result<Vec<MemorySearchResult>, rusqlite::Error> {
    let mut results = Vec::new();
    let mut rows = stmt.raw_query();

    while let Some(row) = rows.next()? {
        let score: f64 = row.get(6)?;
        let relevance = (1.0 / (1.0 + (-score).exp())) as f32; // sigmoid normalization

        if let Some(min) = min_relevance {
            if relevance < min {
                continue;
            }
        }

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

        results.push(MemorySearchResult {
            entry: MemoryEntry {
                id: Some(id),
                content,
                memory_type,
                tags: Vec::new(), // Tags not loaded in search for performance
                created_at,
                importance,
                metadata,
            },
            relevance,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backend_id() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();
        assert_eq!(backend.id(), "sqlite");
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();
        let entry = MemoryEntry::new("Test content", "fact").with_tags(vec!["test".to_string()]);

        let id = backend.store(entry).await.unwrap();
        let retrieved = backend.retrieve(&id).await.unwrap().unwrap();

        assert_eq!(retrieved.content, "Test content");
        assert_eq!(retrieved.memory_type, "fact");
        assert!(retrieved.tags.contains(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_store_with_id() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();
        let mut entry = MemoryEntry::new("Test", "fact");
        entry.id = Some("custom-id-123".to_string());

        let id = backend.store(entry).await.unwrap();
        assert_eq!(id, "custom-id-123");

        let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, Some("custom-id-123".to_string()));
    }

    #[tokio::test]
    async fn test_store_with_importance() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();
        let entry = MemoryEntry::new("Important fact", "fact").with_importance(0.9);

        let id = backend.store(entry).await.unwrap();
        let retrieved = backend.retrieve(&id).await.unwrap().unwrap();

        assert_eq!(retrieved.importance, Some(0.9));
    }

    #[tokio::test]
    async fn test_store_with_metadata() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();
        let mut entry = MemoryEntry::new("Test", "fact");
        entry.metadata.insert("key".to_string(), serde_json::json!("value"));

        let id = backend.store(entry).await.unwrap();
        let retrieved = backend.retrieve(&id).await.unwrap().unwrap();

        assert_eq!(
            retrieved.metadata.get("key"),
            Some(&serde_json::json!("value"))
        );
    }

    #[tokio::test]
    async fn test_retrieve_nonexistent() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();
        let result = backend.retrieve("nonexistent-id").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();
        let entry = MemoryEntry::new("To delete", "temp");

        let id = backend.store(entry).await.unwrap();
        backend.delete(&id).await.unwrap();

        let retrieved = backend.retrieve(&id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();
        // Should not error
        let result = backend.delete("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();
        let entry = MemoryEntry::new("Original", "fact");

        let id = backend.store(entry).await.unwrap();

        let updated = MemoryEntry::new("Updated content", "fact")
            .with_tags(vec!["new-tag".to_string()]);
        backend.update(&id, updated).await.unwrap();

        let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Updated content");
        assert!(retrieved.tags.contains(&"new-tag".to_string()));
    }

    #[tokio::test]
    async fn test_search_basic() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();

        backend.store(MemoryEntry::new("Rust programming", "fact")).await.unwrap();
        backend.store(MemoryEntry::new("Python scripting", "fact")).await.unwrap();

        let query = MemoryQuery {
            text: None,
            memory_type: Some("fact".to_string()),
            tags: vec![],
            limit: 10,
            min_relevance: None,
        };

        let results = backend.search(query).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_search_with_fts() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();

        backend.store(MemoryEntry::new("The quick brown fox", "fact")).await.unwrap();
        backend.store(MemoryEntry::new("A lazy dog sleeps", "fact")).await.unwrap();

        let query = MemoryQuery {
            text: Some("fox".to_string()),
            memory_type: None,
            tags: vec![],
            limit: 10,
            min_relevance: None,
        };

        let results = backend.search(query).await.unwrap();
        assert!(!results.is_empty());
        assert!(results[0].entry.content.contains("fox"));
    }

    #[tokio::test]
    async fn test_search_by_type() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();

        backend.store(MemoryEntry::new("Fact content", "fact")).await.unwrap();
        backend.store(MemoryEntry::new("Preference content", "preference")).await.unwrap();

        let query = MemoryQuery {
            text: None,
            memory_type: Some("preference".to_string()),
            tags: vec![],
            limit: 10,
            min_relevance: None,
        };

        let results = backend.search(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entry.memory_type, "preference");
    }

    #[tokio::test]
    async fn test_search_limit() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();

        for i in 0..5 {
            backend.store(MemoryEntry::new(format!("Entry {}", i), "fact")).await.unwrap();
        }

        let query = MemoryQuery {
            text: None,
            memory_type: None,
            tags: vec![],
            limit: 2,
            min_relevance: None,
        };

        let results = backend.search(query).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_search_by_tags() {
        let backend = SqliteMemoryBackend::in_memory().await.unwrap();

        backend.store(MemoryEntry::new("Tagged entry", "fact").with_tags(vec!["special".to_string()])).await.unwrap();
        backend.store(MemoryEntry::new("Regular entry", "fact")).await.unwrap();

        let query = MemoryQuery {
            text: None,
            memory_type: None,
            tags: vec!["special".to_string()],
            limit: 10,
            min_relevance: None,
        };

        let results = backend.search(query).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_file_backend() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        {
            let backend = SqliteMemoryBackend::open(&db_path).await.unwrap();
            backend.store(MemoryEntry::new("Persistent", "fact")).await.unwrap();
        }

        // Reopen and verify
        let backend = SqliteMemoryBackend::open(&db_path).await.unwrap();
        let query = MemoryQuery {
            text: None,
            memory_type: None,
            tags: vec![],
            limit: 10,
            min_relevance: None,
        };
        let results = backend.search(query).await.unwrap();
        assert_eq!(results.len(), 1);
    }
}
