//! Embedding generation utilities.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Error type for embedding operations.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Embedding failed: {0}")]
    Failed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Embedding result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// Vector representation.
    pub vector: Vec<f32>,
    /// Dimension of the embedding.
    pub dimension: usize,
}

impl Embedding {
    pub fn new(vector: Vec<f32>) -> Self {
        let dimension = vector.len();
        Self { vector, dimension }
    }

    /// Compute cosine similarity with another embedding.
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        if self.dimension != other.dimension {
            return 0.0;
        }

        let dot: f32 = self
            .vector
            .iter()
            .zip(other.vector.iter())
            .map(|(a, b)| a * b)
            .sum();

        let norm_a: f32 = self.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.vector.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }
}

/// Trait for embedding providers.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for text.
    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError>;

    /// Generate embeddings for multiple texts.
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError>;

    /// Get the embedding dimension.
    fn dimension(&self) -> usize;
}

/// Simple hash-based embedding for testing (not semantic).
pub struct SimpleHashEmbedding {
    dimension: usize,
}

impl SimpleHashEmbedding {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    fn hash_text(&self, text: &str) -> Embedding {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut vector = vec![0.0f32; self.dimension];

        for (i, word) in text.split_whitespace().enumerate() {
            let mut hasher = DefaultHasher::new();
            word.to_lowercase().hash(&mut hasher);
            let hash = hasher.finish();

            // Distribute hash across vector dimensions
            for j in 0..self.dimension {
                let idx = (i + j) % self.dimension;
                let val = ((hash >> (j % 64)) & 0xFF) as f32 / 255.0 - 0.5;
                vector[idx] += val;
            }
        }

        // Normalize
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vector {
                *v /= norm;
            }
        }

        Embedding::new(vector)
    }
}

impl Default for SimpleHashEmbedding {
    fn default() -> Self {
        Self::new(128)
    }
}

#[async_trait]
impl EmbeddingProvider for SimpleHashEmbedding {
    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        Ok(self.hash_text(text))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        Ok(texts.iter().map(|t| self.hash_text(t)).collect())
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_creation() {
        let emb = Embedding::new(vec![0.5, 0.5, 0.0, 0.0]);
        assert_eq!(emb.dimension, 4);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let emb1 = Embedding::new(vec![1.0, 0.0, 0.0]);
        let emb2 = Embedding::new(vec![1.0, 0.0, 0.0]);
        let sim = emb1.cosine_similarity(&emb2);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let emb1 = Embedding::new(vec![1.0, 0.0, 0.0]);
        let emb2 = Embedding::new(vec![0.0, 1.0, 0.0]);
        let sim = emb1.cosine_similarity(&emb2);
        assert!(sim.abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let emb1 = Embedding::new(vec![1.0, 0.0]);
        let emb2 = Embedding::new(vec![-1.0, 0.0]);
        let sim = emb1.cosine_similarity(&emb2);
        assert!((sim + 1.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_simple_hash_embedding() {
        let provider = SimpleHashEmbedding::new(64);
        let emb = provider.embed("hello world").await.unwrap();
        assert_eq!(emb.dimension, 64);
    }

    #[tokio::test]
    async fn test_similar_texts_similar_embeddings() {
        let provider = SimpleHashEmbedding::new(128);
        let emb1 = provider.embed("hello world").await.unwrap();
        let emb2 = provider.embed("hello world").await.unwrap();
        let emb3 = provider.embed("goodbye moon").await.unwrap();

        // Identical texts should have similarity 1.0
        assert!((emb1.cosine_similarity(&emb2) - 1.0).abs() < 0.001);
        // Different texts should have lower similarity
        assert!(emb1.cosine_similarity(&emb3) < 0.9);
    }

    #[tokio::test]
    async fn test_batch_embedding() {
        let provider = SimpleHashEmbedding::new(64);
        let texts = &["hello", "world", "test"];
        let embeddings = provider.embed_batch(texts).await.unwrap();
        assert_eq!(embeddings.len(), 3);
    }

    #[test]
    fn test_cosine_similarity_different_dimensions() {
        let emb1 = Embedding::new(vec![1.0, 0.0, 0.0]);
        let emb2 = Embedding::new(vec![1.0, 0.0]); // Different dimension
        let sim = emb1.cosine_similarity(&emb2);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let emb1 = Embedding::new(vec![1.0, 0.0, 0.0]);
        let emb2 = Embedding::new(vec![0.0, 0.0, 0.0]); // Zero vector
        let sim = emb1.cosine_similarity(&emb2);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_embedding_error_display() {
        let err = EmbeddingError::Failed("test error".to_string());
        assert_eq!(err.to_string(), "Embedding failed: test error");

        let err = EmbeddingError::InvalidInput("bad input".to_string());
        assert_eq!(err.to_string(), "Invalid input: bad input");
    }

    #[test]
    fn test_embedding_error_debug() {
        let err = EmbeddingError::Failed("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Failed"));
    }

    #[test]
    fn test_embedding_serialization() {
        let emb = Embedding::new(vec![0.1, 0.2, 0.3]);
        let json = serde_json::to_string(&emb).unwrap();
        assert!(json.contains("0.1"));
        assert!(json.contains("dimension"));
    }

    #[test]
    fn test_embedding_deserialization() {
        let json = r#"{"vector":[0.5,0.5],"dimension":2}"#;
        let emb: Embedding = serde_json::from_str(json).unwrap();
        assert_eq!(emb.dimension, 2);
        assert_eq!(emb.vector, vec![0.5, 0.5]);
    }

    #[test]
    fn test_embedding_clone() {
        let emb = Embedding::new(vec![0.1, 0.2, 0.3]);
        let cloned = emb.clone();
        assert_eq!(cloned.vector, emb.vector);
        assert_eq!(cloned.dimension, emb.dimension);
    }

    #[test]
    fn test_simple_hash_embedding_default() {
        let provider = SimpleHashEmbedding::default();
        assert_eq!(provider.dimension(), 128);
    }

    #[test]
    fn test_simple_hash_embedding_dimension() {
        let provider = SimpleHashEmbedding::new(256);
        assert_eq!(provider.dimension(), 256);
    }

    #[tokio::test]
    async fn test_embed_empty_text() {
        let provider = SimpleHashEmbedding::new(64);
        let emb = provider.embed("").await.unwrap();
        assert_eq!(emb.dimension, 64);
    }

    #[tokio::test]
    async fn test_embed_batch_empty() {
        let provider = SimpleHashEmbedding::new(64);
        let texts: &[&str] = &[];
        let embeddings = provider.embed_batch(texts).await.unwrap();
        assert!(embeddings.is_empty());
    }

    #[test]
    fn test_cosine_similarity_both_zero() {
        let emb1 = Embedding::new(vec![0.0, 0.0, 0.0]);
        let emb2 = Embedding::new(vec![0.0, 0.0, 0.0]);
        let sim = emb1.cosine_similarity(&emb2);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_embedding_debug() {
        let emb = Embedding::new(vec![0.1, 0.2]);
        let debug = format!("{:?}", emb);
        assert!(debug.contains("Embedding"));
    }
}
