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
#[path = "embedding_tests.rs"]
mod tests;
