//! Channel bridge for integrating channels with the RunLoop.
//!
//! This module provides the `ChannelBridge` which:
//! - Listens for inbound messages from all registered channels
//! - Converts messages to Tasks and injects them into the RunLoop
//! - Triggers the RunLoop wakeup mechanism
//!
//! ## Architecture
//!
//! ```text
//! User Input → Channel → InboundMessage → ChannelBridge → RunLoop.inject_task()
//!                                                              ↓
//! User Output ← Channel ← OutboundMessage ← ChannelRegistry ← Agent ← RunLoop
//! ```

use std::sync::Arc;

use autohands_core::registry::ChannelRegistry;
use autohands_protocols::channel::{InboundMessage, ReplyAddress};
use tracing::{debug, error, info, warn};

use crate::run_loop::RunLoop;
use crate::task::{Task, TaskPriority, TaskSource};

/// Bridge between channels and the RunLoop.
///
/// Listens for inbound messages from all channels and converts them
/// to tasks for processing by the RunLoop.
pub struct ChannelBridge {
    /// Channel registry for accessing all channels.
    channel_registry: Arc<ChannelRegistry>,
    /// RunLoop for injecting tasks.
    run_loop: Arc<RunLoop>,
}

impl ChannelBridge {
    /// Create a new channel bridge.
    pub fn new(
        channel_registry: Arc<ChannelRegistry>,
        run_loop: Arc<RunLoop>,
    ) -> Self {
        Self {
            channel_registry,
            run_loop,
        }
    }

    /// Start listening on all channels.
    ///
    /// This spawns a listener task for each registered channel that:
    /// 1. Subscribes to inbound messages
    /// 2. Converts messages to Tasks
    /// 3. Injects tasks into the RunLoop
    pub async fn start(&self) {
        let channel_ids = self.channel_registry.list_ids();

        if channel_ids.is_empty() {
            info!("No channels registered, ChannelBridge not starting listeners");
            return;
        }

        info!(
            "ChannelBridge starting listeners for {} channel(s): {:?}",
            channel_ids.len(),
            channel_ids
        );

        for channel_id in channel_ids {
            if let Some(channel) = self.channel_registry.get(&channel_id) {
                let mut inbound = channel.inbound();
                let run_loop = self.run_loop.clone();
                let cid = channel_id.clone();

                tokio::spawn(async move {
                    debug!("Channel listener started for: {}", cid);

                    loop {
                        match inbound.recv().await {
                            Ok(msg) => {
                                if let Err(e) = handle_inbound_message(&cid, msg, &run_loop).await {
                                    error!("Failed to handle inbound message: {}", e);
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                warn!("Channel {} lagged, missed {} messages", cid, n);
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                debug!("Channel {} closed, stopping listener", cid);
                                break;
                            }
                        }
                    }
                });
            }
        }
    }
}

/// Handle an inbound message by converting it to a task.
async fn handle_inbound_message(
    channel_id: &str,
    msg: InboundMessage,
    run_loop: &RunLoop,
) -> Result<(), String> {
    let msg_id = msg.id.clone();
    info!(
        "ChannelBridge received message from channel {}: {} (conn: {})",
        channel_id, msg_id, msg.reply_to.target
    );

    // Create a task from the inbound message
    let task = create_task_from_message(msg);

    // Inject task into RunLoop (this also wakes up the RunLoop)
    run_loop
        .inject_task(task)
        .await
        .map_err(|e| format!("Failed to inject task: {}", e))?;

    info!("Task injected into RunLoop for message: {}", msg_id);

    // Explicitly wake up the RunLoop
    run_loop.wakeup(format!("channel_message:{}", channel_id));

    Ok(())
}

/// Create a Task from an InboundMessage.
fn create_task_from_message(msg: InboundMessage) -> Task {
    // Build the payload with message content and session info
    let payload = serde_json::json!({
        "prompt": msg.content,
        "session_id": msg.reply_to.target.clone(),
        "message_id": msg.id,
        "metadata": msg.metadata,
    });

    Task::new("agent:execute", payload)
        .with_source(TaskSource::Custom(format!(
            "channel:{}",
            msg.reply_to.channel_id
        )))
        .with_priority(TaskPriority::Normal)
        .with_reply_to(msg.reply_to)
}

/// Configuration for channel bridge behavior.
#[derive(Debug, Clone)]
pub struct ChannelBridgeConfig {
    /// Default task priority for channel messages.
    pub default_priority: TaskPriority,
    /// Task type for channel messages.
    pub task_type: String,
}

impl Default for ChannelBridgeConfig {
    fn default() -> Self {
        Self {
            default_priority: TaskPriority::Normal,
            task_type: "agent:execute".to_string(),
        }
    }
}

/// Helper to create a reply address from channel and connection info.
pub fn make_reply_address(channel_id: &str, connection_id: &str) -> ReplyAddress {
    ReplyAddress::new(channel_id, connection_id)
}

#[cfg(test)]
#[path = "channel_bridge_tests.rs"]
mod tests;
