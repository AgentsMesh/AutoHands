//! Memory backend protocol definitions.
//!
//! Memory backends store and retrieve information for agents.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::MemoryError;
use crate::types::Metadata;

/// Core trait for memory backends.
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Returns the backend ID.
    fn id(&self) -> &str;

    /// Store a memory entry.
    async fn store(&self, entry: MemoryEntry) -> Result<String, MemoryError>;

    /// Retrieve a memory entry by ID.
    async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError>;

    /// Search for memory entries.
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemorySearchResult>, MemoryError>;

    /// Delete a memory entry.
    async fn delete(&self, id: &str) -> Result<(), MemoryError>;

    /// Update a memory entry.
    async fn update(&self, id: &str, entry: MemoryEntry) -> Result<(), MemoryError>;
}

/// A memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Optional ID (assigned by backend if not provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Content of the memory.
    pub content: String,

    /// Type of memory (e.g., "conversation", "fact", "preference").
    pub memory_type: String,

    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,

    /// When the memory was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Importance score (0.0 - 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importance: Option<f32>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: Metadata,
}

impl MemoryEntry {
    pub fn new(content: impl Into<String>, memory_type: impl Into<String>) -> Self {
        Self {
            id: None,
            content: content.into(),
            memory_type: memory_type.into(),
            tags: Vec::new(),
            created_at: Some(chrono::Utc::now()),
            importance: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = Some(importance.clamp(0.0, 1.0));
        self
    }
}

/// Query for searching memories.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// Text query for semantic search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Filter by memory type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_type: Option<String>,

    /// Filter by tags (any match).
    #[serde(default)]
    pub tags: Vec<String>,

    /// Maximum number of results.
    pub limit: usize,

    /// Minimum relevance score (0.0 - 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_relevance: Option<f32>,
}

impl MemoryQuery {
    pub fn text(query: impl Into<String>) -> Self {
        Self {
            text: Some(query.into()),
            memory_type: None,
            tags: Vec::new(),
            limit: 10,
            min_relevance: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

/// Result from a memory search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    pub entry: MemoryEntry,
    pub relevance: f32,
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
