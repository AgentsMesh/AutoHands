//! WebSocket handler implementation.
//!
//! This module handles WebSocket connections and converts messages to RunLoop events.
//! **P0 FIX**: Chat messages are now properly converted to RunLoop events instead of
//! being echoed back directly.

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::runloop_bridge::HybridAppState;
use crate::state::AppState;

use super::message::WsMessage;

/// WebSocket upgrade handler.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// WebSocket upgrade handler with RunLoop support.
pub async fn ws_handler_with_runloop(
    ws: WebSocketUpgrade,
    State(state): State<Arc<HybridAppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket_with_runloop(socket, state))
}

/// Handle a WebSocket connection (direct mode - for backward compatibility).
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let connection_id = Uuid::new_v4().to_string();
    info!("WebSocket connected: {}", connection_id);

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(100);

    // Send connected message
    let connected = WsMessage::Connected {
        connection_id: connection_id.clone(),
    };
    if let Ok(json) = serde_json::to_string(&connected) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    // Spawn sender task
    let sender_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    let tx_clone = tx.clone();
    let conn_id = connection_id.clone();
    let state_clone = state.clone();

    while let Some(result) = receiver.next().await {
        match result {
            Ok(Message::Text(text)) => {
                debug!("Received: {}", text);
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                    if let Err(e) =
                        handle_message_direct(ws_msg, &tx_clone, &conn_id, &state_clone).await
                    {
                        error!("Error handling message: {}", e);
                    }
                } else {
                    warn!("Failed to parse WebSocket message");
                    let _ = tx_clone
                        .send(WsMessage::Error {
                            code: "PARSE_ERROR".to_string(),
                            message: "Failed to parse message".to_string(),
                        })
                        .await;
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket closed: {}", conn_id);
                break;
            }
            Ok(Message::Ping(data)) => {
                debug!("Ping received");
                let _ = data;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup
    sender_task.abort();
    info!("WebSocket disconnected: {}", connection_id);
}

/// Handle a WebSocket connection with RunLoop integration.
///
/// **P0 FIX**: This function properly converts Chat messages to RunLoop events
/// and registers the connection with ApiWsChannel for response routing.
async fn handle_socket_with_runloop(socket: WebSocket, state: Arc<HybridAppState>) {
    let connection_id = Uuid::new_v4().to_string();
    info!("WebSocket connected (RunLoop mode): {}", connection_id);

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(100);

    // Register connection with ApiWsChannel for RunLoop response routing
    state
        .api_ws_channel
        .register_connection(connection_id.clone(), tx.clone());

    // Send connected message
    let connected = WsMessage::Connected {
        connection_id: connection_id.clone(),
    };
    if let Ok(json) = serde_json::to_string(&connected) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    // Spawn sender task
    let sender_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    let tx_clone = tx.clone();
    let conn_id = connection_id.clone();
    let state_clone = state.clone();

    while let Some(result) = receiver.next().await {
        match result {
            Ok(Message::Text(text)) => {
                debug!("Received: {}", text);
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                    if let Err(e) =
                        handle_message_with_runloop(ws_msg, &tx_clone, &conn_id, &state_clone).await
                    {
                        error!("Error handling message: {}", e);
                    }
                } else {
                    warn!("Failed to parse WebSocket message");
                    let _ = tx_clone.send(WsMessage::error("PARSE_ERROR", "Failed to parse message")).await;
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket closed: {}", conn_id);
                break;
            }
            Ok(Message::Ping(data)) => {
                debug!("Ping received");
                let _ = data;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup: unregister from ApiWsChannel
    state.api_ws_channel.unregister_connection(&connection_id);
    sender_task.abort();
    info!("WebSocket disconnected: {}", connection_id);
}

/// Handle a parsed WebSocket message (direct mode).
async fn handle_message_direct(
    msg: WsMessage,
    tx: &tokio::sync::mpsc::Sender<WsMessage>,
    connection_id: &str,
    state: &Arc<AppState>,
) -> Result<(), String> {
    match msg {
        WsMessage::Ping { timestamp } => {
            tx.send(WsMessage::Pong { timestamp })
                .await
                .map_err(|e| e.to_string())?;
        }
        WsMessage::Chat {
            session_id,
            content,
            stream,
        } => {
            let session = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());

            // Execute agent directly
            let message = autohands_protocols::types::Message::user(&content);

            match state
                .agent_runtime
                .execute("general", &session, message)
                .await
            {
                Ok(messages) => {
                    if stream {
                        // Send chunks for streaming
                        for (i, msg) in messages.iter().enumerate() {
                            let content_text = msg.content.text();
                            tx.send(WsMessage::Chunk {
                                session_id: session.clone(),
                                content: content_text.to_string(),
                                index: i as u32,
                            })
                            .await
                            .map_err(|e| e.to_string())?;
                        }
                    }

                    // Send final response
                    let final_content = messages
                        .last()
                        .map(|m| m.content.text().to_string())
                        .unwrap_or_default();

                    tx.send(WsMessage::Response {
                        session_id: session,
                        content: final_content,
                        done: true,
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                }
                Err(e) => {
                    tx.send(WsMessage::error("EXECUTION_ERROR", e.to_string()))
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        WsMessage::Pong { .. } => {
            debug!("Pong received from {}", connection_id);
        }
        _ => {
            warn!("Unhandled message type");
        }
    }
    Ok(())
}

/// Handle a parsed WebSocket message with RunLoop integration.
///
/// **P0 FIX**: Chat messages are converted to RunLoop events and injected into the event queue.
/// A `ReplyAddress` is attached so the RunLoop routes the response back through ApiWsChannel.
async fn handle_message_with_runloop(
    msg: WsMessage,
    tx: &tokio::sync::mpsc::Sender<WsMessage>,
    connection_id: &str,
    state: &Arc<HybridAppState>,
) -> Result<(), String> {
    match msg {
        WsMessage::Ping { timestamp } => {
            tx.send(WsMessage::Pong { timestamp })
                .await
                .map_err(|e| e.to_string())?;
        }
        WsMessage::Chat {
            session_id,
            content,
            stream: _,
        } => {
            let session = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());

            // Create RunLoop event payload
            let payload = serde_json::json!({
                "session_id": session,
                "prompt": content,
                "connection_id": connection_id,
                "agent_id": "general",
            });

            // Construct ReplyAddress so the RunLoop can route the response
            // back through the ApiWsChannel to this specific WebSocket connection.
            // Use thread_id to carry the session_id for response correlation.
            let reply_to = autohands_protocols::channel::ReplyAddress::with_thread(
                "api-ws",
                connection_id,
                &session,
            );

            // Inject event into RunLoop with reply_to
            let runloop_state = state.runloop_state();
            match runloop_state
                .submit_task("agent:execute", payload, Some(reply_to))
                .await
            {
                Ok(()) => {
                    info!("Chat message injected into RunLoop: session={}", session);

                    // Send acknowledgment that execution has started
                    tx.send(WsMessage::execution_started(
                        session.clone(),
                        Some("general".to_string()),
                    ))
                    .await
                    .map_err(|e| e.to_string())?;
                }
                Err(e) => {
                    error!("Failed to inject event into RunLoop: {}", e);
                    tx.send(WsMessage::error(
                        "RUNLOOP_ERROR",
                        format!("Failed to queue task: {}", e),
                    ))
                    .await
                    .map_err(|e| e.to_string())?;
                }
            }
        }
        WsMessage::Pong { .. } => {
            debug!("Pong received from {}", connection_id);
        }
        _ => {
            warn!("Unhandled message type");
        }
    }
    Ok(())
}
