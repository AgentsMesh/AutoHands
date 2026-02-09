//! SQLite memory extension definition.

use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{
    Extension, ExtensionContext, ExtensionManifest, Provides,
};
use autohands_protocols::types::Version;

use crate::SqliteMemoryBackend;

/// SQLite memory extension.
pub struct SqliteMemoryExtension {
    manifest: ExtensionManifest,
    db_path: Option<PathBuf>,
}

impl SqliteMemoryExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "memory-sqlite",
            "SQLite Memory",
            Version::new(0, 1, 0),
        );
        manifest.description = "SQLite-based memory storage".to_string();
        manifest.provides = Provides {
            memory_backends: vec!["sqlite".to_string()],
            ..Default::default()
        };

        Self {
            manifest,
            db_path: None,
        }
    }

    /// Use in-memory database (default).
    pub fn in_memory(self) -> Self {
        Self {
            db_path: None,
            ..self
        }
    }

    /// Use file-backed database.
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.db_path = Some(path.into());
        self
    }
}

impl Default for SqliteMemoryExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for SqliteMemoryExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        let backend = if let Some(path) = &self.db_path {
            SqliteMemoryBackend::open(path)
                .await
                .map_err(|e| ExtensionError::InitializationFailed(e.to_string()))?
        } else {
            SqliteMemoryBackend::in_memory()
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
        let ext = SqliteMemoryExtension::new();
        assert_eq!(ext.manifest().id, "memory-sqlite");
        assert!(ext.manifest().provides.memory_backends.contains(&"sqlite".to_string()));
    }

    #[test]
    fn test_with_path() {
        let ext = SqliteMemoryExtension::new().with_path("/tmp/test.db");
        assert_eq!(ext.db_path, Some(PathBuf::from("/tmp/test.db")));
    }

    #[test]
    fn test_extension_default() {
        let ext = SqliteMemoryExtension::default();
        assert_eq!(ext.manifest().id, "memory-sqlite");
        assert!(ext.db_path.is_none());
    }

    #[test]
    fn test_manifest_name() {
        let ext = SqliteMemoryExtension::new();
        assert_eq!(ext.manifest().name, "SQLite Memory");
    }

    #[test]
    fn test_manifest_description() {
        let ext = SqliteMemoryExtension::new();
        assert_eq!(ext.manifest().description, "SQLite-based memory storage");
    }

    #[test]
    fn test_manifest_version() {
        let ext = SqliteMemoryExtension::new();
        assert_eq!(ext.manifest().version, Version::new(0, 1, 0));
    }

    #[test]
    fn test_in_memory() {
        let ext = SqliteMemoryExtension::new().in_memory();
        assert!(ext.db_path.is_none());
    }

    #[test]
    fn test_with_path_string() {
        let ext = SqliteMemoryExtension::new().with_path(String::from("/data/memory.db"));
        assert_eq!(ext.db_path, Some(PathBuf::from("/data/memory.db")));
    }

    #[test]
    fn test_as_any() {
        let ext = SqliteMemoryExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<SqliteMemoryExtension>().is_some());
    }

    #[test]
    fn test_as_any_mut() {
        let mut ext = SqliteMemoryExtension::new();
        let any_ref = ext.as_any_mut();
        assert!(any_ref.downcast_mut::<SqliteMemoryExtension>().is_some());
    }

    #[test]
    fn test_builder_chain_with_path() {
        let ext = SqliteMemoryExtension::new()
            .with_path("/first/path.db")
            .with_path("/second/path.db");
        assert_eq!(ext.db_path, Some(PathBuf::from("/second/path.db")));
    }

    #[test]
    fn test_in_memory_clears_path() {
        let ext = SqliteMemoryExtension::new()
            .with_path("/tmp/test.db")
            .in_memory();
        assert!(ext.db_path.is_none());
    }
}
