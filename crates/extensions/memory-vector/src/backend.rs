//! Vector memory backend implementation.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use tracing::debug;

use autohands_protocols::error::MemoryError;
use autohands_protocols::memory::{
    MemoryBackend, MemoryEntry, MemoryQuery, MemorySearchResult,
};

use crate::embedding::{EmbeddingProvider, SimpleHashEmbedding};
use crate::index::VectorIndex;

/// Vector memory backend with semantic search.
pub struct VectorMemoryBackend {
    id: String,
    embedder: Arc<dyn EmbeddingProvider>,
    index: VectorIndex,
    entries: RwLock<HashMap<String, MemoryEntry>>,
}

impl VectorMemoryBackend {
    /// Create with a custom embedding provider.
    pub fn new(id: impl Into<String>, embedder: Arc<dyn EmbeddingProvider>) -> Self {
        Self {
            id: id.into(),
            embedder,
            index: VectorIndex::new(),
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Create with the default simple hash embedding.
    pub fn with_simple_embedding(id: impl Into<String>) -> Self {
        Self::new(id, Arc::new(SimpleHashEmbedding::default()))
    }

    /// Restore a pre-computed embedding into the index without re-embedding.
    /// Used for restoring persisted embeddings from storage.
    pub fn restore_embedding(&self, id: String, embedding: crate::embedding::Embedding) {
        self.index.insert(id, embedding);
    }
}

#[async_trait]
impl MemoryBackend for VectorMemoryBackend {
    fn id(&self) -> &str {
        &self.id
    }

    async fn store(&self, mut entry: MemoryEntry) -> Result<String, MemoryError> {
        let id = entry
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        entry.id = Some(id.clone());
        if entry.created_at.is_none() {
            entry.created_at = Some(Utc::now());
        }

        // Generate embedding for the content
        let embedding = self
            .embedder
            .embed(&entry.content)
            .await
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        // Store in index and entries map
        self.index.insert(id.clone(), embedding);
        self.entries.write().insert(id.clone(), entry);

        debug!("Stored memory entry: {}", id);
        Ok(id)
    }

    async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        Ok(self.entries.read().get(id).cloned())
    }

    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemorySearchResult>, MemoryError> {
        let min_relevance = query.min_relevance.unwrap_or(0.0);

        // If text query provided, use semantic search
        let results = if let Some(ref text) = query.text {
            let query_embedding = self
                .embedder
                .embed(text)
                .await
                .map_err(|e| MemoryError::QueryError(e.to_string()))?;

            self.index.search(&query_embedding, query.limit, min_relevance)
        } else {
            // No text query, return all entries up to limit
            let entries = self.entries.read();
            entries
                .keys()
                .take(query.limit)
                .map(|id| crate::index::SearchResult {
                    id: id.clone(),
                    score: 1.0,
                })
                .collect()
        };

        // Convert to MemorySearchResult with full entries
        let entries = self.entries.read();
        let mut memory_results: Vec<MemorySearchResult> = results
            .into_iter()
            .filter_map(|r| {
                entries.get(&r.id).map(|entry| {
                    // Apply filters
                    let type_match = query
                        .memory_type
                        .as_ref()
                        .map(|t| t == &entry.memory_type)
                        .unwrap_or(true);

                    let tags_match = query.tags.is_empty()
                        || query.tags.iter().any(|t| entry.tags.contains(t));

                    if type_match && tags_match {
                        Some(MemorySearchResult {
                            entry: entry.clone(),
                            relevance: r.score,
                        })
                    } else {
                        None
                    }
                })
            })
            .flatten()
            .collect();

        // Sort by relevance
        memory_results.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        memory_results.truncate(query.limit);
        Ok(memory_results)
    }

    async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        self.index.remove(id);
        self.entries.write().remove(id);
        debug!("Deleted memory entry: {}", id);
        Ok(())
    }

