//! AI-powered browser automation tools.
//!
//! These tools use vision LLM capabilities to interact with web pages
//! based on natural language descriptions rather than CSS selectors.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use autohands_protocols::error::ToolError;
use autohands_protocols::provider::{CompletionRequest, LLMProvider};
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::{
    ContentPart, ImageSource, Message, MessageContent, MessageRole, RiskLevel,
};

use crate::manager::BrowserManager;

// ============================================================================
// AI Provider Wrapper
// ============================================================================

/// Wrapper for vision-capable LLM provider.
pub struct VisionProvider {
    provider: Arc<dyn LLMProvider>,
    model: String,
}

impl VisionProvider {
    /// Create a new vision provider wrapper.
    pub fn new(provider: Arc<dyn LLMProvider>, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
        }
    }

    /// Analyze an image with a prompt.
    ///
    /// Note: Screenshots are in JPEG format (60% quality) to reduce size.
    pub async fn analyze(&self, image_base64: &str, prompt: &str) -> Result<String, ToolError> {
        let message = Message {
            role: MessageRole::User,
            content: MessageContent::Parts(vec![
                ContentPart::Image {
                    source: ImageSource::Base64 {
                        media_type: "image/jpeg".to_string(),
                        data: image_base64.to_string(),
                    },
                },
                ContentPart::Text {
                    text: prompt.to_string(),
                },
            ]),
            name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
            metadata: HashMap::new(),
        };

        let request = CompletionRequest::new(&self.model, vec![message])
            .with_max_tokens(1024)
            .with_temperature(0.0);

        let response = self
            .provider
            .complete(request)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Vision API error: {}", e)))?;

        Ok(response.message.content.text())
    }
}

// ============================================================================
// Coordinate Parsing
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementCoordinates {
    pub x: i32,
    pub y: i32,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub confidence: f32,
}

fn parse_coordinates(response: &str) -> Result<ElementCoordinates, ToolError> {
    // Try to parse JSON response first
    if let Ok(coords) = serde_json::from_str::<ElementCoordinates>(response) {
        return Ok(coords);
    }

    // Try to extract coordinates from text response
    // Look for patterns like "x: 100, y: 200" or "(100, 200)" or "coordinates: 100, 200"
    let patterns = [
        r"x[:\s]*(\d+)[,\s]+y[:\s]*(\d+)",
        r"\((\d+)[,\s]+(\d+)\)",
        r"coordinates[:\s]*(\d+)[,\s]+(\d+)",
        r"position[:\s]*(\d+)[,\s]+(\d+)",
        r"(\d+)[,\s]+(\d+)",
    ];

    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(caps) = re.captures(response) {
                if let (Some(x), Some(y)) = (caps.get(1), caps.get(2)) {
                    if let (Ok(x_val), Ok(y_val)) = (x.as_str().parse(), y.as_str().parse()) {
                        return Ok(ElementCoordinates {
                            x: x_val,
                            y: y_val,
                            width: None,
                            height: None,
                            confidence: 0.8,
                        });
                    }
                }
            }
        }
    }

    Err(ToolError::ExecutionFailed(format!(
        "Could not parse coordinates from response: {}",
        response
    )))
}

// ============================================================================
// AI Click Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AiClickParams {
    /// Page ID to operate on.
    pub page_id: String,
    /// Natural language description of the element to click.
    /// Examples: "the login button", "the search icon", "Submit button"
    pub target: String,
}

#[derive(Debug, Serialize)]
pub struct AiClickResult {
    pub success: bool,
    pub clicked_at: ElementCoordinates,
    pub description: String,
}

/// AI-powered click tool that identifies elements using vision.
pub struct AiClickTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
    vision: Arc<VisionProvider>,
}

impl AiClickTool {
    pub fn new(manager: Arc<BrowserManager>, vision: Arc<VisionProvider>) -> Self {
        let mut definition = ToolDefinition::new(
            "browser_ai_click",
            "Browser AI Click",
            "Click an element identified by natural language description using AI vision. \
             Use this when you don't know the CSS selector but can describe what to click.",
        );
        definition.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "page_id": {
                    "type": "string",
                    "description": "The page ID to operate on"
                },
                "target": {
                    "type": "string",
                    "description": "Natural language description of the element to click (e.g., 'the login button', 'the blue Submit button')"
                }
            },
            "required": ["page_id", "target"]
        }));
        definition.risk_level = RiskLevel::Medium;

        Self {
            definition,
            manager,
            vision,
        }
    }
}

