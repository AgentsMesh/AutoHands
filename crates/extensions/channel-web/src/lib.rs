//! # AutoHands Channel - Web
//!
//! Web channel providing HTTP/WebSocket based communication for AutoHands.
//!
//! This channel:
//! - Serves a simple HTML/JS UI embedded in the binary
//! - Accepts WebSocket connections for real-time bidirectional communication
//! - Converts user messages to `InboundMessage` and routes agent responses back
//!
//! ## Usage
//!
//! ```ignore
//! use autohands_channel_web::{WebChannel, WebChannelConfig};
//!
//! let config = WebChannelConfig {
//!     host: "127.0.0.1".to_string(),
//!     port: 8080,
//! };
//! let channel = WebChannel::new("web", config);
//! channel.start().await?;
//! ```

mod connection;
mod server;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, info};

use autohands_protocols::channel::{
    Channel, ChannelCapabilities, ChannelId, InboundMessage, OutboundMessage, ReplyAddress,
    SentMessage,
};
use autohands_protocols::error::ChannelError;

pub use connection::WebSocketConnection;
pub use server::create_router;

/// Web channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebChannelConfig {
    /// Host to bind to (default: "127.0.0.1").
    #[serde(default = "default_host")]
    pub host: String,
    /// Port to listen on (default: 8080).
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

impl Default for WebChannelConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

/// Web channel state shared across handlers.
pub struct WebChannelState {
    /// Channel ID.
    pub id: ChannelId,
    /// Active WebSocket connections.
    pub connections: DashMap<String, WebSocketConnection>,
    /// Broadcast sender for inbound messages.
    pub inbound_tx: broadcast::Sender<InboundMessage>,
    /// Channel started flag.
    pub started: AtomicBool,
}

impl WebChannelState {
    /// Create a new channel state.
    pub fn new(id: impl Into<String>) -> Self {
        let (inbound_tx, _) = broadcast::channel(256);
        Self {
            id: id.into(),
            connections: DashMap::new(),
            inbound_tx,
            started: AtomicBool::new(false),
        }
    }
}

/// Web channel for HTTP/WebSocket communication.
pub struct WebChannel {
    /// Channel ID.
    id: ChannelId,
    /// Configuration.
    config: WebChannelConfig,
    /// Channel capabilities.
    capabilities: ChannelCapabilities,
    /// Shared state.
    state: Arc<WebChannelState>,
    /// Server shutdown signal.
    #[allow(dead_code)]
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    /// Server join handle.
    #[allow(dead_code)]
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WebChannel {
    /// Create a new web channel.
    pub fn new(id: impl Into<String>, config: WebChannelConfig) -> Self {
        let id = id.into();
        let state = Arc::new(WebChannelState::new(&id));

        Self {
            id,
            config,
            capabilities: ChannelCapabilities {
                supports_images: false,
                supports_files: false,
                supports_reactions: false,
                supports_threads: false,
                supports_editing: false,
                max_message_length: Some(65536), // 64KB
            },
            state,
            shutdown_tx: None,
            server_handle: None,
        }
    }

    /// Get the channel's listen address.
    pub fn address(&self) -> String {
        format!("{}:{}", self.config.host, self.config.port)
    }

    /// Get a reference to the shared state.
    pub fn state(&self) -> Arc<WebChannelState> {
        self.state.clone()
    }

    /// Check if the channel is started.
    pub fn is_started(&self) -> bool {
        self.state.started.load(Ordering::SeqCst)
    }

    /// Get the number of active connections.
    pub fn connection_count(&self) -> usize {
        self.state.connections.len()
    }
}

#[async_trait]
impl Channel for WebChannel {
    fn id(&self) -> &ChannelId {
        &self.id
    }

    fn capabilities(&self) -> &ChannelCapabilities {
        &self.capabilities
    }

    async fn start(&self) -> Result<(), ChannelError> {
        if self.is_started() {
            return Ok(());
        }

        let addr = self.address();
        let state = self.state.clone();

        // Create the router
        let router = create_router(state.clone());

        // Parse the address
        let listener_addr: std::net::SocketAddr = addr
            .parse()
            .map_err(|e| ChannelError::ConnectionFailed(format!("Invalid address: {}", e)))?;

        // Create TCP listener
        let listener = tokio::net::TcpListener::bind(listener_addr)
            .await
            .map_err(|e| ChannelError::ConnectionFailed(format!("Failed to bind: {}", e)))?;

        info!("Web channel started at http://{}", addr);
        self.state.started.store(true, Ordering::SeqCst);

        // Spawn server task
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                tracing::error!("Web server error: {}", e);
            }
        });

        Ok(())
    }

    async fn stop(&self) -> Result<(), ChannelError> {
        if !self.is_started() {
            return Ok(());
        }

        self.state.started.store(false, Ordering::SeqCst);

        // Close all connections
        for entry in self.state.connections.iter() {
            entry.value().close().await;
        }
        self.state.connections.clear();

        debug!("Web channel stopped");
        Ok(())
    }

    async fn send(
        &self,
        target: &ReplyAddress,
        message: OutboundMessage,
    ) -> Result<SentMessage, ChannelError> {
        if !self.is_started() {
            return Err(ChannelError::Disconnected);
        }

        // target.target is the connection_id
        let conn = self
            .state
            .connections
            .get(&target.target)
            .ok_or_else(|| ChannelError::NotFound(target.target.clone()))?;

        conn.send_message(&message.content).await?;

        Ok(SentMessage {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
        })
    }

    fn inbound(&self) -> broadcast::Receiver<InboundMessage> {
        self.state.inbound_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_channel_config_default() {
        let config = WebChannelConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_web_channel_config_serialization() {
        let config = WebChannelConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("0.0.0.0"));
        assert!(json.contains("3000"));
    }

    #[test]
    fn test_web_channel_config_deserialization() {
        let json = r#"{"host":"localhost","port":9000}"#;
        let config: WebChannelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 9000);
    }

    #[test]
    fn test_web_channel_creation() {
        let config = WebChannelConfig::default();
        let channel = WebChannel::new("web", config);
        assert_eq!(channel.id(), "web");
        assert_eq!(channel.address(), "127.0.0.1:8080");
    }

    #[test]
    fn test_web_channel_capabilities() {
        let channel = WebChannel::new("web", WebChannelConfig::default());
        let caps = channel.capabilities();
        assert!(!caps.supports_images);
        assert!(!caps.supports_files);
        assert_eq!(caps.max_message_length, Some(65536));
    }

    #[test]
    fn test_web_channel_state() {
        let state = WebChannelState::new("web");
        assert_eq!(state.id, "web");
        assert!(state.connections.is_empty());
        assert!(!state.started.load(Ordering::SeqCst));
    }

    #[test]
    fn test_web_channel_inbound_receiver() {
        let channel = WebChannel::new("web", WebChannelConfig::default());
        let _rx = channel.inbound();
        // Should not panic
    }

    #[tokio::test]
    async fn test_send_when_not_started() {
        let channel = WebChannel::new("web", WebChannelConfig::default());
        let target = ReplyAddress::new("web", "conn-123");
        let message = OutboundMessage::text("Hello");

        let result = channel.send(&target, message).await;
        assert!(matches!(result, Err(ChannelError::Disconnected)));
    }
}
