//! Image processing tools.

use std::io::Cursor;
use std::path::Path;

use async_trait::async_trait;
use image::{DynamicImage, GenericImageView, ImageFormat};
use serde::{Deserialize, Serialize};
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

// ============================================================================
// Helper functions
// ============================================================================

fn load_image(path: &str) -> Result<DynamicImage, ToolError> {
    image::open(path).map_err(|e| ToolError::ExecutionFailed(format!("Failed to load image: {}", e)))
}

fn save_image(img: &DynamicImage, path: &str, format: Option<&str>) -> Result<(), ToolError> {
    let format = format
        .map(|f| parse_format(f))
        .transpose()?
        .or_else(|| ImageFormat::from_path(path).ok());

    match format {
        Some(fmt) => img
            .save_with_format(path, fmt)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to save image: {}", e))),
        None => img
            .save(path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to save image: {}", e))),
    }
}

fn parse_format(format: &str) -> Result<ImageFormat, ToolError> {
    match format.to_lowercase().as_str() {
        "png" => Ok(ImageFormat::Png),
        "jpg" | "jpeg" => Ok(ImageFormat::Jpeg),
        "gif" => Ok(ImageFormat::Gif),
        "bmp" => Ok(ImageFormat::Bmp),
        "webp" => Ok(ImageFormat::WebP),
        "tiff" | "tif" => Ok(ImageFormat::Tiff),
        "ico" => Ok(ImageFormat::Ico),
        _ => Err(ToolError::ExecutionFailed(format!(
            "Unsupported format: {}",
            format
        ))),
    }
}

fn image_to_base64(img: &DynamicImage, format: ImageFormat) -> Result<String, ToolError> {
    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, format)
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to encode image: {}", e)))?;

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(buffer.into_inner()))
}

// ============================================================================
// Image Resize Tool
// ============================================================================

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

// ============================================================================
// Image Crop Tool
// ============================================================================

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

// ============================================================================
// Image Convert Tool
// ============================================================================

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

// ============================================================================
// Image Info Tool
// ============================================================================

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_format() {
        assert!(matches!(parse_format("png").unwrap(), ImageFormat::Png));
        assert!(matches!(parse_format("jpg").unwrap(), ImageFormat::Jpeg));
        assert!(matches!(parse_format("jpeg").unwrap(), ImageFormat::Jpeg));
        assert!(matches!(parse_format("gif").unwrap(), ImageFormat::Gif));
        assert!(matches!(parse_format("webp").unwrap(), ImageFormat::WebP));
        assert!(parse_format("invalid").is_err());
    }

    #[test]
    fn test_resize_params_deserialize() {
        let json = r#"{"input": "test.png", "width": 100}"#;
        let params: ImageResizeParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.input, "test.png");
        assert_eq!(params.width, Some(100));
        assert!(params.preserve_aspect); // default
    }

    #[test]
    fn test_crop_params_deserialize() {
        let json = r#"{"input": "test.png", "x": 10, "y": 20, "width": 100, "height": 50}"#;
        let params: ImageCropParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.x, 10);
        assert_eq!(params.y, 20);
        assert_eq!(params.width, 100);
        assert_eq!(params.height, 50);
    }

    #[test]
    fn test_convert_params_deserialize() {
        let json = r#"{"input": "test.png", "output": "test.jpg", "format": "jpeg"}"#;
        let params: ImageConvertParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.format, Some("jpeg".to_string()));
    }

    #[test]
    fn test_info_params_deserialize() {
        let json = r#"{"path": "test.png"}"#;
        let params: ImageInfoParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.path, "test.png");
    }

    #[test]
    fn test_rotate_params_deserialize() {
        let json = r#"{"input": "test.png", "degrees": 90}"#;
        let params: ImageRotateParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.degrees, 90);
    }

    #[test]
    fn test_flip_params_deserialize() {
        let json = r#"{"input": "test.png", "direction": "horizontal"}"#;
        let params: ImageFlipParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.direction, "horizontal");
    }

    #[test]
    fn test_tool_definitions() {
        let resize = ImageResizeTool::new();
        assert_eq!(resize.definition().id, "image_resize");

        let crop = ImageCropTool::new();
        assert_eq!(crop.definition().id, "image_crop");

        let convert = ImageConvertTool::new();
        assert_eq!(convert.definition().id, "image_convert");

        let info = ImageInfoTool::new();
        assert_eq!(info.definition().id, "image_info");

        let rotate = ImageRotateTool::new();
        assert_eq!(rotate.definition().id, "image_rotate");

        let flip = ImageFlipTool::new();
        assert_eq!(flip.definition().id, "image_flip");
    }
}
