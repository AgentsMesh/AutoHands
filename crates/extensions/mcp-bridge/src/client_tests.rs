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
