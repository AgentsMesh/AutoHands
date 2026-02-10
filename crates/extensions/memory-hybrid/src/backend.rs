//! Hybrid memory backend implementation.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use tracing::{debug, info};

use autohands_memory_vector::{Embedding, EmbeddingProvider, VectorMemoryBackend};
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
    embedder: Arc<dyn EmbeddingProvider>,
}

impl HybridMemoryBackend {
    /// Create a new hybrid backend with in-memory storage.
    pub async fn new(
        id: impl Into<String>,
        embedder: Arc<dyn EmbeddingProvider>,
        config: HybridMemoryConfig,
    ) -> Result<Self, MemoryError> {
        let id = id.into();
        let vector = VectorMemoryBackend::new(format!("{}-vector", id), embedder.clone());
        let fts = FTSBackend::new().await?;

        Ok(Self {
            id,
            vector,
            fts,
            config,
            entries: RwLock::new(HashMap::new()),
            embedder,
        })
    }

    /// Create with file-based FTS storage (embeddings persisted to SQLite).
    pub async fn with_fts_path(
        id: impl Into<String>,
        embedder: Arc<dyn EmbeddingProvider>,
        fts_path: impl Into<std::path::PathBuf>,
        config: HybridMemoryConfig,
    ) -> Result<Self, MemoryError> {
        let id = id.into();
        let vector = VectorMemoryBackend::new(format!("{}-vector", id), embedder.clone());
        let fts = FTSBackend::with_path(fts_path).await?;

        let backend = Self {
            id,
            vector,
            fts,
            config,
            entries: RwLock::new(HashMap::new()),
            embedder,
        };

        // Restore persisted embeddings from SQLite
        backend.restore_embeddings().await?;

        Ok(backend)
    }

    /// Restore embeddings and entries from SQLite on startup.
    async fn restore_embeddings(&self) -> Result<(), MemoryError> {
        let stored = self.fts.load_embeddings().await?;
        if stored.is_empty() {
            return Ok(());
        }

        let mut restored = 0;
        for (memory_id, vector) in &stored {
            let embedding = Embedding::new(vector.clone());
            self.vector.restore_embedding(memory_id.clone(), embedding);

            // Also restore the entry from FTS if available
            if let Some(entry) = self.fts.get_entry(memory_id) {
                self.entries.write().insert(memory_id.clone(), entry);
            }

            restored += 1;
        }

        if restored > 0 {
            info!(
                "Restored {} embeddings from persistent storage",
                restored
            );
        }
        Ok(())
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

        // Persist embedding to SQLite for restart recovery
        let embedding = self
            .embedder
            .embed(&entry.content)
            .await
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let dimension = embedding.dimension;
        if let Err(e) = self
            .fts
            .store_embedding(&id, &embedding.vector, "default", dimension)
            .await
        {
            // Non-fatal: log and continue
            debug!("Failed to persist embedding for {}: {}", id, e);
        }

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
        let _ = self.fts.remove_embedding(id).await;
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
#[path = "backend_tests.rs"]
mod tests;
