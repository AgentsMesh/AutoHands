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
mod tests {
    use super::*;

    fn create_test_embedding(values: Vec<f32>) -> Embedding {
        Embedding::new(values)
    }

    #[test]
    fn test_index_insert_and_get() {
        let index = VectorIndex::new();
        let emb = create_test_embedding(vec![1.0, 0.0, 0.0]);

        index.insert("test".to_string(), emb.clone());

        let retrieved = index.get("test").unwrap();
        assert_eq!(retrieved.dimension, emb.dimension);
    }

    #[test]
    fn test_index_remove() {
        let index = VectorIndex::new();
        let emb = create_test_embedding(vec![1.0, 0.0, 0.0]);

        index.insert("test".to_string(), emb);
        assert!(index.get("test").is_some());

        index.remove("test");
        assert!(index.get("test").is_none());
    }

    #[test]
    fn test_index_search() {
        let index = VectorIndex::new();

        // Insert some vectors
        index.insert("a".to_string(), create_test_embedding(vec![1.0, 0.0, 0.0]));
        index.insert("b".to_string(), create_test_embedding(vec![0.9, 0.1, 0.0]));
        index.insert("c".to_string(), create_test_embedding(vec![0.0, 1.0, 0.0]));

        // Search for vector similar to "a"
        let query = create_test_embedding(vec![1.0, 0.0, 0.0]);
        let results = index.search(&query, 2, 0.0);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a");
        assert!((results[0].score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_index_search_with_min_score() {
        let index = VectorIndex::new();

        index.insert("a".to_string(), create_test_embedding(vec![1.0, 0.0, 0.0]));
        index.insert("b".to_string(), create_test_embedding(vec![0.0, 1.0, 0.0]));

        let query = create_test_embedding(vec![1.0, 0.0, 0.0]);
        let results = index.search(&query, 10, 0.5);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "a");
    }

    #[test]
    fn test_index_len() {
        let index = VectorIndex::new();
        assert!(index.is_empty());

        index.insert("a".to_string(), create_test_embedding(vec![1.0]));
        assert_eq!(index.len(), 1);

        index.insert("b".to_string(), create_test_embedding(vec![1.0]));
        assert_eq!(index.len(), 2);
    }

    #[test]
    fn test_index_clear() {
        let index = VectorIndex::new();
        index.insert("a".to_string(), create_test_embedding(vec![1.0]));
        index.insert("b".to_string(), create_test_embedding(vec![1.0]));

        index.clear();
        assert!(index.is_empty());
    }

    #[test]
    fn test_index_default() {
        let index = VectorIndex::default();
        assert!(index.is_empty());
    }

    #[test]
    fn test_get_nonexistent() {
        let index = VectorIndex::new();
        assert!(index.get("nonexistent").is_none());
    }

    #[test]
    fn test_remove_nonexistent() {
        let index = VectorIndex::new();
        let result = index.remove("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_search_empty_index() {
        let index = VectorIndex::new();
        let query = create_test_embedding(vec![1.0, 0.0, 0.0]);
        let results = index.search(&query, 10, 0.0);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_limit_truncation() {
        let index = VectorIndex::new();
        for i in 0..10 {
            index.insert(format!("item-{}", i), create_test_embedding(vec![1.0, 0.0, 0.0]));
        }

        let query = create_test_embedding(vec![1.0, 0.0, 0.0]);
        let results = index.search(&query, 3, 0.0);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_result_debug() {
        let result = SearchResult {
            id: "test".to_string(),
            score: 0.95,
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("SearchResult"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_search_result_clone() {
        let result = SearchResult {
            id: "test".to_string(),
            score: 0.85,
        };
        let cloned = result.clone();
        assert_eq!(cloned.id, result.id);
        assert_eq!(cloned.score, result.score);
    }

    #[test]
    fn test_insert_overwrite() {
        let index = VectorIndex::new();
        index.insert("same-id".to_string(), create_test_embedding(vec![1.0, 0.0]));
        index.insert("same-id".to_string(), create_test_embedding(vec![0.0, 1.0]));

        assert_eq!(index.len(), 1);
        let retrieved = index.get("same-id").unwrap();
        assert_eq!(retrieved.vector, vec![0.0, 1.0]);
    }

    #[test]
    fn test_search_returns_sorted() {
        let index = VectorIndex::new();
        index.insert("low".to_string(), create_test_embedding(vec![0.0, 1.0, 0.0]));
        index.insert("high".to_string(), create_test_embedding(vec![1.0, 0.0, 0.0]));
        index.insert("medium".to_string(), create_test_embedding(vec![0.7, 0.3, 0.0]));

        let query = create_test_embedding(vec![1.0, 0.0, 0.0]);
        let results = index.search(&query, 3, 0.0);

        // Should be sorted by score descending
        assert!(results[0].score >= results[1].score);
        assert!(results[1].score >= results[2].score);
    }
}
