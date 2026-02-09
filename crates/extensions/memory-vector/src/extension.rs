//! Vector memory extension.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::backend::VectorMemoryBackend;
use crate::embedding::SimpleHashEmbedding;

/// Vector memory extension for semantic search.
pub struct VectorMemoryExtension {
    manifest: ExtensionManifest,
    dimension: usize,
}

impl VectorMemoryExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "memory-vector",
            "Vector Memory",
            Version::new(0, 1, 0),
        );
        manifest.description = "Vector-based memory with semantic search".to_string();
        manifest.provides = Provides {
            memory_backends: vec!["vector".to_string()],
            ..Default::default()
        };

        Self {
            manifest,
            dimension: 128,
        }
    }

    /// Set the embedding dimension.
    pub fn with_dimension(mut self, dimension: usize) -> Self {
        self.dimension = dimension;
        self
    }
}

impl Default for VectorMemoryExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for VectorMemoryExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        let embedder = Arc::new(SimpleHashEmbedding::new(self.dimension));
        let backend = VectorMemoryBackend::new("vector", embedder);

        ctx.memory_registry
            .register_backend(Arc::new(backend))
            .map_err(|e| ExtensionError::InitializationFailed(e.to_string()))?;

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
mod tests {
    use super::*;

    #[test]
    fn test_extension_manifest() {
        let ext = VectorMemoryExtension::new();
        assert_eq!(ext.manifest().id, "memory-vector");
        assert!(ext
            .manifest()
            .provides
            .memory_backends
            .contains(&"vector".to_string()));
    }

    #[test]
    fn test_with_dimension() {
        let ext = VectorMemoryExtension::new().with_dimension(256);
        assert_eq!(ext.dimension, 256);
    }

    #[test]
    fn test_extension_default() {
        let ext = VectorMemoryExtension::default();
        assert_eq!(ext.manifest().id, "memory-vector");
        assert_eq!(ext.dimension, 128);
    }

    #[test]
    fn test_manifest_name() {
        let ext = VectorMemoryExtension::new();
        assert_eq!(ext.manifest().name, "Vector Memory");
    }

    #[test]
    fn test_manifest_description() {
        let ext = VectorMemoryExtension::new();
        assert_eq!(ext.manifest().description, "Vector-based memory with semantic search");
    }

    #[test]
    fn test_manifest_version() {
        let ext = VectorMemoryExtension::new();
        assert_eq!(ext.manifest().version, Version::new(0, 1, 0));
    }

    #[test]
    fn test_as_any() {
        let ext = VectorMemoryExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<VectorMemoryExtension>().is_some());
    }

    #[test]
    fn test_as_any_mut() {
        let mut ext = VectorMemoryExtension::new();
        let any_ref = ext.as_any_mut();
        assert!(any_ref.downcast_mut::<VectorMemoryExtension>().is_some());
    }

    #[test]
    fn test_with_dimension_builder_chain() {
        let ext = VectorMemoryExtension::new()
            .with_dimension(64);
        assert_eq!(ext.dimension, 64);
        assert_eq!(ext.manifest().id, "memory-vector");
    }

    #[test]
    fn test_multiple_dimension_changes() {
        let ext = VectorMemoryExtension::new()
            .with_dimension(128)
            .with_dimension(512);
        assert_eq!(ext.dimension, 512);
    }

    #[test]
    fn test_default_dimension() {
        let ext = VectorMemoryExtension::new();
        assert_eq!(ext.dimension, 128);
    }
}