    async fn update(&self, id: &str, mut entry: MemoryEntry) -> Result<(), MemoryError> {
        entry.id = Some(id.to_string());

        // Update embedding
        let embedding = self
            .embedder
            .embed(&entry.content)
            .await
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        self.index.insert(id.to_string(), embedding);
        self.entries.write().insert(id.to_string(), entry);

        debug!("Updated memory entry: {}", id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_backend() -> VectorMemoryBackend {
        VectorMemoryBackend::with_simple_embedding("test")
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let backend = create_backend();

        let entry = MemoryEntry::new("Hello world", "fact");
        let id = backend.store(entry).await.unwrap();

        let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Hello world");
        assert_eq!(retrieved.memory_type, "fact");
    }

    #[tokio::test]
    async fn test_delete() {
        let backend = create_backend();

        let entry = MemoryEntry::new("Test content", "fact");
        let id = backend.store(entry).await.unwrap();

        backend.delete(&id).await.unwrap();
        assert!(backend.retrieve(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update() {
        let backend = create_backend();

        let entry = MemoryEntry::new("Original", "fact");
        let id = backend.store(entry).await.unwrap();

        let updated = MemoryEntry::new("Updated", "fact");
        backend.update(&id, updated).await.unwrap();

        let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Updated");
    }

    #[tokio::test]
    async fn test_semantic_search() {
        let backend = create_backend();

        // Store some entries
        backend
            .store(MemoryEntry::new("The cat sat on the mat", "fact"))
            .await
            .unwrap();
        backend
            .store(MemoryEntry::new("Dogs are friendly animals", "fact"))
            .await
            .unwrap();
        backend
            .store(MemoryEntry::new("Cats are independent creatures", "fact"))
            .await
            .unwrap();

        // Search for cat-related content
        let query = MemoryQuery::text("cat").with_limit(2);
        let results = backend.search(query).await.unwrap();

        assert!(!results.is_empty());
        assert!(results.len() <= 2);
    }

    #[tokio::test]
    async fn test_search_with_type_filter() {
        let backend = create_backend();

        backend
            .store(MemoryEntry::new("Fact about cats", "fact"))
            .await
            .unwrap();
        backend
            .store(MemoryEntry::new("User prefers cats", "preference"))
            .await
            .unwrap();

        let mut query = MemoryQuery::text("cats").with_limit(10);
        query.memory_type = Some("fact".to_string());

        let results = backend.search(query).await.unwrap();
        assert!(results.iter().all(|r| r.entry.memory_type == "fact"));
    }

    #[tokio::test]
    async fn test_search_with_tags_filter() {
        let backend = create_backend();

        backend
            .store(MemoryEntry::new("Tagged entry", "fact").with_tags(vec!["important".into()]))
            .await
            .unwrap();
        backend
            .store(MemoryEntry::new("Untagged entry", "fact"))
            .await
            .unwrap();

        let mut query = MemoryQuery::text("entry").with_limit(10);
        query.tags = vec!["important".to_string()];

        let results = backend.search(query).await.unwrap();
        assert!(results.iter().all(|r| r.entry.tags.contains(&"important".to_string())));
    }

    #[test]
    fn test_backend_id() {
        let backend = create_backend();
        assert_eq!(backend.id(), "test");
    }

    #[tokio::test]
    async fn test_store_with_custom_id() {
        let backend = create_backend();
        let mut entry = MemoryEntry::new("Test", "fact");
        entry.id = Some("custom-id".to_string());

        let id = backend.store(entry).await.unwrap();
        assert_eq!(id, "custom-id");
    }

    #[tokio::test]
    async fn test_store_with_created_at() {
        let backend = create_backend();
        let mut entry = MemoryEntry::new("Test", "fact");
        let fixed_time = chrono::Utc::now() - chrono::Duration::hours(1);
        entry.created_at = Some(fixed_time);

        let id = backend.store(entry).await.unwrap();
        let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.created_at, Some(fixed_time));
    }

    #[tokio::test]
    async fn test_retrieve_nonexistent() {
        let backend = create_backend();
        let result = backend.retrieve("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_search_without_text_query() {
        let backend = create_backend();

        backend.store(MemoryEntry::new("Entry 1", "fact")).await.unwrap();
        backend.store(MemoryEntry::new("Entry 2", "fact")).await.unwrap();
        backend.store(MemoryEntry::new("Entry 3", "fact")).await.unwrap();

        let query = MemoryQuery {
            text: None,
            memory_type: None,
            tags: vec![],
            limit: 10,
            min_relevance: None,
        };

        let results = backend.search(query).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_search_with_limit() {
        let backend = create_backend();

        for i in 0..5 {
            backend.store(MemoryEntry::new(format!("Entry {}", i), "fact")).await.unwrap();
        }

        let query = MemoryQuery::text("Entry").with_limit(2);
        let results = backend.search(query).await.unwrap();
        assert!(results.len() <= 2);
    }

    #[tokio::test]
    async fn test_search_with_min_relevance() {
        let backend = create_backend();

        backend.store(MemoryEntry::new("Highly relevant content about cats", "fact")).await.unwrap();
        backend.store(MemoryEntry::new("Something completely different", "fact")).await.unwrap();

        let mut query = MemoryQuery::text("cats").with_limit(10);
        query.min_relevance = Some(0.1);

        let results = backend.search(query).await.unwrap();
        // Results should be filtered by relevance
        assert!(results.iter().all(|r| r.relevance >= 0.1));
    }

    #[tokio::test]
    async fn test_backend_new_with_custom_embedder() {
        let embedder = Arc::new(SimpleHashEmbedding::new(64));
        let backend = VectorMemoryBackend::new("custom", embedder);
        assert_eq!(backend.id(), "custom");
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let backend = create_backend();
        // Should not error when deleting non-existent entry
        let result = backend.delete("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_nonexistent() {
        let backend = create_backend();
        // Update creates entry if it doesn't exist
        let entry = MemoryEntry::new("New content", "fact");
        let result = backend.update("new-id", entry).await;
        assert!(result.is_ok());

        let retrieved = backend.retrieve("new-id").await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_search_sorts_by_relevance() {
        let backend = create_backend();

        backend.store(MemoryEntry::new("cats cats cats", "fact")).await.unwrap();
        backend.store(MemoryEntry::new("dogs are cute", "fact")).await.unwrap();
        backend.store(MemoryEntry::new("cats are independent", "fact")).await.unwrap();

        let query = MemoryQuery::text("cats").with_limit(10);
        let results = backend.search(query).await.unwrap();

        // Check that results are sorted by relevance (descending)
        for i in 1..results.len() {
            assert!(results[i - 1].relevance >= results[i].relevance);
        }
    }
}
