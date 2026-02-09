//! MCP Bridge extension definition.

use std::any::Any;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

/// MCP Bridge extension.
pub struct McpBridgeExtension {
    manifest: ExtensionManifest,
}

impl McpBridgeExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "mcp-bridge",
            "MCP Bridge",
            Version::new(0, 1, 0),
        );
        manifest.description = "Bridge to MCP (Model Context Protocol) servers".to_string();
        manifest.provides = Provides {
            ..Default::default()
        };

        Self { manifest }
    }
}

impl Default for McpBridgeExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for McpBridgeExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, _ctx: ExtensionContext) -> Result<(), ExtensionError> {
        // MCP servers would be configured and connected here
        // Tools from MCP servers would be registered with the tool registry
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
        let ext = McpBridgeExtension::new();
        assert_eq!(ext.manifest().id, "mcp-bridge");
        assert_eq!(ext.manifest().name, "MCP Bridge");
    }

    #[test]
    fn test_extension_default() {
        let ext = McpBridgeExtension::default();
        assert_eq!(ext.manifest().id, "mcp-bridge");
    }

    #[test]
    fn test_manifest_description() {
        let ext = McpBridgeExtension::new();
        assert_eq!(ext.manifest().description, "Bridge to MCP (Model Context Protocol) servers");
    }

    #[test]
    fn test_manifest_version() {
        let ext = McpBridgeExtension::new();
        assert_eq!(ext.manifest().version, Version::new(0, 1, 0));
    }

    #[test]
    fn test_as_any() {
        let ext = McpBridgeExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<McpBridgeExtension>().is_some());
    }

    #[test]
    fn test_as_any_mut() {
        let mut ext = McpBridgeExtension::new();
        let any_ref = ext.as_any_mut();
        assert!(any_ref.downcast_mut::<McpBridgeExtension>().is_some());
    }

    #[test]
    fn test_provides_defaults() {
        let ext = McpBridgeExtension::new();
        assert!(ext.manifest().provides.tools.is_empty());
        assert!(ext.manifest().provides.providers.is_empty());
        assert!(ext.manifest().provides.memory_backends.is_empty());
    }
}
