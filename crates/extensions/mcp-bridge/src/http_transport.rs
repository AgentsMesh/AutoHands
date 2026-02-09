//! HTTP transport for MCP communication.

use async_trait::async_trait;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::Mutex;

use crate::protocol::{McpRequest, McpResponse};
use crate::transport::{Transport, TransportError};

/// HTTP transport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpTransportConfig {
    /// Base URL for the MCP server.
    pub url: String,
    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Authorization header value (e.g., "Bearer token").
    pub authorization: Option<String>,
    /// Custom headers.
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
}

fn default_timeout() -> u64 {
    30
}

/// HTTP transport for MCP servers.
pub struct HttpTransport {
    client: Client,
    config: HttpTransportConfig,
    closed: Mutex<bool>,
}

impl HttpTransport {
    /// Create a new HTTP transport.
    pub fn new(config: HttpTransportConfig) -> Result<Self, TransportError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| TransportError::Process(e.to_string()))?;

        Ok(Self {
            client,
            config,
            closed: Mutex::new(false),
        })
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn send(&self, request: McpRequest) -> Result<McpResponse, TransportError> {
        if *self.closed.lock().await {
            return Err(TransportError::Closed);
        }

        let mut req = self
            .client
            .post(&self.config.url)
            .header(header::CONTENT_TYPE, "application/json");

        if let Some(ref auth) = self.config.authorization {
            req = req.header(header::AUTHORIZATION, auth);
        }

        for (key, value) in &self.config.headers {
            req = req.header(key, value);
        }

        let response = req
            .json(&request)
            .send()
            .await
            .map_err(|e| TransportError::Process(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TransportError::Process(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| TransportError::Process(e.to_string()))?;

        let mcp_response: McpResponse =
            serde_json::from_str(&body).map_err(TransportError::Json)?;

        Ok(mcp_response)
    }

    async fn close(&self) -> Result<(), TransportError> {
        *self.closed.lock().await = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> HttpTransportConfig {
        HttpTransportConfig {
            url: "http://localhost:8080/mcp".to_string(),
            timeout_seconds: 30,
            authorization: None,
            headers: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_http_transport_config_defaults() {
        let json = serde_json::json!({
            "url": "http://localhost:8080/mcp"
        });
        let config: HttpTransportConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.timeout_seconds, 30);
        assert!(config.authorization.is_none());
    }

    #[test]
    fn test_http_transport_creation() {
        let config = create_test_config();
        let transport = HttpTransport::new(config);
        assert!(transport.is_ok());
    }

    #[tokio::test]
    async fn test_http_transport_close() {
        let config = create_test_config();
        let transport = HttpTransport::new(config).unwrap();
        assert!(transport.close().await.is_ok());
    }

    #[tokio::test]
    async fn test_closed_transport_returns_error() {
        use crate::protocol::RequestId;

        let config = create_test_config();
        let transport = HttpTransport::new(config).unwrap();
        transport.close().await.unwrap();

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            method: "test".to_string(),
            params: None,
        };

        let result = transport.send(request).await;
        assert!(matches!(result, Err(TransportError::Closed)));
    }
}
