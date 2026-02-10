//! Memory tools extension definition.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::memory::MemoryBackend;
use autohands_protocols::types::Version;

use crate::{MemoryGetTool, MemorySearchTool, MemoryStoreTool};

/// Extension that registers memory_search, memory_get, memory_store tools.
pub struct MemoryToolsExtension {
    manifest: ExtensionManifest,
    backend: Arc<dyn MemoryBackend>,
}

impl MemoryToolsExtension {
    /// Create a new MemoryToolsExtension with the given memory backend.
    pub fn new(backend: Arc<dyn MemoryBackend>) -> Self {
        let mut manifest = ExtensionManifest::new(
            "tools-memory",
            "Memory Tools",
            Version::new(0, 1, 0),
        );
        manifest.description =
            "Agent memory tools for searching, retrieving, and storing long-term memories"
                .to_string();
        manifest.provides = Provides {
            tools: vec![
                "memory_search".to_string(),
                "memory_get".to_string(),
                "memory_store".to_string(),
            ],
            ..Default::default()
        };

        Self { manifest, backend }
    }

    /// Get the memory backend (for passing to AgentLoop/AgentRuntime).
    pub fn backend(&self) -> Arc<dyn MemoryBackend> {
        self.backend.clone()
    }
}

#[async_trait]
impl Extension for MemoryToolsExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        ctx.tool_registry
            .register_tool(Arc::new(MemorySearchTool::new(self.backend.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(MemoryGetTool::new(self.backend.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(MemoryStoreTool::new(self.backend.clone())))?;
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
    use autohands_protocols::error::MemoryError;
    use autohands_protocols::memory::{MemoryEntry, MemoryQuery, MemorySearchResult};

    struct MockMemoryBackend;

    #[async_trait]
    impl MemoryBackend for MockMemoryBackend {
        fn id(&self) -> &str {
            "mock"
        }
        async fn store(&self, _entry: MemoryEntry) -> Result<String, MemoryError> {
            Ok("id".to_string())
        }
        async fn retrieve(&self, _id: &str) -> Result<Option<MemoryEntry>, MemoryError> {
            Ok(None)
        }
        async fn search(
            &self,
            _query: MemoryQuery,
        ) -> Result<Vec<MemorySearchResult>, MemoryError> {
            Ok(Vec::new())
        }
        async fn delete(&self, _id: &str) -> Result<(), MemoryError> {
            Ok(())
        }
        async fn update(&self, _id: &str, _entry: MemoryEntry) -> Result<(), MemoryError> {
            Ok(())
        }
    }

    #[test]
    fn test_extension_manifest() {
        let ext = MemoryToolsExtension::new(Arc::new(MockMemoryBackend));
        assert_eq!(ext.manifest().id, "tools-memory");
        assert_eq!(ext.manifest().provides.tools.len(), 3);
        assert!(ext.manifest().provides.tools.contains(&"memory_search".to_string()));
        assert!(ext.manifest().provides.tools.contains(&"memory_get".to_string()));
        assert!(ext.manifest().provides.tools.contains(&"memory_store".to_string()));
    }

    #[test]
    fn test_extension_backend() {
        let backend: Arc<dyn MemoryBackend> = Arc::new(MockMemoryBackend);
        let ext = MemoryToolsExtension::new(backend);
        assert_eq!(ext.backend().id(), "mock");
    }

    #[test]
    fn test_extension_as_any() {
        let ext = MemoryToolsExtension::new(Arc::new(MockMemoryBackend));
        assert!(ext.as_any().is::<MemoryToolsExtension>());
    }

    #[test]
    fn test_extension_as_any_mut() {
        let mut ext = MemoryToolsExtension::new(Arc::new(MockMemoryBackend));
        assert!(ext.as_any_mut().is::<MemoryToolsExtension>());
    }
}
