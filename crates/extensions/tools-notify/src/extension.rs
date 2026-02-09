//! Notify tools extension definition.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::tools::NotifySendTool;

/// Notify tools extension providing notification capabilities for agents.
pub struct NotifyToolsExtension {
    manifest: ExtensionManifest,
}

impl NotifyToolsExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "tools-notify",
            "Notify Tools",
            Version::new(0, 1, 0),
        );
        manifest.description =
            "Agent notification capabilities: send messages via various channels".to_string();
        manifest.provides = Provides {
            tools: vec!["notify_send".to_string()],
            ..Default::default()
        };

        Self { manifest }
    }
}

impl Default for NotifyToolsExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for NotifyToolsExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        // Register tools
        ctx.tool_registry
            .register_tool(Arc::new(NotifySendTool::new()))?;

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
        let ext = NotifyToolsExtension::new();
        assert_eq!(ext.manifest().id, "tools-notify");
        assert_eq!(ext.manifest().name, "Notify Tools");
        assert!(ext.manifest().description.contains("notification"));
    }

    #[test]
    fn test_extension_default() {
        let ext = NotifyToolsExtension::default();
        assert_eq!(ext.manifest().id, "tools-notify");
    }

    #[test]
    fn test_extension_provides_tools() {
        let ext = NotifyToolsExtension::new();
        let tools = &ext.manifest().provides.tools;

        assert_eq!(tools.len(), 1);
        assert!(tools.contains(&"notify_send".to_string()));
    }

    #[test]
    fn test_as_any() {
        let ext = NotifyToolsExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<NotifyToolsExtension>().is_some());
    }
}
