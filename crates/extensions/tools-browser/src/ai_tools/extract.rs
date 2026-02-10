//! AI-powered data extraction tool.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::info;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::manager::BrowserManager;

use super::VisionProvider;

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
                    "description": "Description of what data to extract"
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

        let screenshot_base64 = self
            .manager
            .screenshot(&params.page_id, true)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Screenshot failed: {}", e)))?;

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
