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
mod tests {
    use super::*;
    use std::path::PathBuf;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[test]
    fn test_tool_definition() {
        let tool = WebFetchTool::new();
        assert_eq!(tool.definition().id, "web_fetch");
        assert_eq!(tool.definition().risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_tool_default() {
        let tool = WebFetchTool::default();
        assert_eq!(tool.definition().id, "web_fetch");
    }

    #[test]
    fn test_default_method() {
        assert_eq!(default_method(), "GET");
    }

    #[test]
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 30);
    }

    #[test]
    fn test_fetch_params_parsing() {
        let json = serde_json::json!({
            "url": "https://example.com"
        });
        let params: FetchParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.url, "https://example.com");
        assert_eq!(params.method, "GET");
        assert_eq!(params.timeout, 30);
        assert!(params.body.is_none());
        assert!(params.headers.is_empty());
    }

    #[test]
    fn test_fetch_params_full() {
        let json = serde_json::json!({
            "url": "https://api.example.com",
            "method": "POST",
            "body": "{\"key\": \"value\"}",
            "headers": {"Content-Type": "application/json"},
            "timeout": 60
        });
        let params: FetchParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.method, "POST");
        assert_eq!(params.body, Some("{\"key\": \"value\"}".to_string()));
        assert_eq!(params.timeout, 60);
        assert_eq!(params.headers.get("Content-Type"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_fetch_result_serialize() {
        let result = FetchResult {
            status: 200,
            headers: std::collections::HashMap::new(),
            body: "test body".to_string(),
            url: "https://example.com".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("200"));
        assert!(json.contains("test body"));
        assert!(json.contains("example.com"));
    }

    #[tokio::test]
    async fn test_fetch_get() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Hello, World!"))
            .mount(&mock_server)
            .await;

        let tool = WebFetchTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("."));
        let params = serde_json::json!({
            "url": format!("{}/test", mock_server.uri())
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("200"));
        assert!(result.content.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_fetch_post() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api"))
            .respond_with(ResponseTemplate::new(201).set_body_string("{\"id\": 1}"))
            .mount(&mock_server)
            .await;

        let tool = WebFetchTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("."));
        let params = serde_json::json!({
            "url": format!("{}/api", mock_server.uri()),
            "method": "POST",
            "body": "{\"name\": \"test\"}"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("201"));
    }

    #[tokio::test]
    async fn test_fetch_put() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/resource/1"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let tool = WebFetchTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("."));
        let params = serde_json::json!({
            "url": format!("{}/resource/1", mock_server.uri()),
            "method": "PUT",
            "body": "{\"updated\": true}"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_fetch_delete() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/resource/1"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let tool = WebFetchTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("."));
        let params = serde_json::json!({
            "url": format!("{}/resource/1", mock_server.uri()),
            "method": "DELETE"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("204"));
    }

    #[tokio::test]
    async fn test_fetch_head() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(path("/info"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let tool = WebFetchTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("."));
        let params = serde_json::json!({
            "url": format!("{}/info", mock_server.uri()),
            "method": "HEAD"
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("200"));
    }

    #[tokio::test]
    async fn test_fetch_invalid_url() {
        let tool = WebFetchTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("."));
        let params = serde_json::json!({
            "url": "not-a-url"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_unsupported_method() {
        let tool = WebFetchTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("."));
        let params = serde_json::json!({
            "url": "https://example.com",
            "method": "PATCH"
        });

        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_with_headers() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/auth"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let tool = WebFetchTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("."));
        let params = serde_json::json!({
            "url": format!("{}/auth", mock_server.uri()),
            "headers": {
                "Authorization": "Bearer token123"
            }
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("200"));
    }

    #[tokio::test]
    async fn test_fetch_404() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/notfound"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;

        let tool = WebFetchTool::new();
        let ctx = ToolContext::new("test", PathBuf::from("."));
        let params = serde_json::json!({
            "url": format!("{}/notfound", mock_server.uri())
        });

        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.content.contains("404"));
    }
}
