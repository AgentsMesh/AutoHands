//! Image format conversion tool.

use std::path::Path;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use super::image_utils::{load_image, save_image};

#[derive(Debug, Deserialize)]
pub struct ImageConvertParams {
    /// Input image path.
    pub input: String,
    /// Output image path.
    pub output: String,
    /// Target format: png, jpg, gif, bmp, webp.
    pub format: Option<String>,
    /// Quality for JPEG (1-100, default: 85).
    pub quality: Option<u8>,
}

#[derive(Debug, Serialize)]
pub struct ImageConvertResult {
    pub output_path: String,
    pub format: String,
    pub size_bytes: u64,
}

/// Convert an image to a different format.
pub struct ImageConvertTool {
    definition: ToolDefinition,
}

impl ImageConvertTool {
    pub fn new() -> Self {
        let mut definition = ToolDefinition::new(
            "image_convert",
            "Image Convert",
            "Convert an image to a different format (PNG, JPEG, GIF, BMP, WebP).",
        );
        definition.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Path to the input image"
                },
                "output": {
                    "type": "string",
                    "description": "Path for the output image (format determined by extension)"
                },
                "format": {
                    "type": "string",
                    "enum": ["png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff"],
                    "description": "Target format (optional, inferred from output path)"
                },
                "quality": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 100,
                    "description": "Quality for JPEG compression (1-100, default: 85)"
                }
            },
            "required": ["input", "output"]
        }));

        Self { definition }
    }
}

impl Default for ImageConvertTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ImageConvertTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ImageConvertParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let img = load_image(&params.input)?;

        // Determine format - clone output path to avoid borrow issues
        let output_path = params.output;
        let format_str = params.format.as_deref().map(|s| s.to_string()).unwrap_or_else(|| {
            Path::new(&output_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png")
                .to_string()
        });

        save_image(&img, &output_path, Some(&format_str))?;

        // Get file size
        let size = std::fs::metadata(&output_path)
            .map(|m| m.len())
            .unwrap_or(0);

        debug!("Converted image to {} format", format_str);

        let result = ImageConvertResult {
            output_path,
            format: format_str,
            size_bytes: size,
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap()))
    }
}
