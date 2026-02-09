//! WebSocket connection management.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use autohands_protocols::channel::{InboundMessage, ReplyAddress};
use autohands_protocols::error::ChannelError;

use crate::WebChannelState;

/// A WebSocket connection to a client.
pub struct WebSocketConnection {
    /// Unique connection ID.
    pub id: String,
    /// Channel for sending messages to the client.
    tx: mpsc::Sender<String>,
    /// Whether the connection is open.
    open: Arc<RwLock<bool>>,
}

impl WebSocketConnection {
    /// Create a new WebSocket connection and spawn the handler task.
    pub fn spawn(
        id: String,
        socket: WebSocket,
        state: Arc<WebChannelState>,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<String>(32);
        let open = Arc::new(RwLock::new(true));

        let conn = Self {
            id: id.clone(),
            tx,
            open: open.clone(),
        };

        // Spawn the connection handler
        tokio::spawn(handle_connection(id, socket, rx, state, open));

        conn
    }

    /// Send a message to the client.
    pub async fn send_message(&self, content: &str) -> Result<(), ChannelError> {
        if !*self.open.read().await {
            return Err(ChannelError::Disconnected);
        }

        self.tx
            .send(content.to_string())
            .await
            .map_err(|e| ChannelError::SendFailed(e.to_string()))
    }

    /// Check if the connection is open.
    pub async fn is_open(&self) -> bool {
        *self.open.read().await
    }

    /// Close the connection.
    pub async fn close(&self) {
        *self.open.write().await = false;
    }
}

/// Handle a WebSocket connection.
async fn handle_connection(
    conn_id: String,
    socket: WebSocket,
    mut outbound_rx: mpsc::Receiver<String>,
    state: Arc<WebChannelState>,
    open: Arc<RwLock<bool>>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    info!("WebSocket connection established: {}", conn_id);

    loop {
        tokio::select! {
            // Handle outbound messages (server -> client)
            Some(content) = outbound_rx.recv() => {
                let msg = serde_json::json!({
                    "type": "message",
                    "content": content,
                });
                if let Err(e) = ws_tx.send(Message::Text(msg.to_string().into())).await {
                    warn!("Failed to send message to {}: {}", conn_id, e);
                    break;
                }
            }

            // Handle inbound messages (client -> server)
            result = ws_rx.next() => {
                match result {
                    Some(Ok(msg)) => {
                        if let Err(e) = handle_message(&conn_id, msg, &state).await {
                            warn!("Failed to handle message from {}: {}", conn_id, e);
                        }
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error from {}: {}", conn_id, e);
                        break;
                    }
                    None => {
                        info!("WebSocket connection closed: {}", conn_id);
                        break;
                    }
                }
            }
        }
    }

    // Mark connection as closed
    *open.write().await = false;

    // Remove from active connections
    state.connections.remove(&conn_id);
    debug!("WebSocket connection removed: {}", conn_id);
}

/// Handle an incoming WebSocket message.
async fn handle_message(
    conn_id: &str,
    msg: Message,
    state: &WebChannelState,
) -> Result<(), ChannelError> {
    match msg {
        Message::Text(text) => {
            // Parse the message JSON
            let parsed: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| ChannelError::ReceiveFailed(format!("Invalid JSON: {}", e)))?;

            let content = parsed
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ChannelError::ReceiveFailed("Missing 'content' field".to_string()))?;

            // Create inbound message
            let inbound = InboundMessage::new(
                uuid::Uuid::new_v4().to_string(),
                content,
                ReplyAddress::new(&state.id, conn_id),
            );

            // Broadcast to listeners
            if let Err(e) = state.inbound_tx.send(inbound) {
                warn!("No receivers for inbound message: {}", e);
            }

            info!("Received message from {}: {}", conn_id, content);
        }
        Message::Binary(_) => {
            debug!("Received binary message from {} (ignored)", conn_id);
        }
        Message::Ping(data) => {
            debug!("Received ping from {}", conn_id);
            // Axum handles pong automatically
            let _ = data;
        }
        Message::Pong(_) => {
            debug!("Received pong from {}", conn_id);
        }
        Message::Close(_) => {
            debug!("Received close from {}", conn_id);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_state_creation() {
        let state = WebChannelState::new("web");
        assert_eq!(state.id, "web");
        assert!(state.connections.is_empty());
    }
}
