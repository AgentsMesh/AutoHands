//! Channel protocol definitions.
//!
//! Channels are adapters for different messaging platforms (HTTP, WebSocket,
//! Telegram, Slack, etc.).
//!
//! ## Core Concepts
//!
//! - **Channel**: A bidirectional communication adapter (Web, Telegram, WeChat, etc.)
//! - **ReplyAddress**: Enables "reply to origin" pattern - messages are routed back
//!   to where they came from
//! - **InboundMessage**: Messages from users to AutoHands
//! - **OutboundMessage**: Messages from AutoHands to users

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

use crate::error::ChannelError;

#[path = "channel_legacy.rs"]
mod channel_legacy;
pub use channel_legacy::*;

#[cfg(test)]
#[path = "channel_tests.rs"]
mod tests;

/// Channel unique identifier type.
pub type ChannelId = String;

/// Reply address for routing responses back to the source.
///
/// This enables the "reply to origin" pattern where responses are automatically
/// routed back to the channel and target that originated the request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ReplyAddress {
    /// The channel ID (e.g., "web", "telegram", "wechat").
    pub channel_id: ChannelId,
    /// The specific target within the channel (e.g., connection_id, chat_id, user_id).
    pub target: String,
    /// Optional thread/topic ID for threaded conversations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

impl ReplyAddress {
    /// Create a new reply address.
    pub fn new(channel_id: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            channel_id: channel_id.into(),
            target: target.into(),
            thread_id: None,
        }
    }

    /// Create a reply address with a thread ID.
    pub fn with_thread(
        channel_id: impl Into<String>,
        target: impl Into<String>,
        thread_id: impl Into<String>,
    ) -> Self {
        Self {
            channel_id: channel_id.into(),
            target: target.into(),
            thread_id: Some(thread_id.into()),
        }
    }
}

/// Inbound message (User -> AutoHands).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    /// Unique message ID.
    pub id: String,
    /// Message content (natural language).
    pub content: String,
    /// Reply address for routing responses back.
    pub reply_to: ReplyAddress,
    /// Message timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Channel-specific metadata.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    /// Attachments (files, images, etc.).
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

impl InboundMessage {
    /// Create a new inbound message.
    pub fn new(
        id: impl Into<String>,
        content: impl Into<String>,
        reply_to: ReplyAddress,
    ) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            reply_to,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
            attachments: Vec::new(),
        }
    }

    /// Add metadata to the message.
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Outbound message (AutoHands -> User).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    /// Message content.
    pub content: String,
    /// Optional: reply to a specific message ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_message_id: Option<String>,
    /// Channel-specific metadata.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    /// Attachments (files, images, etc.).
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

impl OutboundMessage {
    /// Create a simple text message.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            reply_to_message_id: None,
            metadata: HashMap::new(),
            attachments: Vec::new(),
        }
    }

    /// Create a message that replies to a specific message.
    pub fn reply(content: impl Into<String>, message_id: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            reply_to_message_id: Some(message_id.into()),
            metadata: HashMap::new(),
            attachments: Vec::new(),
        }
    }

    /// Add metadata to the message.
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Add an attachment to the message.
    pub fn with_attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }
}

/// Core trait for message channels.
///
/// Channels are stateless bidirectional communication adapters that:
/// - Receive user input and convert to `InboundMessage`
/// - Send agent replies via `OutboundMessage` to the reply address
#[async_trait]
pub trait Channel: Send + Sync {
    /// Returns the channel ID (e.g., "web", "telegram").
    fn id(&self) -> &ChannelId;

    /// Returns the channel capabilities.
    fn capabilities(&self) -> &ChannelCapabilities;

    /// Start the channel (begin listening for messages).
    async fn start(&self) -> Result<(), ChannelError>;

    /// Stop the channel.
    async fn stop(&self) -> Result<(), ChannelError>;

    /// Send a message to the specified reply address.
    async fn send(
        &self,
        target: &ReplyAddress,
        message: OutboundMessage,
    ) -> Result<SentMessage, ChannelError>;

    /// Get a receiver for inbound messages.
    fn inbound(&self) -> broadcast::Receiver<InboundMessage>;
}

/// Channel capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelCapabilities {
    pub supports_images: bool,
    pub supports_files: bool,
    pub supports_reactions: bool,
    pub supports_threads: bool,
    pub supports_editing: bool,
    pub max_message_length: Option<usize>,
}

/// A sent message confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentMessage {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// An attachment (file, image, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub content_type: String,
    pub url: Option<String>,
    pub data: Option<Vec<u8>>,
}
