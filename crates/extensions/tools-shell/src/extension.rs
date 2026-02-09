//! Shell extension definition.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::background::BackgroundManager;
use crate::background_tool::BackgroundTool;
use crate::exec::ExecTool;
use crate::session::SessionManager;
use crate::session_tool::SessionTool;

/// Shell extension providing command execution tools.
pub struct ShellExtension {
    manifest: ExtensionManifest,
    session_manager: Arc<SessionManager>,
    background_manager: Arc<BackgroundManager>,
}

impl ShellExtension {
    pub fn new() -> Self {
        let mut manifest =
            ExtensionManifest::new("tools-shell", "Shell Tools", Version::new(0, 1, 0));
        manifest.description =
            "Shell command execution, persistent sessions, and background processes".to_string();
        manifest.provides = Provides {
            tools: vec![
                "exec".to_string(),
                "shell_session".to_string(),
                "background".to_string(),
            ],
            ..Default::default()
        };

        Self {
            manifest,
            session_manager: Arc::new(SessionManager::new()),
            background_manager: Arc::new(BackgroundManager::new()),
        }
    }
}

impl Default for ShellExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for ShellExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        ctx.tool_registry.register_tool(Arc::new(ExecTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(SessionTool::new(self.session_manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(BackgroundTool::new(self.background_manager.clone())))?;
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
        let ext = ShellExtension::new();
        assert_eq!(ext.manifest().id, "tools-shell");
        assert_eq!(ext.manifest().provides.tools.len(), 3);
        assert!(ext.manifest().provides.tools.contains(&"exec".to_string()));
        assert!(ext.manifest().provides.tools.contains(&"shell_session".to_string()));
        assert!(ext.manifest().provides.tools.contains(&"background".to_string()));
    }

    #[test]
    fn test_default() {
        let ext = ShellExtension::default();
        assert_eq!(ext.manifest().id, "tools-shell");
    }

    #[test]
    fn test_manifest_name() {
        let ext = ShellExtension::new();
        assert_eq!(ext.manifest().name, "Shell Tools");
    }

    #[test]
    fn test_manifest_description() {
        let ext = ShellExtension::new();
        assert!(ext.manifest().description.contains("Shell"));
        assert!(ext.manifest().description.contains("execution"));
    }

    #[test]
    fn test_manifest_version() {
        let ext = ShellExtension::new();
        assert_eq!(ext.manifest().version, Version::new(0, 1, 0));
    }

    #[test]
    fn test_as_any() {
        let ext = ShellExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<ShellExtension>().is_some());
    }

    #[test]
    fn test_as_any_mut() {
        let mut ext = ShellExtension::new();
        let any_ref = ext.as_any_mut();
        assert!(any_ref.downcast_mut::<ShellExtension>().is_some());
    }

    #[test]
    fn test_provides_no_providers() {
        let ext = ShellExtension::new();
        assert!(ext.manifest().provides.providers.is_empty());
    }

    #[test]
    fn test_provides_no_memory_backends() {
        let ext = ShellExtension::new();
        assert!(ext.manifest().provides.memory_backends.is_empty());
    }
}
