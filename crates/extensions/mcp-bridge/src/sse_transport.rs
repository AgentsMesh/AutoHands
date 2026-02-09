//! SSE (Server-Sent Events) transport for MCP communication.

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, error, warn};

use crate::protocol::{McpRequest, McpResponse, RequestId};
use crate::transport::{Transport, TransportError};

/// SSE transport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseTransportConfig {
    /// SSE endpoint URL for receiving events.
    pub sse_url: String,
    /// HTTP endpoint URL for sending requests.
    pub http_url: String,
    /// Connection timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Authorization header value.
    pub authorization: Option<String>,
    /// Reconnect delay in milliseconds.
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_ms: u64,
}

fn default_timeout() -> u64 {
    30
}

fn default_reconnect_delay() -> u64 {
    1000
}

/// Pending request awaiting response.
struct PendingRequest {
    sender: oneshot::Sender<Result<McpResponse, TransportError>>,
}

/// SSE transport for MCP servers with bidirectional communication.
pub struct SseTransport {
    client: Client,
    config: SseTransportConfig,
    pending: Arc<Mutex<HashMap<String, PendingRequest>>>,
    closed: AtomicBool,
}

impl SseTransport {
    /// Create a new SSE transport.
    pub async fn connect(config: SseTransportConfig) -> Result<Self, TransportError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| TransportError::Process(e.to_string()))?;

        let pending = Arc::new(Mutex::new(HashMap::new()));

        let transport = Self {
            client: client.clone(),
            config: config.clone(),
            pending: pending.clone(),
            closed: AtomicBool::new(false),
        };

        // Start SSE listener task
        let pending_clone = pending;
        let client_clone = client;
        let config_clone = config;

        tokio::spawn(async move {
            Self::sse_listener(client_clone, config_clone, pending_clone).await;
        });

        Ok(transport)
    }

    async fn sse_listener(
        client: Client,
        config: SseTransportConfig,
        pending: Arc<Mutex<HashMap<String, PendingRequest>>>,
    ) {
        loop {
            let mut req = client.get(&config.sse_url);
            if let Some(ref auth) = config.authorization {
                req = req.header(header::AUTHORIZATION, auth);
            }
            req = req.header(header::ACCEPT, "text/event-stream");

            match req.send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        warn!("SSE connection failed: {}", response.status());
                        tokio::time::sleep(Duration::from_millis(config.reconnect_delay_ms)).await;
                        continue;
                    }

                    debug!("SSE connection established");
                    let mut stream = response.bytes_stream();

                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                                    Self::process_sse_data(&text, &pending).await;
                                }
                            }
                            Err(e) => {
                                error!("SSE stream error: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("SSE connection error: {}", e);
                }
            }

            debug!("SSE reconnecting...");
            tokio::time::sleep(Duration::from_millis(config.reconnect_delay_ms)).await;
        }
    }

    async fn process_sse_data(data: &str, pending: &Arc<Mutex<HashMap<String, PendingRequest>>>) {
        for line in data.lines() {
            if let Some(json_data) = line.strip_prefix("data: ") {
                if let Ok(response) = serde_json::from_str::<McpResponse>(json_data) {
                    let id_str = Self::request_id_to_string(&response.id);
                    let mut pending_guard = pending.lock().await;
                    if let Some(req) = pending_guard.remove(&id_str) {
                        let _ = req.sender.send(Ok(response));
                    }
                }
            }
        }
    }

    fn request_id_to_string(id: &RequestId) -> String {
        match id {
            RequestId::Number(n) => n.to_string(),
            RequestId::String(s) => s.clone(),
        }
    }
}

