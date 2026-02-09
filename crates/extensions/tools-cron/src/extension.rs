//! Cron tools extension definition.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::tools::{CronCreateTool, CronDeleteTool, CronListTool, CronStatusTool};

/// Cron tools extension providing cron job management for agents.
pub struct CronToolsExtension {
    manifest: ExtensionManifest,
}

impl CronToolsExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "tools-cron",
            "Cron Tools",
            Version::new(0, 1, 0),
        );
        manifest.description =
            "Agent-managed scheduled tasks: create, list, delete, and status".to_string();
        manifest.provides = Provides {
            tools: vec![
                "cron_create".to_string(),
                "cron_list".to_string(),
                "cron_delete".to_string(),
                "cron_status".to_string(),
            ],
            ..Default::default()
        };

        Self { manifest }
    }
}

impl Default for CronToolsExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for CronToolsExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        // Register tools
        ctx.tool_registry
            .register_tool(Arc::new(CronCreateTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(CronListTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(CronDeleteTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(CronStatusTool::new()))?;

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
        let ext = CronToolsExtension::new();
        assert_eq!(ext.manifest().id, "tools-cron");
        assert_eq!(ext.manifest().name, "Cron Tools");
        assert!(ext.manifest().description.contains("scheduled tasks"));
    }

    #[test]
    fn test_extension_default() {
        let ext = CronToolsExtension::default();
        assert_eq!(ext.manifest().id, "tools-cron");
    }

    #[test]
    fn test_extension_provides_tools() {
        let ext = CronToolsExtension::new();
        let tools = &ext.manifest().provides.tools;

        assert_eq!(tools.len(), 4);
        assert!(tools.contains(&"cron_create".to_string()));
        assert!(tools.contains(&"cron_list".to_string()));
        assert!(tools.contains(&"cron_delete".to_string()));
        assert!(tools.contains(&"cron_status".to_string()));
    }

    #[test]
    fn test_as_any() {
        let ext = CronToolsExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<CronToolsExtension>().is_some());
    }
}
