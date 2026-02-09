//! Web tools extension definition.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::tools::{WebFetchTool, WebSearchTool};

/// Web tools extension.
pub struct WebToolsExtension {
    manifest: ExtensionManifest,
}

impl WebToolsExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "tools-web",
            "Web Tools",
            Version::new(0, 1, 0),
        );
        manifest.description = "Web fetch and search tools".to_string();
        manifest.provides = Provides {
            tools: vec!["web_fetch".to_string(), "web_search".to_string()],
            ..Default::default()
        };

        Self { manifest }
    }
}

impl Default for WebToolsExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for WebToolsExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        // Register tools
        ctx.tool_registry
            .register_tool(Arc::new(WebFetchTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(WebSearchTool::new()))?;

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
        let ext = WebToolsExtension::new();
        assert_eq!(ext.manifest().id, "tools-web");
        assert!(ext.manifest().provides.tools.contains(&"web_fetch".to_string()));
        assert!(ext.manifest().provides.tools.contains(&"web_search".to_string()));
    }

    #[test]
    fn test_extension_default() {
        let ext = WebToolsExtension::default();
        assert_eq!(ext.manifest().id, "tools-web");
    }

    #[test]
    fn test_extension_manifest_name() {
        let ext = WebToolsExtension::new();
        assert_eq!(ext.manifest().name, "Web Tools");
    }

    #[test]
    fn test_extension_manifest_description() {
        let ext = WebToolsExtension::new();
        assert!(ext.manifest().description.contains("Web"));
    }

    #[test]
    fn test_extension_manifest_version() {
        let ext = WebToolsExtension::new();
        assert_eq!(ext.manifest().version.major, 0);
        assert_eq!(ext.manifest().version.minor, 1);
        assert_eq!(ext.manifest().version.patch, 0);
    }

    #[test]
    fn test_extension_as_any() {
        let ext = WebToolsExtension::new();
        let any = ext.as_any();
        assert!(any.is::<WebToolsExtension>());
    }

    #[test]
    fn test_extension_as_any_mut() {
        let mut ext = WebToolsExtension::new();
        let any_mut = ext.as_any_mut();
        assert!(any_mut.is::<WebToolsExtension>());
    }

    #[test]
    fn test_extension_provides_count() {
        let ext = WebToolsExtension::new();
        assert_eq!(ext.manifest().provides.tools.len(), 2);
    }
}
