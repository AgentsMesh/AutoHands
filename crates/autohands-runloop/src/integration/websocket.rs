//! WebSocket integration with RunLoop.
//!
//! WebSocket events are handled via the `WebChannel` + `ChannelBridge` pattern.
//! This module provides `HttpTaskInjector` for direct task injection from HTTP handlers,
//! and `WsMessageType` for WebSocket protocol message definitions.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;

use crate::error::RunLoopResult;
use crate::task::{Task, TaskPriority, TaskSource};

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
