//! Hybrid memory backend implementation.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use tracing::{debug, info};

use autohands_memory_vector::{EmbeddingProvider, VectorMemoryBackend};
use autohands_protocols::error::MemoryError;
use autohands_protocols::memory::{MemoryBackend, MemoryEntry, MemoryQuery, MemorySearchResult};

use crate::fts::FTSBackend;
use crate::fusion::{rrf_fusion, FusionConfig};

/// Configuration for the hybrid memory backend.
#[derive(Debug, Clone)]
pub struct HybridMemoryConfig {
    /// Fusion configuration for combining results.
    pub fusion: FusionConfig,
    /// Minimum relevance threshold (0.0 - 1.0).
    pub min_relevance: f32,
}

impl Default for HybridMemoryConfig {
    fn default() -> Self {
        Self {
            fusion: FusionConfig::default(),
            min_relevance: 0.0,
        }
    }
}

/// Hybrid memory backend combining vector and keyword search.
pub struct HybridMemoryBackend {
    id: String,
    vector: VectorMemoryBackend,
    fts: FTSBackend,
    config: HybridMemoryConfig,
    entries: RwLock<HashMap<String, MemoryEntry>>,
}

impl HybridMemoryBackend {
    /// Create a new hybrid backend with in-memory storage.
    pub async fn new(
        id: impl Into<String>,
        embedder: Arc<dyn EmbeddingProvider>,
        config: HybridMemoryConfig,
    ) -> Result<Self, MemoryError> {
        let id = id.into();
        let vector = VectorMemoryBackend::new(format!("{}-vector", id), embedder);
        let fts = FTSBackend::new().await?;

        Ok(Self {
            id,
            vector,
            fts,
            config,
            entries: RwLock::new(HashMap::new()),
        })
    }

    /// Create with file-based FTS storage.
    pub async fn with_fts_path(
        id: impl Into<String>,
        embedder: Arc<dyn EmbeddingProvider>,
        fts_path: impl Into<std::path::PathBuf>,
        config: HybridMemoryConfig,
    ) -> Result<Self, MemoryError> {
        let id = id.into();
        let vector = VectorMemoryBackend::new(format!("{}-vector", id), embedder);
        let fts = FTSBackend::with_path(fts_path).await?;

        Ok(Self {
            id,
            vector,
            fts,
            config,
            entries: RwLock::new(HashMap::new()),
        })
    }

    /// Perform hybrid search combining vector and keyword results.
    async fn hybrid_search(&self, query: &MemoryQuery) -> Result<Vec<MemorySearchResult>, MemoryError> {
        let text = match &query.text {
            Some(t) => t,
            None => {
                // No text query, just return vector results
                return self.vector.search(query.clone()).await;
            }
        };

        // Run both searches in parallel
        let vector_query = query.clone();
        let fts_limit = query.limit * 2; // Get more for fusion

        let (vector_results, keyword_results) = tokio::join!(
            self.vector.search(vector_query),
            self.fts.search(text, fts_limit)
        );

        let vector_results = vector_results?;
        let keyword_results = keyword_results?;

        // Convert to (id, score) format for fusion
        let vector_pairs: Vec<(String, f32)> = vector_results
            .iter()
            .filter_map(|r| r.entry.id.clone().map(|id| (id, r.relevance)))
            .collect();

        // Normalize FTS scores (BM25 can be negative, convert to 0-1 range)
        let keyword_pairs: Vec<(String, f32)> = if keyword_results.is_empty() {
            vec![]
        } else {
            let min_score = keyword_results
                .iter()
                .map(|(_, s)| *s)
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0);
            let max_score = keyword_results
                .iter()
                .map(|(_, s)| *s)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(1.0);

            let range = (max_score - min_score).max(0.001);

            keyword_results
                .into_iter()
                .map(|(id, score)| {
                    let normalized = (score - min_score) / range;
                    (id, normalized)
                })
                .collect()
        };

        // Fuse results
        let fused = rrf_fusion(&vector_pairs, &keyword_pairs, &self.config.fusion);

