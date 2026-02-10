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
#[path = "client_tests.rs"]
mod tests;
