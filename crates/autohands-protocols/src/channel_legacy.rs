//! Legacy channel types for backward compatibility.
//!
//! These types will be deprecated in future versions.
//! Use `ReplyAddress`, `InboundMessage`, and `OutboundMessage` instead.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{Attachment, InboundMessage, OutboundMessage, ReplyAddress};

/// Target for sending a message (legacy, use ReplyAddress instead).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTarget {
    pub channel_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl From<ReplyAddress> for MessageTarget {
    fn from(addr: ReplyAddress) -> Self {
        Self {
            channel_id: addr.channel_id,
            thread_id: addr.thread_id,
            user_id: Some(addr.target),
        }
    }
}

impl From<MessageTarget> for ReplyAddress {
    fn from(target: MessageTarget) -> Self {
        Self {
            channel_id: target.channel_id,
            target: target.user_id.unwrap_or_default(),
            thread_id: target.thread_id,
        }
    }
}

/// An incoming message from a channel (legacy, use InboundMessage instead).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    pub id: String,
    pub channel_id: String,
    pub sender_id: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

impl From<InboundMessage> for IncomingMessage {
    fn from(msg: InboundMessage) -> Self {
        Self {
            id: msg.id,
            channel_id: msg.reply_to.channel_id,
            sender_id: msg.reply_to.target,
            content: msg.content,
            timestamp: msg.timestamp,
            thread_id: msg.reply_to.thread_id,
            attachments: msg.attachments,
        }
    }
}

/// An outgoing message to a channel (legacy, use OutboundMessage instead).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

impl OutgoingMessage {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            reply_to: None,
            attachments: Vec::new(),
        }
    }
}

impl From<OutboundMessage> for OutgoingMessage {
    fn from(msg: OutboundMessage) -> Self {
        Self {
            content: msg.content,
            reply_to: msg.reply_to_message_id,
            attachments: msg.attachments,
        }
    }
}

impl From<OutgoingMessage> for OutboundMessage {
    fn from(msg: OutgoingMessage) -> Self {
        Self {
            content: msg.content,
            reply_to_message_id: msg.reply_to,
            metadata: HashMap::new(),
            attachments: msg.attachments,
        }
    }
}
