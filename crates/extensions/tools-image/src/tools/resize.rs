//! Image resize tool.

use async_trait::async_trait;
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use super::image_utils::{load_image, save_image};

#[derive(Debug, Deserialize)]
pub struct ImageResizeParams {
    /// Input image path.
    pub input: String,
    /// Output image path (defaults to overwriting input if not specified).
    pub output: Option<String>,
    /// Target width in pixels.
    pub width: Option<u32>,
    /// Target height in pixels.
    pub height: Option<u32>,
    /// Preserve aspect ratio (default: true).
    #[serde(default = "default_preserve_aspect")]
    pub preserve_aspect: bool,
    /// Resize filter: nearest, triangle, catmull-rom, gaussian, lanczos3.
    #[serde(default = "default_filter")]
    pub filter: String,
}

fn default_preserve_aspect() -> bool {
    true
}

fn default_filter() -> String {
    "lanczos3".to_string()
}

#[derive(Debug, Serialize)]
pub struct ImageResizeResult {
    pub output_path: String,
    pub original_width: u32,
    pub original_height: u32,
    pub new_width: u32,
    pub new_height: u32,
}

/// Resize an image to specified dimensions.
pub struct ImageResizeTool {
    definition: ToolDefinition,
}

impl ImageResizeTool {
    pub fn new() -> Self {
        let mut definition = ToolDefinition::new(
            "image_resize",
            "Image Resize",
            "Resize an image to specified dimensions. Can preserve aspect ratio.",
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
                    "description": "Path for the output image (optional, defaults to overwriting input)"
                },
                "width": {
                    "type": "integer",
                    "description": "Target width in pixels"
                },
                "height": {
                    "type": "integer",
                    "description": "Target height in pixels"
                },
                "preserve_aspect": {
                    "type": "boolean",
                    "description": "Preserve aspect ratio (default: true)"
                },
                "filter": {
                    "type": "string",
                    "enum": ["nearest", "triangle", "catmull-rom", "gaussian", "lanczos3"],
                    "description": "Resize filter to use (default: lanczos3)"
                }
            },
            "required": ["input"]
        }));

        Self { definition }
    }
}

impl Default for ImageResizeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ImageResizeTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: ImageResizeParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let img = load_image(&params.input)?;
        let (orig_width, orig_height) = img.dimensions();

        // Calculate target dimensions
        let (new_width, new_height) = match (params.width, params.height) {
            (Some(w), Some(h)) if params.preserve_aspect => {
                let ratio = (w as f64 / orig_width as f64).min(h as f64 / orig_height as f64);
                (
                    (orig_width as f64 * ratio) as u32,
                    (orig_height as f64 * ratio) as u32,
                )
            }
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => {
                let ratio = w as f64 / orig_width as f64;
                (w, (orig_height as f64 * ratio) as u32)
            }
            (None, Some(h)) => {
                let ratio = h as f64 / orig_height as f64;
                ((orig_width as f64 * ratio) as u32, h)
            }
            (None, None) => {
                return Err(ToolError::ExecutionFailed(
                    "Must specify at least width or height".to_string(),
                ));
            }
        };

        // Select filter
        let filter = match params.filter.as_str() {
            "nearest" => image::imageops::FilterType::Nearest,
            "triangle" => image::imageops::FilterType::Triangle,
            "catmull-rom" => image::imageops::FilterType::CatmullRom,
            "gaussian" => image::imageops::FilterType::Gaussian,
            _ => image::imageops::FilterType::Lanczos3,
        };

        let resized = img.resize(new_width, new_height, filter);

        let output_path = params.output.unwrap_or_else(|| params.input.clone());
        save_image(&resized, &output_path, None)?;

        debug!(
            "Resized image from {}x{} to {}x{}",
            orig_width, orig_height, new_width, new_height
        );

        let result = ImageResizeResult {
            output_path: output_path.clone(),
            original_width: orig_width,
            original_height: orig_height,
            new_width,
            new_height,
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap()))
    }
}
