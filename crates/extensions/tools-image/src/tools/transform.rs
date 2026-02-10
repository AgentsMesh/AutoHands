//! Image rotation and flip tools.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use super::image_utils::{load_image, save_image};

// ============================================================================
// Image Rotate Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ImageRotateParams {
    /// Input image path.
    pub input: String,
    /// Output image path.
    pub output: Option<String>,
    /// Rotation in degrees: 90, 180, 270, or any angle.
    pub degrees: i32,
}

/// Rotate an image.
pub struct ImageRotateTool {
    definition: ToolDefinition,
}

impl ImageRotateTool {
    pub fn new() -> Self {
        let mut definition = ToolDefinition::new(
            "image_rotate",
            "Image Rotate",
            "Rotate an image by the specified degrees (90, 180, 270 for lossless rotation).",
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
                "degrees": {
                    "type": "integer",
                    "description": "Rotation in degrees (positive = clockwise)"
                }
            },
            "required": ["input", "degrees"]
        }));

        Self { definition }
    }
}

impl Default for ImageRotateTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ImageRotateTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ImageRotateParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let img = load_image(&params.input)?;

        // Normalize degrees to 0-360 range
        let degrees = ((params.degrees % 360) + 360) % 360;

        let rotated = match degrees {
            90 => img.rotate90(),
            180 => img.rotate180(),
            270 => img.rotate270(),
            0 => img,
            _ => {
                return Err(ToolError::ExecutionFailed(
                    "Only 90, 180, 270 degree rotations are supported".to_string(),
                ));
            }
        };

        let output_path = params.output.unwrap_or_else(|| params.input.clone());
        save_image(&rotated, &output_path, None)?;

        debug!("Rotated image by {} degrees", degrees);

        Ok(ToolResult::success(format!(
            "Rotated image by {} degrees, saved to {}",
            degrees, output_path
        )))
    }
}

// ============================================================================
// Image Flip Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ImageFlipParams {
    /// Input image path.
    pub input: String,
    /// Output image path.
    pub output: Option<String>,
    /// Flip direction: horizontal or vertical.
    pub direction: String,
}

/// Flip an image horizontally or vertically.
pub struct ImageFlipTool {
    definition: ToolDefinition,
}

impl ImageFlipTool {
    pub fn new() -> Self {
        let mut definition = ToolDefinition::new(
            "image_flip",
            "Image Flip",
            "Flip an image horizontally or vertically.",
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
                "direction": {
                    "type": "string",
                    "enum": ["horizontal", "vertical"],
                    "description": "Flip direction"
                }
            },
            "required": ["input", "direction"]
        }));

        Self { definition }
    }
}

impl Default for ImageFlipTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ImageFlipTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ImageFlipParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let img = load_image(&params.input)?;

        let flipped = match params.direction.to_lowercase().as_str() {
            "horizontal" | "h" => img.fliph(),
            "vertical" | "v" => img.flipv(),
            _ => {
                return Err(ToolError::ExecutionFailed(
                    "Direction must be 'horizontal' or 'vertical'".to_string(),
                ));
            }
        };

        let output_path = params.output.unwrap_or_else(|| params.input.clone());
        save_image(&flipped, &output_path, None)?;

        debug!("Flipped image {}", params.direction);

        Ok(ToolResult::success(format!(
            "Flipped image {}, saved to {}",
            params.direction, output_path
        )))
    }
}
