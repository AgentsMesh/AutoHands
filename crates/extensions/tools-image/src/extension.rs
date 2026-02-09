//! Image tools extension.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::tools::*;

/// Image processing tools extension.
pub struct ImageToolsExtension {
    manifest: ExtensionManifest,
}

impl ImageToolsExtension {
    /// Create a new image tools extension.
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "tools-image",
            "Image Tools",
            Version::new(0, 1, 0),
        );
        manifest.description = "Image processing and manipulation tools".to_string();
        manifest.provides = Provides {
            tools: vec![
                "image_resize".to_string(),
                "image_crop".to_string(),
                "image_convert".to_string(),
                "image_info".to_string(),
                "image_rotate".to_string(),
                "image_flip".to_string(),
            ],
            ..Default::default()
        };

        Self { manifest }
    }
}

impl Default for ImageToolsExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for ImageToolsExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        // Register all image processing tools
        ctx.tool_registry
            .register_tool(Arc::new(ImageResizeTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(ImageCropTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(ImageConvertTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(ImageInfoTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(ImageRotateTool::new()))?;
        ctx.tool_registry
            .register_tool(Arc::new(ImageFlipTool::new()))?;

        tracing::info!("Image tools extension initialized with 6 tools");
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
        let ext = ImageToolsExtension::new();
        assert_eq!(ext.manifest().id, "tools-image");
        assert_eq!(ext.manifest().provides.tools.len(), 6);
    }

    #[test]
    fn test_manifest_tools() {
        let ext = ImageToolsExtension::new();
        let tools = &ext.manifest().provides.tools;
        assert!(tools.contains(&"image_resize".to_string()));
        assert!(tools.contains(&"image_crop".to_string()));
        assert!(tools.contains(&"image_convert".to_string()));
        assert!(tools.contains(&"image_info".to_string()));
        assert!(tools.contains(&"image_rotate".to_string()));
        assert!(tools.contains(&"image_flip".to_string()));
    }

    #[test]
    fn test_extension_default() {
        let ext = ImageToolsExtension::default();
        assert_eq!(ext.manifest().id, "tools-image");
    }

    #[test]
    fn test_manifest_description() {
        let ext = ImageToolsExtension::new();
        assert!(ext.manifest().description.contains("Image"));
    }

    #[test]
    fn test_as_any() {
        let ext = ImageToolsExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<ImageToolsExtension>().is_some());
    }
}
