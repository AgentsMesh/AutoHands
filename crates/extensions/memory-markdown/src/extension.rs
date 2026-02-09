//! Markdown memory extension.

use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::backend::MarkdownMemoryBackend;

/// Markdown memory extension.
pub struct MarkdownMemoryExtension {
    manifest: ExtensionManifest,
    storage_path: Option<PathBuf>,
}

impl MarkdownMemoryExtension {
    /// Create a new extension with default config.
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "memory-markdown",
            "Markdown Memory",
            Version::new(0, 1, 0),
        );
        manifest.description =
            "Persistent memory storage using Markdown files with YAML front matter".to_string();
        manifest.provides = Provides {
            memory_backends: vec!["markdown".to_string()],
            ..Default::default()
        };

        Self {
            manifest,
            storage_path: None,
        }
    }

    /// Use default storage path (~/.autohands/memory/).
    pub fn default_path(self) -> Self {
        Self {
            storage_path: None,
            ..self
        }
    }

    /// Use custom storage path.
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.storage_path = Some(path.into());
        self
    }
}

impl Default for MarkdownMemoryExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for MarkdownMemoryExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        let backend = if let Some(ref path) = self.storage_path {
            MarkdownMemoryBackend::new(path)
                .await
                .map_err(|e| ExtensionError::InitializationFailed(e.to_string()))?
        } else {
            MarkdownMemoryBackend::default_path()
                .await
                .map_err(|e| ExtensionError::InitializationFailed(e.to_string()))?
        };

        ctx.memory_registry.register_backend(Arc::new(backend))?;
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
        let ext = MarkdownMemoryExtension::new();
        assert_eq!(ext.manifest().id, "memory-markdown");
        assert!(ext
            .manifest()
            .provides
            .memory_backends
            .contains(&"markdown".to_string()));
    }

    #[test]
    fn test_default_config() {
        let ext = MarkdownMemoryExtension::default();
        assert!(ext.storage_path.is_none());
    }

    #[test]
    fn test_with_path() {
        let ext = MarkdownMemoryExtension::new().with_path("/tmp/test");
        assert!(ext.storage_path.is_some());
        assert_eq!(ext.storage_path.unwrap(), PathBuf::from("/tmp/test"));
    }

    #[test]
    fn test_manifest_name() {
        let ext = MarkdownMemoryExtension::new();
        assert_eq!(ext.manifest().name, "Markdown Memory");
    }

    #[test]
    fn test_manifest_description() {
        let ext = MarkdownMemoryExtension::new();
        assert!(ext.manifest().description.contains("Markdown"));
    }

    #[test]
    fn test_manifest_version() {
        let ext = MarkdownMemoryExtension::new();
        assert_eq!(ext.manifest().version, Version::new(0, 1, 0));
    }

    #[test]
    fn test_as_any() {
        let ext = MarkdownMemoryExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<MarkdownMemoryExtension>().is_some());
    }

    #[test]
    fn test_as_any_mut() {
        let mut ext = MarkdownMemoryExtension::new();
        let any_ref = ext.as_any_mut();
        assert!(any_ref.downcast_mut::<MarkdownMemoryExtension>().is_some());
    }

    #[test]
    fn test_default_path() {
        let ext = MarkdownMemoryExtension::new()
            .with_path("/custom")
            .default_path();
        assert!(ext.storage_path.is_none());
    }
}
