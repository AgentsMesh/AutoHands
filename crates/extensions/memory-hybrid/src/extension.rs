//! Hybrid memory extension.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_memory_vector::EmbeddingProvider;
use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::backend::{HybridMemoryBackend, HybridMemoryConfig};
use crate::embedding::OpenAIEmbedding;
use crate::fusion::FusionConfig;

/// Configuration for the hybrid memory extension.
pub struct HybridMemoryExtensionConfig {
    /// ID for the memory backend.
    pub id: String,
    /// Embedding provider.
    pub embedder: Arc<dyn EmbeddingProvider>,
    /// Hybrid search configuration.
    pub config: HybridMemoryConfig,
    /// Optional path for FTS database.
    pub fts_path: Option<std::path::PathBuf>,
}

impl HybridMemoryExtensionConfig {
    /// Create config with OpenAI embeddings.
    pub fn with_openai(api_key: impl Into<String>) -> Self {
        let embedder = Arc::new(OpenAIEmbedding::from_api_key(api_key));
        Self {
            id: "hybrid".to_string(),
            embedder,
            config: HybridMemoryConfig::default(),
            fts_path: None,
        }
    }

    /// Create with custom embedding provider.
    pub fn with_embedder(embedder: Arc<dyn EmbeddingProvider>) -> Self {
        Self {
            id: "hybrid".to_string(),
            embedder,
            config: HybridMemoryConfig::default(),
            fts_path: None,
        }
    }

    /// Set the backend ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set fusion configuration.
    pub fn fusion(mut self, fusion: FusionConfig) -> Self {
        self.config.fusion = fusion;
        self
    }

    /// Favor semantic (vector) search results.
    pub fn favor_semantic(mut self) -> Self {
        self.config.fusion = FusionConfig::favor_semantic();
        self
    }

    /// Favor keyword search results.
    pub fn favor_keyword(mut self) -> Self {
        self.config.fusion = FusionConfig::favor_keyword();
        self
    }

    /// Set minimum relevance threshold.
    pub fn min_relevance(mut self, threshold: f32) -> Self {
        self.config.min_relevance = threshold;
        self
    }

    /// Set path for FTS database persistence.
    pub fn fts_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.fts_path = Some(path.into());
        self
    }
}

/// Hybrid memory extension.
pub struct HybridMemoryExtension {
    manifest: ExtensionManifest,
    extension_config: Option<HybridMemoryExtensionConfig>,
    backend: Option<Arc<HybridMemoryBackend>>,
}

impl HybridMemoryExtension {
    /// Create a new hybrid memory extension.
    ///
    /// Note: You must call `with_config` before initialization.
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "memory-hybrid",
            "Hybrid Memory",
            Version::new(0, 1, 0),
        );
        manifest.description =
            "Hybrid memory backend combining vector and keyword search".to_string();
        manifest.provides = Provides {
            memory_backends: vec!["hybrid".to_string()],
            ..Default::default()
        };

        Self {
            manifest,
            extension_config: None,
            backend: None,
        }
    }

    /// Configure the extension.
    pub fn with_config(mut self, config: HybridMemoryExtensionConfig) -> Self {
        self.extension_config = Some(config);
        self
    }

    /// Get the backend after initialization.
    pub fn backend(&self) -> Option<Arc<HybridMemoryBackend>> {
        self.backend.clone()
    }
}

impl Default for HybridMemoryExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for HybridMemoryExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        let config = self.extension_config.take().ok_or_else(|| {
            ExtensionError::InitializationFailed(
                "HybridMemoryExtension requires configuration via with_config()".to_string(),
            )
        })?;

        let backend = if let Some(fts_path) = config.fts_path {
            HybridMemoryBackend::with_fts_path(&config.id, config.embedder, fts_path, config.config)
                .await
        } else {
            HybridMemoryBackend::new(&config.id, config.embedder, config.config).await
        };

        let backend = Arc::new(
            backend.map_err(|e| ExtensionError::InitializationFailed(e.to_string()))?,
        );

        // Register the backend
        ctx.memory_registry.register_backend(backend.clone())?;

        self.backend = Some(backend);

        tracing::info!("Hybrid memory extension initialized");
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
#[path = "extension_tests.rs"]
mod tests;
