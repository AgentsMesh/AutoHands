//! OCR recognition tools.

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::ocr::OcrController;

// Helper to run blocking code
async fn run_blocking<F, T>(f: F) -> Result<T, ToolError>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
        .map_err(ToolError::ExecutionFailed)
}

// ============================================================================
// OCR Screen Tool
// ============================================================================

/// Recognize text from the entire screen.
pub struct OcrScreenTool {
    definition: ToolDefinition,
}

impl OcrScreenTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_ocr_screen",
                "Desktop OCR Screen",
                "Recognize text from the entire screen using OCR",
            ),
        }
    }
}

impl Default for OcrScreenTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for OcrScreenTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        _params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let result = run_blocking(|| {
            let controller = OcrController::new().map_err(|e| e.to_string())?;
            controller.recognize_screen().map_err(|e| e.to_string())
        })
        .await?;

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "OCR recognized {} characters with {:.2}% confidence",
            result.text.len(),
            result.confidence * 100.0
        );

        Ok(ToolResult::success(json))
    }
}

// ============================================================================
// OCR Region Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct OcrRegionParams {
    /// X position of the region.
    pub x: i32,
    /// Y position of the region.
    pub y: i32,
    /// Width of the region.
    pub width: u32,
    /// Height of the region.
    pub height: u32,
}

/// Recognize text from a region of the screen.
pub struct OcrRegionTool {
    definition: ToolDefinition,
}

impl OcrRegionTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_ocr_region",
                "Desktop OCR Region",
                "Recognize text from a specific region of the screen using OCR",
            ),
        }
    }
}

impl Default for OcrRegionTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for OcrRegionTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: OcrRegionParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let x = params.x;
        let y = params.y;
        let width = params.width;
        let height = params.height;

        let result = run_blocking(move || {
            let controller = OcrController::new().map_err(|e| e.to_string())?;
            controller
                .recognize_region(x, y, width, height)
                .map_err(|e| e.to_string())
        })
        .await?;

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "OCR recognized {} characters from region ({}, {}, {}x{}) with {:.2}% confidence",
            result.text.len(),
            x,
            y,
            width,
            height,
            result.confidence * 100.0
        );

        Ok(ToolResult::success(json))
    }
}

// ============================================================================
// OCR Image Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct OcrImageParams {
    /// Base64 encoded image data.
    pub image_base64: String,
}

/// Recognize text from base64 encoded image data.
pub struct OcrImageTool {
    definition: ToolDefinition,
}

impl OcrImageTool {
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "desktop_ocr_image",
                "Desktop OCR Image",
                "Recognize text from a base64 encoded image using OCR",
            ),
        }
    }
}

impl Default for OcrImageTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for OcrImageTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: OcrImageParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        // Decode base64
        use base64::Engine;
        let image_data = base64::engine::general_purpose::STANDARD
            .decode(&params.image_base64)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid base64: {}", e)))?;

        let result = run_blocking(move || {
            let controller = OcrController::new().map_err(|e| e.to_string())?;
            controller
                .recognize_image(&image_data)
                .map_err(|e| e.to_string())
        })
        .await?;

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "OCR recognized {} characters from image with {:.2}% confidence",
            result.text.len(),
            result.confidence * 100.0
        );

        Ok(ToolResult::success(json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_screen_tool_definition() {
        let tool = OcrScreenTool::new();
        assert_eq!(tool.definition().id, "desktop_ocr_screen");
    }

    #[test]
    fn test_ocr_region_tool_definition() {
        let tool = OcrRegionTool::new();
        assert_eq!(tool.definition().id, "desktop_ocr_region");
    }

    #[test]
    fn test_ocr_image_tool_definition() {
        let tool = OcrImageTool::new();
        assert_eq!(tool.definition().id, "desktop_ocr_image");
    }

    #[test]
    fn test_ocr_region_params() {
        let json = serde_json::json!({
            "x": 100,
            "y": 200,
            "width": 300,
            "height": 400
        });
        let params: OcrRegionParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.x, 100);
        assert_eq!(params.y, 200);
        assert_eq!(params.width, 300);
        assert_eq!(params.height, 400);
    }

    #[test]
    fn test_ocr_image_params() {
        let json = serde_json::json!({
            "image_base64": "aGVsbG8="
        });
        let params: OcrImageParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.image_base64, "aGVsbG8=");
    }

    #[test]
    fn test_tools_default_impl() {
        let _ = OcrScreenTool::default();
        let _ = OcrRegionTool::default();
        let _ = OcrImageTool::default();
    }
}
