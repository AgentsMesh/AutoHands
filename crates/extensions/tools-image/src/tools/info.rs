//! Image info tool.

use async_trait::async_trait;
use image::{GenericImageView, ImageFormat};
use serde::{Deserialize, Serialize};

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use super::image_utils::load_image;

#[derive(Debug, Deserialize)]
pub struct ImageInfoParams {
    /// Image path.
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ImageInfoResult {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub color_type: String,
    pub size_bytes: u64,
}

/// Get information about an image.
pub struct ImageInfoTool {
    definition: ToolDefinition,
}

impl ImageInfoTool {
    pub fn new() -> Self {
        let mut definition = ToolDefinition::new(
            "image_info",
            "Image Info",
            "Get metadata about an image (dimensions, format, color type, size).",
        );
        definition.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the image"
                }
            },
            "required": ["path"]
        }));

        Self { definition }
    }
}

impl Default for ImageInfoTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ImageInfoTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ImageInfoParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let img = load_image(&params.path)?;
        let (width, height) = img.dimensions();

        let format = ImageFormat::from_path(&params.path)
            .map(|f| format!("{:?}", f))
            .unwrap_or_else(|_| "Unknown".to_string());

        let color_type = format!("{:?}", img.color());

        let size = std::fs::metadata(&params.path)
            .map(|m| m.len())
            .unwrap_or(0);

        let result = ImageInfoResult {
            path: params.path,
            width,
            height,
            format,
            color_type,
            size_bytes: size,
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap()))
    }
}
