//! Search functionality for SQLite memory backend.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use autohands_protocols::memory::{MemoryEntry, MemoryQuery, MemorySearchResult};

pub(crate) fn search_with_fts(
    conn: &rusqlite::Connection,
    text: &str,
    query: &MemoryQuery,
    limit: usize,
) -> Result<Vec<MemorySearchResult>, rusqlite::Error> {
    let sql = build_fts_query(query);
    let mut stmt = conn.prepare(&sql)?;
    execute_search(&mut stmt, text, query, limit)
}

pub(crate) fn search_without_fts(
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
