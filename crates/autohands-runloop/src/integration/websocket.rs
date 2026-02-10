//! WebSocket integration with RunLoop.
//!
//! Provides a Source1 for handling WebSocket messages.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::error::RunLoopResult;
use crate::task::{Task, TaskPriority, TaskSource};
use crate::mode::RunLoopMode;
use crate::source::{PortMessage, Source1, Source1Receiver};

/// WebSocket message types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessageType {
    /// Chat message from user.
    Chat {
        session_id: Option<String>,
        content: String,
        #[serde(default)]
        stream: bool,
    },
    /// Ping message.
    Ping { timestamp: i64 },
    /// Pong response.
    Pong { timestamp: i64 },
    /// Connection established.
    Connected { connection_id: String },
    /// Error message.
    Error { code: String, message: String },
}

/// WebSocket Source1.
///
/// Receives WebSocket messages and produces RunLoop events.
pub struct WebSocketSource1 {
    id: String,
    cancelled: AtomicBool,
    modes: Vec<RunLoopMode>,
}

impl WebSocketSource1 {
    /// Create a new WebSocket source.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            cancelled: AtomicBool::new(false),
            modes: vec![RunLoopMode::Default, RunLoopMode::Common],
        }
    }

    /// Create a Source1Receiver for this source.
    ///
    /// Returns the receiver and a sender for WebSocket handlers to use.
    pub fn create_receiver(self) -> (Source1Receiver, WebSocketSender) {
        let (tx, rx) = mpsc::channel(256);
        let source = Arc::new(self);
        let sender = WebSocketSender { sender: tx };
        (Source1Receiver::new(source, rx), sender)
    }

    /// Create a PortMessage from a chat message.
    pub fn create_chat_message(
        session_id: Option<String>,
        content: impl Into<String>,
        connection_id: impl Into<String>,
    ) -> PortMessage {
        PortMessage::new(
            "websocket",
            json!({
                "type": "chat",
                "session_id": session_id,
                "content": content.into(),
                "connection_id": connection_id.into(),
            }),
        )
    }
}

impl Default for WebSocketSource1 {
    fn default() -> Self {
        Self::new("websocket")
    }
}

#[async_trait]
impl Source1 for WebSocketSource1 {
    fn id(&self) -> &str {
        &self.id
    }

    async fn handle(&self, msg: PortMessage) -> RunLoopResult<Vec<Task>> {
        let msg_type = msg.payload.get("type").and_then(|v| v.as_str());

        match msg_type {
            Some("chat") => {
                let content = msg
                    .payload
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let session_id = msg
                    .payload
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let connection_id = msg
                    .payload
                    .get("connection_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                info!("WebSocket chat message: {} chars", content.len());

                let event = Task::new(
                    "agent:execute",
                    json!({
                        "prompt": content,
                        "session_id": session_id,
                        "connection_id": connection_id,
                        "source": "websocket",
                    }),
                )
                .with_source(TaskSource::WebSocket)
                .with_priority(TaskPriority::High);

                Ok(vec![event])
            }
            Some("ping") => {
                debug!("WebSocket ping received");
                // Ping doesn't generate RunLoop events
                Ok(vec![])
            }
            _ => {
                debug!("Unknown WebSocket message type: {:?}", msg_type);
                Ok(vec![])
            }
        }
    }

    fn modes(&self) -> &[RunLoopMode] {
        &self.modes
    }

    fn is_valid(&self) -> bool {
        !self.cancelled.load(Ordering::SeqCst)
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

/// WebSocket sender for handlers to use.
#[derive(Clone)]
pub struct WebSocketSender {
    sender: mpsc::Sender<PortMessage>,
}

impl WebSocketSender {
    /// Send a chat message.
    pub async fn send_chat(
        &self,
        session_id: Option<String>,
        content: impl Into<String>,
        connection_id: impl Into<String>,
    ) -> Result<(), mpsc::error::SendError<PortMessage>> {
        let msg = WebSocketSource1::create_chat_message(session_id, content, connection_id);
        self.sender.send(msg).await
    }

    /// Send a raw port message.
    pub async fn send(&self, msg: PortMessage) -> Result<(), mpsc::error::SendError<PortMessage>> {
        self.sender.send(msg).await
    }
}

/// HTTP task event injector.
///
/// Provides a convenient way to inject task events from HTTP handlers.
#[derive(Clone)]
pub struct HttpTaskInjector {
    run_loop_tx: mpsc::Sender<crate::WakeupSignal>,
    task_queue: Arc<crate::TaskQueue>,
}

impl HttpTaskInjector {
    /// Create a new HTTP task injector.
    pub fn new(
        run_loop_tx: mpsc::Sender<crate::WakeupSignal>,
        task_queue: Arc<crate::TaskQueue>,
    ) -> Self {
        Self {
            run_loop_tx,
            task_queue,
        }
    }

    /// Inject a task execution event.
    pub async fn inject_task(
        &self,
        task: impl Into<String>,
        session_id: impl Into<String>,
        agent_id: Option<String>,
    ) -> RunLoopResult<()> {
        let event = Task::new(
            "agent:execute",
            json!({
                "prompt": task.into(),
                "session_id": session_id.into(),
                "agent": agent_id.unwrap_or_else(|| "general".to_string()),
                "source": "http",
            }),
        )
        .with_source(TaskSource::User)
        .with_priority(TaskPriority::Normal);

        self.task_queue.enqueue(event).await?;

        // Wakeup the RunLoop
        let _ = self
            .run_loop_tx
            .send(crate::WakeupSignal::Explicit {
                reason: "http_task_submitted".to_string(),
            })
            .await;

        Ok(())
    }
}

#[cfg(test)]
#[path = "websocket_tests.rs"]
mod tests;
