//! Desktop tools extension.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::tools::*;

/// Desktop tools extension for system-level automation.
pub struct DesktopToolsExtension {
    manifest: ExtensionManifest,
}

impl DesktopToolsExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "tools-desktop",
            "Desktop Tools",
            Version::new(0, 1, 0),
        );
        manifest.description =
            "Desktop automation: mouse, keyboard, screenshot, clipboard".to_string();
        manifest.provides = Provides {
            tools: vec![
                "desktop_screenshot".to_string(),
                "desktop_mouse_move".to_string(),
                "desktop_mouse_click".to_string(),
                "desktop_mouse_scroll".to_string(),
                "desktop_keyboard_type".to_string(),
                "desktop_keyboard_key".to_string(),
                "desktop_keyboard_hotkey".to_string(),
                "desktop_clipboard_get".to_string(),
                "desktop_clipboard_set".to_string(),
                "desktop_screen_info".to_string(),
            ],
            ..Default::default()
        };

        Self { manifest }
    }
}

impl Default for DesktopToolsExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for DesktopToolsExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        // Register tools (each tool creates its own controller when needed)
        ctx.tool_registry
            .register_tool(Arc::new(DesktopScreenshotTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(ScreenInfoTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(MouseMoveTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(MouseClickTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(MouseScrollTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(KeyboardTypeTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(KeyboardKeyTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(KeyboardHotkeyTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(ClipboardGetTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(ClipboardSetTool::new()))?;

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
        let ext = DesktopToolsExtension::new();
        assert_eq!(ext.manifest().id, "tools-desktop");
        assert!(ext
            .manifest()
            .provides
            .tools
            .contains(&"desktop_screenshot".to_string()));
        assert!(ext
            .manifest()
            .provides
            .tools
            .contains(&"desktop_mouse_click".to_string()));
    }

    #[test]
    fn test_tool_count() {
        let ext = DesktopToolsExtension::new();
        assert_eq!(ext.manifest().provides.tools.len(), 10);
    }

    #[test]
    fn test_extension_default() {
        let ext = DesktopToolsExtension::default();
        assert_eq!(ext.manifest().id, "tools-desktop");
    }

    #[test]
    fn test_manifest_name() {
        let ext = DesktopToolsExtension::new();
        assert_eq!(ext.manifest().name, "Desktop Tools");
    }

    #[test]
    fn test_manifest_description() {
        let ext = DesktopToolsExtension::new();
        assert!(ext.manifest().description.contains("Desktop automation"));
    }

    #[test]
    fn test_manifest_version() {
        let ext = DesktopToolsExtension::new();
        assert_eq!(ext.manifest().version, Version::new(0, 1, 0));
    }

    #[test]
    fn test_all_tools_provided() {
        let ext = DesktopToolsExtension::new();
        let tools = &ext.manifest().provides.tools;
        assert!(tools.contains(&"desktop_screenshot".to_string()));
        assert!(tools.contains(&"desktop_mouse_move".to_string()));
        assert!(tools.contains(&"desktop_mouse_click".to_string()));
        assert!(tools.contains(&"desktop_mouse_scroll".to_string()));
        assert!(tools.contains(&"desktop_keyboard_type".to_string()));
        assert!(tools.contains(&"desktop_keyboard_key".to_string()));
        assert!(tools.contains(&"desktop_keyboard_hotkey".to_string()));
        assert!(tools.contains(&"desktop_clipboard_get".to_string()));
        assert!(tools.contains(&"desktop_clipboard_set".to_string()));
        assert!(tools.contains(&"desktop_screen_info".to_string()));
    }

    #[test]
    fn test_as_any() {
        let ext = DesktopToolsExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<DesktopToolsExtension>().is_some());
    }

    #[test]
    fn test_as_any_mut() {
        let mut ext = DesktopToolsExtension::new();
        let any_ref = ext.as_any_mut();
        assert!(any_ref.downcast_mut::<DesktopToolsExtension>().is_some());
    }

    #[test]
    fn test_provides_no_providers() {
        let ext = DesktopToolsExtension::new();
        assert!(ext.manifest().provides.providers.is_empty());
    }
}
