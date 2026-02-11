//! API WebSocket Channel implementation.
//!
//! Implements the Channel trait for the API WebSocket interface, enabling
//! the RunLoop to send responses back to specific WebSocket connections.
//! This completes the async result return chain:
//! WebSocket → RunLoop → AgentHandler → RunLoop → Channel → WebSocket client.

use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::{broadcast, mpsc};
use tracing::debug;
use uuid::Uuid;

use autohands_protocols::channel::{
    Channel, ChannelCapabilities, ChannelId, InboundMessage, OutboundMessage, ReplyAddress,
    SentMessage,
};
use autohands_protocols::error::ChannelError;

use super::message::WsMessage;

/// API WebSocket Channel.
///
/// This channel bridges the RunLoop's response delivery mechanism with
/// the WebSocket connections managed by the API server. When the RunLoop
/// completes a task with a `reply_to` address targeting "api-ws", this
/// channel routes the response to the correct WebSocket connection.
pub struct ApiWsChannel {
    /// Channel ID (always "api-ws").
    id: ChannelId,
    /// Channel capabilities.
    capabilities: ChannelCapabilities,
    /// Active connections: connection_id -> mpsc sender for WsMessage.
    connections: DashMap<String, mpsc::Sender<WsMessage>>,
    /// Broadcast sender for inbound messages (not used by this channel,
    /// since WebSocket messages are submitted directly via submit_task).
    inbound_tx: broadcast::Sender<InboundMessage>,
    /// Whether the channel is started.
    started: AtomicBool,
}

impl ApiWsChannel {
    /// Create a new API WebSocket channel.
    pub fn new() -> Self {
        let (inbound_tx, _) = broadcast::channel(256);
        Self {
            id: "api-ws".to_string(),
            capabilities: ChannelCapabilities {
                supports_images: false,
                supports_files: false,
                supports_reactions: false,
                supports_threads: false,
                supports_editing: false,
                max_message_length: Some(65536),
            },
            connections: DashMap::new(),
            inbound_tx,
            started: AtomicBool::new(false),
        }
    }

    /// Register a WebSocket connection.
    ///
    /// Called when a new WebSocket connection is established in the handler.
    pub fn register_connection(&self, id: String, tx: mpsc::Sender<WsMessage>) {
        debug!("ApiWsChannel: registering connection {}", id);
        self.connections.insert(id, tx);
    }

    /// Unregister a WebSocket connection.
    ///
    /// Called when a WebSocket connection is closed.
    pub fn unregister_connection(&self, id: &str) {
        debug!("ApiWsChannel: unregistering connection {}", id);
        self.connections.remove(id);
    }

    /// Get the number of active connections.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

impl Default for ApiWsChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Channel for ApiWsChannel {
    fn id(&self) -> &ChannelId {
        &self.id
    }

    fn capabilities(&self) -> &ChannelCapabilities {
        &self.capabilities
    }

    async fn start(&self) -> Result<(), ChannelError> {
        // This channel doesn't run its own server; it piggybacks on the API server's
        // WebSocket handler. Starting just flips the flag.
        self.started.store(true, Ordering::SeqCst);
        debug!("ApiWsChannel started");
        Ok(())
    }

    async fn stop(&self) -> Result<(), ChannelError> {
        self.started.store(false, Ordering::SeqCst);
        self.connections.clear();
        debug!("ApiWsChannel stopped");
        Ok(())
    }

    async fn send(
        &self,
        target: &ReplyAddress,
        message: OutboundMessage,
    ) -> Result<SentMessage, ChannelError> {
        if !self.started.load(Ordering::SeqCst) {
            return Err(ChannelError::Disconnected);
        }

        let connection_id = &target.target;
        let sender = self
            .connections
            .get(connection_id)
            .ok_or_else(|| ChannelError::NotFound(connection_id.clone()))?;

        // Convert OutboundMessage to WsMessage::Response
        // Use the thread_id as session_id if available, otherwise use the target (connection_id)
        let session_id = target
            .thread_id
            .clone()
            .unwrap_or_else(|| connection_id.clone());

        let ws_msg = WsMessage::Response {
            session_id,
            content: message.content,
            done: true,
        };

        sender
            .send(ws_msg)
            .await
            .map_err(|e| ChannelError::SendFailed(format!("Failed to send to WebSocket: {}", e)))?;

        Ok(SentMessage {
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
        })
    }

    fn inbound(&self) -> broadcast::Receiver<InboundMessage> {
        // This channel doesn't inject via inbound; WebSocket messages go through
        // submit_task directly. Return a receiver that will never get messages
        // from this channel (but could receive from other channels if shared).
        self.inbound_tx.subscribe()
    }
}

#[cfg(test)]
#[path = "channel_tests.rs"]
mod tests;
