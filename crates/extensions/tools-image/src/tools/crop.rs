//! Image crop tool.

use async_trait::async_trait;
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use super::image_utils::{load_image, save_image};

#[derive(Debug, Deserialize)]
pub struct ImageCropParams {
    /// Input image path.
    pub input: String,
    /// Output image path.
    pub output: Option<String>,
    /// X coordinate of crop region.
    pub x: u32,
    /// Y coordinate of crop region.
    pub y: u32,
    /// Width of crop region.
    pub width: u32,
    /// Height of crop region.
    pub height: u32,
}

#[derive(Debug, Serialize)]
pub struct ImageCropResult {
    pub output_path: String,
    pub cropped_width: u32,
    pub cropped_height: u32,
}

/// Crop a region from an image.
pub struct ImageCropTool {
    definition: ToolDefinition,
}

impl ImageCropTool {
    pub fn new() -> Self {
        let mut definition = ToolDefinition::new(
            "image_crop",
            "Image Crop",
            "Crop a rectangular region from an image.",
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
                    "description": "Path for the output image"
                },
                "x": {
                    "type": "integer",
                    "description": "X coordinate of the top-left corner"
                },
                "y": {
                    "type": "integer",
                    "description": "Y coordinate of the top-left corner"
                },
                "width": {
                    "type": "integer",
                    "description": "Width of the crop region"
                },
                "height": {
                    "type": "integer",
                    "description": "Height of the crop region"
                }
            },
            "required": ["input", "x", "y", "width", "height"]
        }));

        Self { definition }
    }
}

impl Default for ImageCropTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ImageCropTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ImageCropParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let img = load_image(&params.input)?;
        let (img_width, img_height) = img.dimensions();

        // Validate crop region
        if params.x + params.width > img_width || params.y + params.height > img_height {
            return Err(ToolError::ExecutionFailed(format!(
                "Crop region exceeds image bounds ({}x{})",
                img_width, img_height
            )));
        }

        let cropped = img.crop_imm(params.x, params.y, params.width, params.height);

        let output_path = params.output.unwrap_or_else(|| params.input.clone());
        save_image(&cropped, &output_path, None)?;

        debug!(
            "Cropped image to {}x{} at ({}, {})",
            params.width, params.height, params.x, params.y
        );

        let result = ImageCropResult {
            output_path,
            cropped_width: params.width,
            cropped_height: params.height,
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap()))
    }
}
