//! Channel protocol definitions.
//!
//! Channels are adapters for different messaging platforms (HTTP, WebSocket,
//! Telegram, Slack, etc.).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::error::ChannelError;

/// Core trait for message channels.
#[async_trait]
pub trait Channel: Send + Sync {
    /// Returns the channel ID.
    fn id(&self) -> &str;

    /// Returns the channel capabilities.
    fn capabilities(&self) -> &ChannelCapabilities;

    /// Connect to the channel.
    async fn connect(&mut self) -> Result<(), ChannelError>;

    /// Disconnect from the channel.
    async fn disconnect(&mut self) -> Result<(), ChannelError>;

    /// Send a message.
    async fn send(
        &self,
        target: &MessageTarget,
        message: OutgoingMessage,
    ) -> Result<SentMessage, ChannelError>;

    /// Get a receiver for incoming messages.
    fn on_message(&self) -> broadcast::Receiver<IncomingMessage>;
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

/// Target for sending a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTarget {
    pub channel_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// An incoming message from a channel.
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

/// An outgoing message to a channel.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_capabilities_default() {
        let caps = ChannelCapabilities::default();
        assert!(!caps.supports_images);
        assert!(!caps.supports_files);
        assert!(!caps.supports_reactions);
        assert!(!caps.supports_threads);
        assert!(!caps.supports_editing);
        assert!(caps.max_message_length.is_none());
    }

    #[test]
    fn test_channel_capabilities_serialization() {
        let caps = ChannelCapabilities {
            supports_images: true,
            supports_files: true,
            supports_reactions: false,
            supports_threads: true,
            supports_editing: false,
            max_message_length: Some(4096),
        };
        let json = serde_json::to_string(&caps).unwrap();
        assert!(json.contains("supports_images"));
        assert!(json.contains("4096"));
    }

    #[test]
    fn test_channel_capabilities_deserialization() {
        let json = r#"{"supports_images":true,"supports_files":false,"supports_reactions":false,"supports_threads":false,"supports_editing":false,"max_message_length":null}"#;
        let caps: ChannelCapabilities = serde_json::from_str(json).unwrap();
        assert!(caps.supports_images);
        assert!(!caps.supports_files);
    }

    #[test]
    fn test_message_target_serialization() {
        let target = MessageTarget {
            channel_id: "chan-123".to_string(),
            thread_id: Some("thread-456".to_string()),
            user_id: None,
        };
        let json = serde_json::to_string(&target).unwrap();
        assert!(json.contains("chan-123"));
        assert!(json.contains("thread-456"));
        // user_id should be skipped when None
        assert!(!json.contains("user_id"));
    }

    #[test]
    fn test_message_target_deserialization() {
        let json = r#"{"channel_id":"chan-123"}"#;
        let target: MessageTarget = serde_json::from_str(json).unwrap();
        assert_eq!(target.channel_id, "chan-123");
        assert!(target.thread_id.is_none());
        assert!(target.user_id.is_none());
    }

    #[test]
    fn test_incoming_message_serialization() {
        let msg = IncomingMessage {
            id: "msg-123".to_string(),
            channel_id: "chan-456".to_string(),
            sender_id: "user-789".to_string(),
            content: "Hello, world!".to_string(),
            timestamp: chrono::Utc::now(),
            thread_id: None,
            attachments: Vec::new(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("msg-123"));
        assert!(json.contains("Hello, world!"));
    }

    #[test]
    fn test_incoming_message_deserialization() {
        let json = r#"{"id":"msg-123","channel_id":"chan-456","sender_id":"user-789","content":"Test","timestamp":"2024-01-01T00:00:00Z","attachments":[]}"#;
        let msg: IncomingMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.id, "msg-123");
        assert_eq!(msg.content, "Test");
    }

    #[test]
    fn test_outgoing_message_text() {
        let msg = OutgoingMessage::text("Hello!");
        assert_eq!(msg.content, "Hello!");
        assert!(msg.reply_to.is_none());
        assert!(msg.attachments.is_empty());
    }

    #[test]
    fn test_outgoing_message_serialization() {
        let msg = OutgoingMessage {
            content: "Hello!".to_string(),
            reply_to: Some("msg-123".to_string()),
            attachments: Vec::new(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Hello!"));
        assert!(json.contains("msg-123"));
    }

    #[test]
    fn test_sent_message_serialization() {
        let msg = SentMessage {
            id: "msg-123".to_string(),
            timestamp: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("msg-123"));
    }

    #[test]
    fn test_attachment_serialization() {
        let attachment = Attachment {
            name: "test.txt".to_string(),
            content_type: "text/plain".to_string(),
            url: Some("https://example.com/test.txt".to_string()),
            data: None,
        };
        let json = serde_json::to_string(&attachment).unwrap();
        assert!(json.contains("test.txt"));
        assert!(json.contains("text/plain"));
    }

    #[test]
    fn test_attachment_with_data() {
        let attachment = Attachment {
            name: "data.bin".to_string(),
            content_type: "application/octet-stream".to_string(),
            url: None,
            data: Some(vec![1, 2, 3, 4]),
        };
        let json = serde_json::to_string(&attachment).unwrap();
        assert!(json.contains("data.bin"));
    }

    #[test]
    fn test_channel_capabilities_clone() {
        let caps = ChannelCapabilities {
            supports_images: true,
            max_message_length: Some(1000),
            ..Default::default()
        };
        let cloned = caps.clone();
        assert_eq!(cloned.supports_images, caps.supports_images);
        assert_eq!(cloned.max_message_length, caps.max_message_length);
    }

    #[test]
    fn test_message_target_clone() {
        let target = MessageTarget {
            channel_id: "chan-123".to_string(),
            thread_id: Some("thread-456".to_string()),
            user_id: Some("user-789".to_string()),
        };
        let cloned = target.clone();
        assert_eq!(cloned.channel_id, target.channel_id);
        assert_eq!(cloned.thread_id, target.thread_id);
        assert_eq!(cloned.user_id, target.user_id);
    }

    #[test]
    fn test_incoming_message_with_attachments() {
        let msg = IncomingMessage {
            id: "msg-123".to_string(),
            channel_id: "chan-456".to_string(),
            sender_id: "user-789".to_string(),
            content: "File attached".to_string(),
            timestamp: chrono::Utc::now(),
            thread_id: None,
            attachments: vec![Attachment {
                name: "doc.pdf".to_string(),
                content_type: "application/pdf".to_string(),
                url: Some("https://example.com/doc.pdf".to_string()),
                data: None,
            }],
        };
        assert_eq!(msg.attachments.len(), 1);
        assert_eq!(msg.attachments[0].name, "doc.pdf");
    }

    #[test]
    fn test_channel_capabilities_debug() {
        let caps = ChannelCapabilities::default();
        let debug = format!("{:?}", caps);
        assert!(debug.contains("ChannelCapabilities"));
    }

    #[test]
    fn test_message_target_debug() {
        let target = MessageTarget {
            channel_id: "chan".to_string(),
            thread_id: None,
            user_id: None,
        };
        let debug = format!("{:?}", target);
        assert!(debug.contains("MessageTarget"));
    }

    #[test]
    fn test_attachment_debug() {
        let attachment = Attachment {
            name: "test.txt".to_string(),
            content_type: "text/plain".to_string(),
            url: None,
            data: None,
        };
        let debug = format!("{:?}", attachment);
        assert!(debug.contains("Attachment"));
    }
}
