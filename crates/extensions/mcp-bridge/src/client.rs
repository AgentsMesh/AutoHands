//! MCP client implementation.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use tracing::{debug, info};

use crate::protocol::{
    McpMethod, McpRequest, McpResponse, McpToolDefinition, McpToolResult,
};
use crate::transport::{Transport, TransportError};

/// MCP client for communicating with MCP servers.
pub struct McpClient {
    transport: Arc<dyn Transport>,
    request_id: AtomicI64,
    server_capabilities: Option<serde_json::Value>,
}

impl McpClient {
    /// Create a new MCP client.
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self {
            transport,
            request_id: AtomicI64::new(1),
            server_capabilities: None,
        }
    }

    /// Get the next request ID.
    fn next_id(&self) -> i64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Send a request.
    async fn request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<McpResponse, McpClientError> {
        let id = self.next_id();
        let mut request = McpRequest::new(id, method);
        if let Some(p) = params {
            request = request.with_params(p);
        }

        debug!("Sending MCP request: {} (id={})", method, id);

        let response = self.transport.send(request).await?;

        if response.is_error() {
            let err = response.error.unwrap();
            return Err(McpClientError::ServerError {
                code: err.code,
                message: err.message,
            });
        }

        Ok(response)
    }

    /// Initialize the connection.
    pub async fn initialize(&mut self) -> Result<serde_json::Value, McpClientError> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {}
            },
            "clientInfo": {
                "name": "autohands",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        let response = self.request(McpMethod::Initialize.as_str(), Some(params)).await?;
        let result = response.result.unwrap_or(serde_json::Value::Null);
        self.server_capabilities = Some(result.clone());

        info!("MCP connection initialized");
        Ok(result)
    }

    /// List available tools.
    pub async fn list_tools(&self) -> Result<Vec<McpToolDefinition>, McpClientError> {
        let response = self.request(McpMethod::ListTools.as_str(), None).await?;
        let result = response.result.unwrap_or(serde_json::Value::Null);

        let tools: Vec<McpToolDefinition> = result
            .get("tools")
            .and_then(|t| serde_json::from_value(t.clone()).ok())
            .unwrap_or_default();

        Ok(tools)
    }

    /// Call a tool.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpClientError> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        let response = self.request(McpMethod::CallTool.as_str(), Some(params)).await?;
        let result = response.result.unwrap_or(serde_json::Value::Null);

        let tool_result: McpToolResult = serde_json::from_value(result)
            .map_err(|e| McpClientError::ProtocolError(e.to_string()))?;

        Ok(tool_result)
    }

    /// Close the connection.
    pub async fn close(&self) -> Result<(), McpClientError> {
        self.transport.close().await?;
        Ok(())
    }
}

/// MCP client errors.
#[derive(Debug, thiserror::Error)]
pub enum McpClientError {
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("Server error ({code}): {message}")]
    ServerError { code: i32, message: String },

    #[error("Protocol error: {0}")]
    ProtocolError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockTransport {
        responses: Mutex<Vec<McpResponse>>,
    }

    impl MockTransport {
        fn new(responses: Vec<McpResponse>) -> Self {
            Self {
                responses: Mutex::new(responses),
            }
        }
    }

    #[async_trait]
    impl Transport for MockTransport {
        async fn send(&self, _request: McpRequest) -> Result<McpResponse, TransportError> {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err(TransportError::Closed);
            }
            Ok(responses.remove(0))
        }

        async fn close(&self) -> Result<(), TransportError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_initialize() {
        let response = McpResponse::success(
            1i64,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {"name": "test"}
            }),
        );

        let transport = Arc::new(MockTransport::new(vec![response]));
        let mut client = McpClient::new(transport);

        let result = client.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_tools() {
        let response = McpResponse::success(
            1i64,
            serde_json::json!({
                "tools": [
                    {
                        "name": "test_tool",
                        "description": "A test tool",
                        "inputSchema": {"type": "object"}
                    }
                ]
            }),
        );

        let transport = Arc::new(MockTransport::new(vec![response]));
        let client = McpClient::new(transport);

        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_call_tool() {
        let response = McpResponse::success(
            1i64,
            serde_json::json!({
                "content": [{"type": "text", "text": "Hello"}],
                "isError": false
            }),
        );

        let transport = Arc::new(MockTransport::new(vec![response]));
        let client = McpClient::new(transport);

        let result = client.call_tool("test", serde_json::json!({})).await.unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
    }

    #[tokio::test]
    async fn test_server_error() {
        let response = McpResponse::error(
            1i64,
            crate::protocol::McpError::method_not_found(),
        );

        let transport = Arc::new(MockTransport::new(vec![response]));
        let client = McpClient::new(transport);

        let result = client.list_tools().await;
        assert!(matches!(result, Err(McpClientError::ServerError { .. })));
    }
}
