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
mod tests {
    use super::*;

    #[test]
    fn test_memory_entry_new() {
        let entry = MemoryEntry::new("Test content", "fact");
        assert_eq!(entry.content, "Test content");
        assert_eq!(entry.memory_type, "fact");
        assert!(entry.id.is_none());
        assert!(entry.tags.is_empty());
        assert!(entry.created_at.is_some());
    }

    #[test]
    fn test_memory_entry_with_tags() {
        let entry = MemoryEntry::new("Test", "fact")
            .with_tags(vec!["tag1".to_string(), "tag2".to_string()]);
        assert_eq!(entry.tags.len(), 2);
        assert!(entry.tags.contains(&"tag1".to_string()));
    }

    #[test]
    fn test_memory_entry_with_importance() {
        let entry = MemoryEntry::new("Test", "fact")
            .with_importance(0.8);
        assert_eq!(entry.importance, Some(0.8));
    }

    #[test]
    fn test_memory_entry_importance_clamped() {
        let entry1 = MemoryEntry::new("Test", "fact").with_importance(1.5);
        let entry2 = MemoryEntry::new("Test", "fact").with_importance(-0.5);
        assert_eq!(entry1.importance, Some(1.0));
        assert_eq!(entry2.importance, Some(0.0));
    }

    #[test]
    fn test_memory_entry_serialization() {
        let entry = MemoryEntry::new("Test content", "conversation");
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("Test content"));
        assert!(json.contains("conversation"));
    }

    #[test]
    fn test_memory_entry_deserialization() {
        let json = r#"{"content":"Test","memory_type":"fact"}"#;
        let entry: MemoryEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.content, "Test");
        assert_eq!(entry.memory_type, "fact");
    }

    #[test]
    fn test_memory_entry_clone() {
        let entry = MemoryEntry::new("Test", "fact")
            .with_tags(vec!["tag".to_string()])
            .with_importance(0.5);
        let cloned = entry.clone();
        assert_eq!(cloned.content, entry.content);
        assert_eq!(cloned.tags, entry.tags);
        assert_eq!(cloned.importance, entry.importance);
    }

    #[test]
    fn test_memory_query_text() {
        let query = MemoryQuery::text("search term");
        assert_eq!(query.text, Some("search term".to_string()));
        assert_eq!(query.limit, 10);
    }

    #[test]
    fn test_memory_query_with_limit() {
        let query = MemoryQuery::text("test").with_limit(20);
        assert_eq!(query.limit, 20);
    }

    #[test]
    fn test_memory_query_default() {
        let query = MemoryQuery::default();
        assert!(query.text.is_none());
        assert!(query.memory_type.is_none());
        assert!(query.tags.is_empty());
        assert_eq!(query.limit, 0);
    }

    #[test]
    fn test_memory_query_serialization() {
        let query = MemoryQuery::text("test").with_limit(5);
        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_memory_query_deserialization() {
        let json = r#"{"text":"search","limit":10}"#;
        let query: MemoryQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.text, Some("search".to_string()));
        assert_eq!(query.limit, 10);
    }

    #[test]
    fn test_memory_search_result_serialization() {
        let entry = MemoryEntry::new("Test", "fact");
        let result = MemorySearchResult {
            entry,
            relevance: 0.95,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("0.95"));
    }

    #[test]
    fn test_memory_search_result_clone() {
        let entry = MemoryEntry::new("Test", "fact");
        let result = MemorySearchResult {
            entry,
            relevance: 0.8,
        };
        let cloned = result.clone();
        assert_eq!(cloned.relevance, result.relevance);
    }

    #[test]
    fn test_memory_entry_debug() {
        let entry = MemoryEntry::new("Test", "fact");
        let debug = format!("{:?}", entry);
        assert!(debug.contains("MemoryEntry"));
    }

    #[test]
    fn test_memory_query_debug() {
        let query = MemoryQuery::text("test");
        let debug = format!("{:?}", query);
        assert!(debug.contains("MemoryQuery"));
    }

    #[test]
    fn test_memory_search_result_debug() {
        let entry = MemoryEntry::new("Test", "fact");
        let result = MemorySearchResult {
            entry,
            relevance: 0.5,
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("MemorySearchResult"));
    }

    #[test]
    fn test_memory_query_with_all_fields() {
        let query = MemoryQuery {
            text: Some("search".to_string()),
            memory_type: Some("fact".to_string()),
            tags: vec!["tag1".to_string()],
            limit: 15,
            min_relevance: Some(0.5),
        };
        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("search"));
        assert!(json.contains("fact"));
        assert!(json.contains("tag1"));
    }
}