#[async_trait]
impl Tool for AiClickTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: AiClickParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        // Take screenshot
        let screenshot_base64 = self
            .manager
            .screenshot(&params.page_id, false)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Screenshot failed: {}", e)))?;

        // Ask vision model to find the element
        let prompt = format!(
            r#"Find the element described as "{}" in this screenshot.
Return the center coordinates of the element in JSON format:
{{"x": <number>, "y": <number>, "confidence": <0.0-1.0>}}

If you cannot find the element, respond with:
{{"error": "Element not found", "reason": "<explanation>"}}

Only respond with the JSON, no other text."#,
            params.target
        );

        let response = self.vision.analyze(&screenshot_base64, &prompt).await?;
        info!("Vision response: {}", response);

        // Check for error response
        if response.contains("\"error\"") {
            return Err(ToolError::ExecutionFailed(format!(
                "Element not found: {}",
                response
            )));
        }

        // Parse coordinates
        let coords = parse_coordinates(&response)?;

        // Verify confidence threshold
        if coords.confidence < 0.5 {
            return Err(ToolError::ExecutionFailed(format!(
                "Low confidence ({}) in element identification",
                coords.confidence
            )));
        }

        // Click at coordinates
        self.manager
            .click(&params.page_id, coords.x as f64, coords.y as f64)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Click failed: {}", e)))?;

        debug!(
            "AI click executed at ({}, {})",
            coords.x, coords.y
        );

        let result = AiClickResult {
            success: true,
            clicked_at: coords,
            description: format!("Clicked '{}' at identified location", params.target),
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap()))
    }
}

// ============================================================================
// AI Fill Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AiFillParams {
    /// Page ID to operate on.
    pub page_id: String,
    /// Natural language description of the form field to fill.
    pub field: String,
    /// Value to enter into the field.
    pub value: String,
    /// Whether to clear the field first (default: true).
    /// TODO: Implement clear functionality in fill operation.
    #[serde(default = "default_clear")]
    #[allow(dead_code)]
    pub clear_first: bool,
}

fn default_clear() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct AiFillResult {
    pub success: bool,
    pub field_coordinates: ElementCoordinates,
    pub value_entered: String,
}

/// AI-powered form fill tool.
pub struct AiFillTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
    vision: Arc<VisionProvider>,
}

impl AiFillTool {
    pub fn new(manager: Arc<BrowserManager>, vision: Arc<VisionProvider>) -> Self {
        let mut definition = ToolDefinition::new(
            "browser_ai_fill",
            "Browser AI Fill",
            "Fill a form field identified by natural language description using AI vision. \
             Use this when you don't know the CSS selector but can describe the field.",
        );
        definition.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "page_id": {
                    "type": "string",
                    "description": "The page ID to operate on"
                },
                "field": {
                    "type": "string",
                    "description": "Natural language description of the form field (e.g., 'email input', 'password field', 'search box')"
                },
                "value": {
                    "type": "string",
                    "description": "The value to enter into the field"
                },
                "clear_first": {
                    "type": "boolean",
                    "description": "Whether to clear existing content first (default: true)"
                }
            },
            "required": ["page_id", "field", "value"]
        }));
        definition.risk_level = RiskLevel::Medium;

        Self {
            definition,
            manager,
            vision,
        }
    }
}

#[async_trait]
impl Tool for AiFillTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: AiFillParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        // Take screenshot
        let screenshot_base64 = self
            .manager
            .screenshot(&params.page_id, false)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Screenshot failed: {}", e)))?;

        // Ask vision model to find the input field
        let prompt = format!(
            r#"Find the input/form field described as "{}" in this screenshot.
Return the center coordinates of the field in JSON format:
{{"x": <number>, "y": <number>, "confidence": <0.0-1.0>}}

If you cannot find the field, respond with:
{{"error": "Field not found", "reason": "<explanation>"}}

Only respond with the JSON, no other text."#,
            params.field
        );

        let response = self.vision.analyze(&screenshot_base64, &prompt).await?;
        info!("Vision response: {}", response);

        if response.contains("\"error\"") {
            return Err(ToolError::ExecutionFailed(format!(
                "Field not found: {}",
                response
            )));
        }

        let coords = parse_coordinates(&response)?;

        // Click on the field first
        self.manager
            .click(&params.page_id, coords.x as f64, coords.y as f64)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Click failed: {}", e)))?;

        // Type the value
        self.manager
            .type_text(&params.page_id, &params.value)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Type failed: {}", e)))?;

        debug!(
            "AI fill executed at ({}, {})",
            coords.x, coords.y
        );

        let result = AiFillResult {
            success: true,
            field_coordinates: coords,
            value_entered: params.value,
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap()))
    }
}

// ============================================================================
// AI Extract Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AiExtractParams {
    /// Page ID to operate on.
    pub page_id: String,
    /// Description of what data to extract from the page.
    pub query: String,
    /// Expected output format (json, list, text).
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "json".to_string()
}

/// AI-powered data extraction tool.
pub struct AiExtractTool {
    definition: ToolDefinition,
    manager: Arc<BrowserManager>,
    vision: Arc<VisionProvider>,
}

