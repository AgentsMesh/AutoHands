//! Filesystem extension definition.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::tools::{
    CreateDirectoryTool, DeleteFileTool, EditFileTool, ListDirectoryTool, MoveFileTool,
    ReadFileTool, WriteFileTool,
};

/// Filesystem extension providing file operation tools.
pub struct FilesystemExtension {
    manifest: ExtensionManifest,
}

impl FilesystemExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "tools-filesystem",
            "Filesystem Tools",
            Version::new(0, 1, 0),
        );
        manifest.description =
            "File system operations: read, write, edit, list, create, delete, move".to_string();
        manifest.provides = Provides {
            tools: vec![
                "read_file".to_string(),
                "write_file".to_string(),
                "edit_file".to_string(),
                "list_directory".to_string(),
                "create_directory".to_string(),
                "delete_file".to_string(),
                "move_file".to_string(),
            ],
            ..Default::default()
        };

        Self { manifest }
    }
}

impl Default for FilesystemExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for FilesystemExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        // Register tools
        ctx.tool_registry
            .register_tool(Arc::new(ReadFileTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(WriteFileTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(EditFileTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(ListDirectoryTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(CreateDirectoryTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(DeleteFileTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(MoveFileTool::new()))?;

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
        let ext = FilesystemExtension::new();
        assert_eq!(ext.manifest().id, "tools-filesystem");
        assert_eq!(ext.manifest().name, "Filesystem Tools");
        assert!(ext.manifest().description.contains("File system operations"));
    }

    #[test]
    fn test_extension_default() {
        let ext = FilesystemExtension::default();
        assert_eq!(ext.manifest().id, "tools-filesystem");
    }

    #[test]
    fn test_extension_provides_tools() {
        let ext = FilesystemExtension::new();
        let tools = &ext.manifest().provides.tools;

        assert_eq!(tools.len(), 7);
        assert!(tools.contains(&"read_file".to_string()));
        assert!(tools.contains(&"write_file".to_string()));
        assert!(tools.contains(&"edit_file".to_string()));
        assert!(tools.contains(&"list_directory".to_string()));
        assert!(tools.contains(&"create_directory".to_string()));
        assert!(tools.contains(&"delete_file".to_string()));
        assert!(tools.contains(&"move_file".to_string()));
    }

    #[test]
    fn test_extension_version() {
        let ext = FilesystemExtension::new();
        assert_eq!(ext.manifest().version.major, 0);
        assert_eq!(ext.manifest().version.minor, 1);
        assert_eq!(ext.manifest().version.patch, 0);
    }

    #[test]
    fn test_as_any() {
        let ext = FilesystemExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<FilesystemExtension>().is_some());
    }

    #[test]
    fn test_as_any_mut() {
        let mut ext = FilesystemExtension::new();
        let any_ref = ext.as_any_mut();
        assert!(any_ref.downcast_mut::<FilesystemExtension>().is_some());
    }

    #[test]
    fn test_provides_no_providers() {
        let ext = FilesystemExtension::new();
        assert!(ext.manifest().provides.providers.is_empty());
    }

    #[test]
    fn test_provides_no_memory_backends() {
        let ext = FilesystemExtension::new();
        assert!(ext.manifest().provides.memory_backends.is_empty());
    }
}
