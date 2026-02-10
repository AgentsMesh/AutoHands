//! Vector index for similarity search.

use parking_lot::RwLock;
use std::collections::HashMap;

use crate::embedding::Embedding;

/// Search result from the index.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
}

/// Simple in-memory vector index using brute-force search.
pub struct VectorIndex {
    vectors: RwLock<HashMap<String, Embedding>>,
}

impl VectorIndex {
    /// Create a new vector index.
    pub fn new() -> Self {
        Self {
            vectors: RwLock::new(HashMap::new()),
        }
    }

    /// Insert a vector into the index.
    pub fn insert(&self, id: String, embedding: Embedding) {
        self.vectors.write().insert(id, embedding);
    }

    /// Remove a vector from the index.
    pub fn remove(&self, id: &str) -> Option<Embedding> {
        self.vectors.write().remove(id)
    }

    /// Get a vector by ID.
    pub fn get(&self, id: &str) -> Option<Embedding> {
        self.vectors.read().get(id).cloned()
    }

    /// Search for similar vectors.
    pub fn search(&self, query: &Embedding, limit: usize, min_score: f32) -> Vec<SearchResult> {
        let vectors = self.vectors.read();
        let mut results: Vec<SearchResult> = vectors
            .iter()
            .map(|(id, emb)| SearchResult {
                id: id.clone(),
                score: query.cosine_similarity(emb),
            })
            .filter(|r| r.score >= min_score)
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        results.truncate(limit);
        results
    }

    /// Get the number of vectors in the index.
    pub fn len(&self) -> usize {
        self.vectors.read().len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.vectors.read().is_empty()
    }

    /// Clear all vectors from the index.
    pub fn clear(&self) {
        self.vectors.write().clear();
    }
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "index_tests.rs"]
mod tests;
