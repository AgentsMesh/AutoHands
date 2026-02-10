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
#[path = "backend_tests.rs"]
mod tests;
