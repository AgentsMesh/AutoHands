//! Web fetch tool implementation.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

/// Parameters for web_fetch tool.
#[derive(Debug, Deserialize)]
struct FetchParams {
    /// URL to fetch.
    url: String,

    /// HTTP method (default: GET).
    #[serde(default = "default_method")]
    method: String,

    /// Optional request body.
    body: Option<String>,

    /// Optional headers.
    #[serde(default)]
    headers: std::collections::HashMap<String, String>,

    /// Timeout in seconds.
    #[serde(default = "default_timeout")]
    timeout: u64,
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_timeout() -> u64 {
    30
}

/// Result from web fetch.
#[derive(Debug, Serialize)]
struct FetchResult {
    status: u16,
    headers: std::collections::HashMap<String, String>,
    body: String,
    url: String,
}

/// Tool for fetching web content.
pub struct WebFetchTool {
    definition: ToolDefinition,
    client: Client,
}

impl WebFetchTool {
    /// Create a new web fetch tool.
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("AutoHands/0.1")
            .build()
            .expect("Failed to create HTTP client");

        let definition = ToolDefinition::new(
            "web_fetch",
            "Web Fetch",
            "Fetch content from a URL",
        )
        .with_parameters_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch"
                },
                "method": {
                    "type": "string",
                    "description": "HTTP method (GET, POST, PUT, DELETE)",
                    "default": "GET"
                },
                "body": {
                    "type": "string",
                    "description": "Request body for POST/PUT"
                },
                "headers": {
                    "type": "object",
                    "description": "Additional headers"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Request timeout in seconds",
                    "default": 30
                }
            },
            "required": ["url"]
        }))
        .with_risk_level(RiskLevel::Medium);

        Self { definition, client }
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: FetchParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;

        // Validate URL
        let url = url::Url::parse(&params.url)
            .map_err(|e| ToolError::InvalidParameters(format!("Invalid URL: {}", e)))?;

        // Build request
        let mut request = match params.method.to_uppercase().as_str() {
            "GET" => self.client.get(url.clone()),
            "POST" => self.client.post(url.clone()),
            "PUT" => self.client.put(url.clone()),
            "DELETE" => self.client.delete(url.clone()),
            "HEAD" => self.client.head(url.clone()),
            _ => return Err(ToolError::InvalidParameters(
                format!("Unsupported method: {}", params.method),
            )),
        };

        // Set timeout
        request = request.timeout(Duration::from_secs(params.timeout));

        // Add headers
        for (key, value) in params.headers {
            request = request.header(&key, &value);
        }

        // Add body if present
        if let Some(body) = params.body {
            request = request.body(body);
        }

        // Execute request
        let response = request.send().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Request failed: {}", e)))?;

        let status = response.status().as_u16();
        let headers: std::collections::HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body = response.text().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read body: {}", e)))?;

        let result = FetchResult {
            status,
            headers,
            body,
            url: params.url,
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap()))
    }
}

#[cfg(test)]
#[path = "web_fetch_tests.rs"]
mod tests;