        // Convert back to MemorySearchResult
        let entries = self.entries.read();
        let mut results: Vec<MemorySearchResult> = fused
            .into_iter()
            .take(query.limit)
            .filter_map(|(id, score)| {
                entries.get(&id).map(|entry| {
                    // Apply type and tag filters
                    let type_match = query
                        .memory_type
                        .as_ref()
                        .map(|t| t == &entry.memory_type)
                        .unwrap_or(true);

                    let tags_match = query.tags.is_empty()
                        || query.tags.iter().any(|t| entry.tags.contains(t));

                    if type_match && tags_match && score >= self.config.min_relevance {
                        Some(MemorySearchResult {
                            entry: entry.clone(),
                            relevance: score,
                        })
                    } else {
                        None
                    }
                })
            })
            .flatten()
            .collect();

        results.truncate(query.limit);
        Ok(results)
    }
}

#[async_trait]
impl MemoryBackend for HybridMemoryBackend {
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

        // Store in both backends
        self.vector.store(entry.clone()).await?;
        self.fts.index(&entry).await?;

        // Store locally for retrieval
        self.entries.write().insert(id.clone(), entry);

        debug!("Stored entry in hybrid backend: {}", id);
        Ok(id)
    }

    async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        Ok(self.entries.read().get(id).cloned())
    }

    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemorySearchResult>, MemoryError> {
        self.hybrid_search(&query).await
    }

    async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        self.vector.delete(id).await?;
        self.fts.remove(id).await?;
        self.entries.write().remove(id);
        debug!("Deleted entry from hybrid backend: {}", id);
        Ok(())
    }

    async fn update(&self, id: &str, mut entry: MemoryEntry) -> Result<(), MemoryError> {
        entry.id = Some(id.to_string());

        self.vector.update(id, entry.clone()).await?;
        self.fts.index(&entry).await?;
        self.entries.write().insert(id.to_string(), entry);

        debug!("Updated entry in hybrid backend: {}", id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use autohands_memory_vector::SimpleHashEmbedding;

    async fn create_test_backend() -> HybridMemoryBackend {
        let embedder = Arc::new(SimpleHashEmbedding::default());
        HybridMemoryBackend::new("test", embedder, HybridMemoryConfig::default())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_hybrid_backend_creation() {
        let backend = create_test_backend().await;
        assert_eq!(backend.id(), "test");
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let backend = create_test_backend().await;

        let entry = MemoryEntry::new("Test content about Rust programming", "fact");
        let id = backend.store(entry).await.unwrap();

        let retrieved = backend.retrieve(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().content.contains("Rust"));
    }

    #[tokio::test]
    async fn test_hybrid_search() {
        let backend = create_test_backend().await;

        backend
            .store(MemoryEntry::new("Rust programming language is fast", "fact"))
            .await
            .unwrap();
        backend
            .store(MemoryEntry::new("Python is a scripting language", "fact"))
            .await
            .unwrap();
        backend
            .store(MemoryEntry::new("Rust has zero-cost abstractions", "fact"))
            .await
            .unwrap();

        let query = MemoryQuery::text("Rust programming").with_limit(10);
        let results = backend.search(query).await.unwrap();

        // Should find Rust-related entries
        assert!(!results.is_empty());
        assert!(results
            .iter()
            .any(|r| r.entry.content.contains("Rust")));
    }

    #[tokio::test]
    async fn test_delete() {
        let backend = create_test_backend().await;

        let entry = MemoryEntry::new("To be deleted", "fact");
        let id = backend.store(entry).await.unwrap();

        backend.delete(&id).await.unwrap();
        assert!(backend.retrieve(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update() {
        let backend = create_test_backend().await;

        let entry = MemoryEntry::new("Original", "fact");
        let id = backend.store(entry).await.unwrap();

        let updated = MemoryEntry::new("Updated content", "fact");
        backend.update(&id, updated).await.unwrap();

        let retrieved = backend.retrieve(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Updated content");
    }

    #[tokio::test]
    async fn test_search_with_type_filter() {
        let backend = create_test_backend().await;

        backend
            .store(MemoryEntry::new("Fact entry", "fact"))
            .await
            .unwrap();
        backend
            .store(MemoryEntry::new("Preference entry", "preference"))
            .await
            .unwrap();

        let mut query = MemoryQuery::text("entry").with_limit(10);
        query.memory_type = Some("fact".to_string());

        let results = backend.search(query).await.unwrap();
        assert!(results.iter().all(|r| r.entry.memory_type == "fact"));
    }

    #[test]
    fn test_config_default() {
        let config = HybridMemoryConfig::default();
        assert!((config.min_relevance - 0.0).abs() < 0.01);
    }
}
