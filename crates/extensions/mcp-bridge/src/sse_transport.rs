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
#[path = "sse_transport_tests.rs"]
mod tests;