#[async_trait]
impl Transport for SseTransport {
    async fn send(&self, request: McpRequest) -> Result<McpResponse, TransportError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(TransportError::Closed);
        }

        // Get request ID as string for tracking
        let id_str = Self::request_id_to_string(&request.id);

        // Create pending request
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id_str.clone(), PendingRequest { sender: tx });
        }

        // Send HTTP request
        let mut req = self.client.post(&self.config.http_url);
        if let Some(ref auth) = self.config.authorization {
            req = req.header(header::AUTHORIZATION, auth);
        }
        req = req.header(header::CONTENT_TYPE, "application/json");

        let send_result = req.json(&request).send().await;

        if let Err(e) = send_result {
            let mut pending = self.pending.lock().await;
            pending.remove(&id_str);
            return Err(TransportError::Process(e.to_string()));
        }

        // Wait for response via SSE
        match tokio::time::timeout(
            Duration::from_secs(self.config.timeout_seconds),
            rx,
        )
        .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(TransportError::Closed),
            Err(_) => {
                let mut pending = self.pending.lock().await;
                pending.remove(&id_str);
                Err(TransportError::Process("Request timeout".to_string()))
            }
        }
    }

    async fn close(&self) -> Result<(), TransportError> {
        self.closed.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> SseTransportConfig {
        SseTransportConfig {
            sse_url: "http://localhost:8080/sse".to_string(),
            http_url: "http://localhost:8080/mcp".to_string(),
            timeout_seconds: 30,
            authorization: None,
            reconnect_delay_ms: 1000,
        }
    }

    #[test]
    fn test_sse_config_defaults() {
        let json = serde_json::json!({
            "sse_url": "http://localhost/sse",
            "http_url": "http://localhost/mcp"
        });
        let config: SseTransportConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.reconnect_delay_ms, 1000);
    }

    #[test]
    fn test_sse_config_serialization() {
        let config = create_test_config();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("sse_url"));
        assert!(json.contains("http_url"));
    }

    #[test]
    fn test_sse_config_with_all_fields() {
        let config = SseTransportConfig {
            sse_url: "https://api.example.com/sse".to_string(),
            http_url: "https://api.example.com/mcp".to_string(),
            timeout_seconds: 60,
            authorization: Some("Bearer token123".to_string()),
            reconnect_delay_ms: 2000,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("https://api.example.com/sse"));
        assert!(json.contains("60"));
        assert!(json.contains("Bearer token123"));
        assert!(json.contains("2000"));
    }

    #[test]
    fn test_sse_config_deserialization() {
        let json = r#"{
            "sse_url": "http://test/sse",
            "http_url": "http://test/mcp",
            "timeout_seconds": 45,
            "authorization": "API-Key xyz",
            "reconnect_delay_ms": 500
        }"#;
        let config: SseTransportConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.sse_url, "http://test/sse");
        assert_eq!(config.http_url, "http://test/mcp");
        assert_eq!(config.timeout_seconds, 45);
        assert_eq!(config.authorization, Some("API-Key xyz".to_string()));
        assert_eq!(config.reconnect_delay_ms, 500);
    }

    #[test]
    fn test_sse_config_without_authorization() {
        let json = r#"{
            "sse_url": "http://test/sse",
            "http_url": "http://test/mcp"
        }"#;
        let config: SseTransportConfig = serde_json::from_str(json).unwrap();
        assert!(config.authorization.is_none());
    }

    #[test]
    fn test_sse_config_clone() {
        let config = create_test_config();
        let cloned = config.clone();
        assert_eq!(cloned.sse_url, config.sse_url);
        assert_eq!(cloned.http_url, config.http_url);
        assert_eq!(cloned.timeout_seconds, config.timeout_seconds);
    }

    #[test]
    fn test_sse_config_debug() {
        let config = create_test_config();
        let debug = format!("{:?}", config);
        assert!(debug.contains("SseTransportConfig"));
        assert!(debug.contains("localhost"));
    }

    #[test]
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 30);
    }

    #[test]
    fn test_default_reconnect_delay() {
        assert_eq!(default_reconnect_delay(), 1000);
    }

    #[test]
    fn test_request_id_to_string_number() {
        let id = RequestId::Number(42);
        assert_eq!(SseTransport::request_id_to_string(&id), "42");
    }

    #[test]
    fn test_request_id_to_string_string() {
        let id = RequestId::String("req-123".to_string());
        assert_eq!(SseTransport::request_id_to_string(&id), "req-123");
    }

    #[test]
    fn test_request_id_to_string_zero() {
        let id = RequestId::Number(0);
        assert_eq!(SseTransport::request_id_to_string(&id), "0");
    }

    #[test]
    fn test_request_id_to_string_empty_string() {
        let id = RequestId::String(String::new());
        assert_eq!(SseTransport::request_id_to_string(&id), "");
    }
}
