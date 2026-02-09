//! Search extension definition.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::{GlobTool, GrepTool};

/// Search extension providing glob and grep tools.
pub struct SearchExtension {
    manifest: ExtensionManifest,
}

impl SearchExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "tools-search",
            "Search Tools",
            Version::new(0, 1, 0),
        );
        manifest.description = "File pattern matching and content search".to_string();
        manifest.provides = Provides {
            tools: vec!["glob".to_string(), "grep".to_string()],
            ..Default::default()
        };

        Self { manifest }
    }
}

impl Default for SearchExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for SearchExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        ctx.tool_registry.register_tool(Arc::new(GlobTool::new()))?;
        ctx.tool_registry.register_tool(Arc::new(GrepTool::new()))?;
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
        let ext = SearchExtension::new();
        assert_eq!(ext.manifest().id, "tools-search");
        assert!(ext.manifest().provides.tools.contains(&"glob".to_string()));
        assert!(ext.manifest().provides.tools.contains(&"grep".to_string()));
    }

    #[test]
    fn test_extension_default() {
        let ext = SearchExtension::default();
        assert_eq!(ext.manifest().id, "tools-search");
    }

    #[test]
    fn test_extension_manifest_name() {
        let ext = SearchExtension::new();
        assert_eq!(ext.manifest().name, "Search Tools");
    }

    #[test]
    fn test_extension_manifest_description() {
        let ext = SearchExtension::new();
        assert!(ext.manifest().description.contains("search"));
    }

    #[test]
    fn test_extension_manifest_version() {
        let ext = SearchExtension::new();
        assert_eq!(ext.manifest().version.major, 0);
        assert_eq!(ext.manifest().version.minor, 1);
        assert_eq!(ext.manifest().version.patch, 0);
    }

    #[test]
    fn test_extension_as_any() {
        let ext = SearchExtension::new();
        let any = ext.as_any();
        assert!(any.is::<SearchExtension>());
    }

    #[test]
    fn test_extension_as_any_mut() {
        let mut ext = SearchExtension::new();
        let any_mut = ext.as_any_mut();
        assert!(any_mut.is::<SearchExtension>());
    }

    #[test]
    fn test_extension_provides_count() {
        let ext = SearchExtension::new();
        assert_eq!(ext.manifest().provides.tools.len(), 2);
    }
}