impl AiExtractTool {
    pub fn new(manager: Arc<BrowserManager>, vision: Arc<VisionProvider>) -> Self {
        let mut definition = ToolDefinition::new(
            "browser_ai_extract",
            "Browser AI Extract",
            "Extract structured data from a web page using AI vision. \
             Use this to extract tables, lists, product info, or any structured content.",
        );
        definition.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "page_id": {
                    "type": "string",
                    "description": "The page ID to operate on"
                },
                "query": {
                    "type": "string",
                    "description": "Description of what data to extract (e.g., 'all product names and prices', 'the main article title and author')"
                },
                "format": {
                    "type": "string",
                    "enum": ["json", "list", "text"],
                    "description": "Output format (default: json)"
                }
            },
            "required": ["page_id", "query"]
        }));

        Self {
            definition,
            manager,
            vision,
        }
    }
}

#[async_trait]
impl Tool for AiExtractTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: AiExtractParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        // Take full-page screenshot for extraction
        let screenshot_base64 = self
            .manager
            .screenshot(&params.page_id, true)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Screenshot failed: {}", e)))?;

        // Build extraction prompt based on format
        let format_instruction = match params.format.as_str() {
            "json" => "Return the extracted data as a valid JSON object or array.",
            "list" => "Return the extracted data as a bullet-point list.",
            "text" => "Return the extracted data as plain text.",
            _ => "Return the extracted data in a structured format.",
        };

        let prompt = format!(
            r#"Analyze this web page screenshot and extract the following information:
{}

{}

If you cannot find the requested information, explain what you found instead.
Be thorough and accurate in your extraction."#,
            params.query, format_instruction
        );

        let response = self.vision.analyze(&screenshot_base64, &prompt).await?;
        info!("Extraction response length: {} chars", response.len());

        Ok(ToolResult::success(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_coordinates_json() {
        let response = r#"{"x": 100, "y": 200, "confidence": 0.95}"#;
        let coords = parse_coordinates(response).unwrap();
        assert_eq!(coords.x, 100);
        assert_eq!(coords.y, 200);
        assert!((coords.confidence - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_parse_coordinates_text_pattern() {
        let response = "The button is located at x: 150, y: 300";
        let coords = parse_coordinates(response).unwrap();
        assert_eq!(coords.x, 150);
        assert_eq!(coords.y, 300);
    }

    #[test]
    fn test_parse_coordinates_tuple_pattern() {
        let response = "Found element at (250, 400)";
        let coords = parse_coordinates(response).unwrap();
        assert_eq!(coords.x, 250);
        assert_eq!(coords.y, 400);
    }

    #[test]
    fn test_parse_coordinates_simple_numbers() {
        let response = "Click at 300, 500";
        let coords = parse_coordinates(response).unwrap();
        assert_eq!(coords.x, 300);
        assert_eq!(coords.y, 500);
    }

    #[test]
    fn test_parse_coordinates_invalid() {
        let response = "I cannot find the element";
        let result = parse_coordinates(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_element_coordinates_serialize() {
        let coords = ElementCoordinates {
            x: 100,
            y: 200,
            width: Some(50),
            height: Some(30),
            confidence: 0.9,
        };
        let json = serde_json::to_string(&coords).unwrap();
        assert!(json.contains("100"));
        assert!(json.contains("200"));
    }

    #[test]
    fn test_ai_click_params_deserialize() {
        let json = r#"{"page_id": "page_1", "target": "login button"}"#;
        let params: AiClickParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.page_id, "page_1");
        assert_eq!(params.target, "login button");
    }

    #[test]
    fn test_ai_fill_params_deserialize() {
        let json = r#"{"page_id": "page_1", "field": "email input", "value": "test@example.com"}"#;
        let params: AiFillParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.page_id, "page_1");
        assert_eq!(params.field, "email input");
        assert_eq!(params.value, "test@example.com");
        assert!(params.clear_first); // default
    }

    #[test]
    fn test_ai_fill_params_with_clear() {
        let json = r#"{"page_id": "page_1", "field": "name", "value": "John", "clear_first": false}"#;
        let params: AiFillParams = serde_json::from_str(json).unwrap();
        assert!(!params.clear_first);
    }

    #[test]
    fn test_ai_extract_params_deserialize() {
        let json = r#"{"page_id": "page_1", "query": "product prices"}"#;
        let params: AiExtractParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.page_id, "page_1");
        assert_eq!(params.query, "product prices");
        assert_eq!(params.format, "json"); // default
    }

    #[test]
    fn test_ai_extract_params_with_format() {
        let json = r#"{"page_id": "page_1", "query": "headlines", "format": "list"}"#;
        let params: AiExtractParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.format, "list");
    }
}
