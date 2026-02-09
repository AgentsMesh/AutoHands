//! OpenAI embedding provider.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use autohands_memory_vector::{Embedding, EmbeddingError, EmbeddingProvider};

/// Configuration for OpenAI embeddings.
#[derive(Debug, Clone)]
pub struct OpenAIEmbeddingConfig {
    /// API key for OpenAI.
    pub api_key: String,
    /// Model to use (default: text-embedding-3-small).
    pub model: String,
    /// Base URL for API (default: https://api.openai.com/v1).
    pub base_url: String,
    /// Embedding dimension (default: 1536 for text-embedding-3-small).
    pub dimension: usize,
}

impl OpenAIEmbeddingConfig {
    /// Create config with API key using defaults.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "text-embedding-3-small".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            dimension: 1536,
        }
    }

    /// Use a different model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set custom base URL (for Azure OpenAI or compatible APIs).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set embedding dimension.
    pub fn with_dimension(mut self, dim: usize) -> Self {
        self.dimension = dim;
        self
    }
}

/// OpenAI embedding provider.
pub struct OpenAIEmbedding {
    client: reqwest::Client,
    config: OpenAIEmbeddingConfig,
}

impl OpenAIEmbedding {
    /// Create a new OpenAI embedding provider.
    pub fn new(config: OpenAIEmbeddingConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    /// Create from API key with defaults.
    pub fn from_api_key(api_key: impl Into<String>) -> Self {
        Self::new(OpenAIEmbeddingConfig::new(api_key))
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbedding {
    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let embeddings = self.embed_batch(&[text]).await?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::Failed("Empty response".to_string()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let request = EmbeddingRequest {
            input: texts.iter().map(|t| t.to_string()).collect(),
            model: self.config.model.clone(),
        };

        let url = format!("{}/embeddings", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| EmbeddingError::Failed(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(EmbeddingError::Failed(format!(
                "API error {}: {}",
                status, body
            )));
        }

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::Failed(format!("Parse error: {}", e)))?;

        debug!(
            "Generated {} embeddings",
            embedding_response.data.len()
        );

        Ok(embedding_response
            .data
            .into_iter()
            .map(|d| Embedding::new(d.embedding))
            .collect())
    }

    fn dimension(&self) -> usize {
        self.config.dimension
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = OpenAIEmbeddingConfig::new("test-key");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.model, "text-embedding-3-small");
        assert_eq!(config.dimension, 1536);
    }

    #[test]
    fn test_config_builder() {
        let config = OpenAIEmbeddingConfig::new("key")
            .with_model("text-embedding-3-large")
            .with_dimension(3072)
            .with_base_url("https://custom.api.com");

        assert_eq!(config.model, "text-embedding-3-large");
        assert_eq!(config.dimension, 3072);
        assert_eq!(config.base_url, "https://custom.api.com");
    }

    #[test]
    fn test_provider_dimension() {
        let provider = OpenAIEmbedding::from_api_key("test-key");
        assert_eq!(provider.dimension(), 1536);
    }

    #[test]
    fn test_config_clone() {
        let config = OpenAIEmbeddingConfig::new("key");
        let cloned = config.clone();
        assert_eq!(cloned.api_key, "key");
    }
}
