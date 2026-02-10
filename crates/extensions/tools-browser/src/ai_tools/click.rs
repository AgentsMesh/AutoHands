//! AI-powered click tool.

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
pub struct AiClickParams {
    /// Page ID to operate on.
    pub page_id: String,
    /// Natural language description of the element to click.
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
                    "description": "Natural language description of the element to click"
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

        let screenshot_base64 = self
            .manager
            .screenshot(&params.page_id, false)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Screenshot failed: {}", e)))?;

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

        if response.contains("\"error\"") {
            return Err(ToolError::ExecutionFailed(format!(
                "Element not found: {}",
                response
            )));
        }

        let coords = parse_coordinates(&response)?;

        if coords.confidence < 0.5 {
            return Err(ToolError::ExecutionFailed(format!(
                "Low confidence ({}) in element identification",
                coords.confidence
            )));
        }

        self.manager
            .click(&params.page_id, coords.x as f64, coords.y as f64)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Click failed: {}", e)))?;

        debug!("AI click executed at ({}, {})", coords.x, coords.y);

        let result = AiClickResult {
            success: true,
            clicked_at: coords,
            description: format!("Clicked '{}' at identified location", params.target),
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap()))
    }
}
