//! AI-powered form fill tool.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

use crate::manager::BrowserManager;

use super::{parse_coordinates, ElementCoordinates, VisionProvider};

#[derive(Debug, Deserialize)]
pub struct AiFillParams {
    /// Page ID to operate on.
    pub page_id: String,
    /// Natural language description of the form field to fill.
    pub field: String,
    /// Value to enter into the field.
    pub value: String,
    /// Whether to clear the field first (default: true).
    #[serde(default = "default_clear", rename = "clear_first")]
    pub _clear_first: bool,
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
                    "description": "Natural language description of the form field"
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

        let screenshot_base64 = self
            .manager
            .screenshot(&params.page_id, false)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Screenshot failed: {}", e)))?;

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

        self.manager
            .click(&params.page_id, coords.x as f64, coords.y as f64)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Click failed: {}", e)))?;

        self.manager
            .type_text(&params.page_id, &params.value)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Type failed: {}", e)))?;

        debug!("AI fill executed at ({}, {})", coords.x, coords.y);

        let result = AiFillResult {
            success: true,
            field_coordinates: coords,
            value_entered: params.value,
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap()))
    }
}
