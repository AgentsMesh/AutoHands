//! Message types for conversations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::content::MessageContent;
use super::common::Metadata;

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender.
    pub role: MessageRole,

    /// Content of the message.
    pub content: MessageContent,

    /// Optional name for the sender.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Tool calls made in this message (for assistant messages).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tool_calls: Vec<ToolCall>,

    /// Tool call ID this message is responding to (for tool messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// Additional metadata.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub metadata: Metadata,
}

impl Message {
    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(content.into()),
            name: None,
            tool_calls: Vec::new(),
            tool_call_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content.into()),
            name: None,
            tool_calls: Vec::new(),
            tool_call_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: MessageContent::Text(content.into()),
            name: None,
            tool_calls: Vec::new(),
            tool_call_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a tool response message.
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: MessageContent::Text(content.into()),
            name: None,
            tool_calls: Vec::new(),
            tool_call_id: Some(tool_call_id.into()),
            metadata: HashMap::new(),
        }
    }
}

/// Role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A tool call made by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_message() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content.text(), "Hello");
        assert!(msg.tool_calls.is_empty());
    }

    #[test]
    fn test_assistant_message() {
        let msg = Message::assistant("Hi there");
        assert_eq!(msg.role, MessageRole::Assistant);
        assert_eq!(msg.content.text(), "Hi there");
    }

    #[test]
    fn test_system_message() {
        let msg = Message::system("You are helpful");
        assert_eq!(msg.role, MessageRole::System);
        assert_eq!(msg.content.text(), "You are helpful");
    }

    #[test]
    fn test_tool_message() {
        let msg = Message::tool("call_123", "Result");
        assert_eq!(msg.role, MessageRole::Tool);
        assert_eq!(msg.tool_call_id, Some("call_123".to_string()));
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::user("Test");
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, MessageRole::User);
    }

    #[test]
    fn test_tool_call() {
        let tc = ToolCall {
            id: "call_1".to_string(),
            name: "test_tool".to_string(),
            arguments: serde_json::json!({"key": "value"}),
        };
        assert_eq!(tc.id, "call_1");
        assert_eq!(tc.name, "test_tool");
    }

    #[test]
    fn test_message_role_serialization() {
        let roles = vec![
            (MessageRole::System, "\"system\""),
            (MessageRole::User, "\"user\""),
            (MessageRole::Assistant, "\"assistant\""),
            (MessageRole::Tool, "\"tool\""),
        ];
        for (role, expected) in roles {
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn test_message_role_deserialization() {
        let role: MessageRole = serde_json::from_str("\"system\"").unwrap();
        assert_eq!(role, MessageRole::System);

        let role: MessageRole = serde_json::from_str("\"user\"").unwrap();
        assert_eq!(role, MessageRole::User);
    }

    #[test]
    fn test_message_role_eq() {
        assert_eq!(MessageRole::User, MessageRole::User);
        assert_ne!(MessageRole::User, MessageRole::Assistant);
    }

    #[test]
    fn test_message_role_clone() {
        let role = MessageRole::System;
        let cloned = role;
        assert_eq!(cloned, MessageRole::System);
    }

    #[test]
    fn test_message_role_debug() {
        let debug = format!("{:?}", MessageRole::Assistant);
        assert!(debug.contains("Assistant"));
    }

    #[test]
    fn test_tool_call_clone() {
        let tc = ToolCall {
            id: "id".to_string(),
            name: "name".to_string(),
            arguments: serde_json::json!(null),
        };
        let cloned = tc.clone();
        assert_eq!(cloned.id, "id");
        assert_eq!(cloned.name, "name");
    }

    #[test]
    fn test_tool_call_debug() {
        let tc = ToolCall {
            id: "id".to_string(),
            name: "name".to_string(),
            arguments: serde_json::json!({"a": 1}),
        };
        let debug = format!("{:?}", tc);
        assert!(debug.contains("ToolCall"));
        assert!(debug.contains("id"));
    }

    #[test]
    fn test_tool_call_serialization() {
        let tc = ToolCall {
            id: "call_123".to_string(),
            name: "my_tool".to_string(),
            arguments: serde_json::json!({"param": "value"}),
        };
        let json = serde_json::to_string(&tc).unwrap();
        assert!(json.contains("call_123"));
        assert!(json.contains("my_tool"));
        assert!(json.contains("param"));
    }

    #[test]
    fn test_message_with_tool_calls() {
        let msg = Message {
            role: MessageRole::Assistant,
            content: MessageContent::Text(String::new()),
            name: None,
            tool_calls: vec![
                ToolCall {
                    id: "call_1".to_string(),
                    name: "tool1".to_string(),
                    arguments: serde_json::json!({}),
                },
            ],
            tool_call_id: None,
            metadata: HashMap::new(),
        };
        assert_eq!(msg.tool_calls.len(), 1);
        assert_eq!(msg.tool_calls[0].name, "tool1");
    }

    #[test]
    fn test_message_with_name() {
        let msg = Message {
            role: MessageRole::User,
            content: MessageContent::Text("hello".to_string()),
            name: Some("John".to_string()),
            tool_calls: Vec::new(),
            tool_call_id: None,
            metadata: HashMap::new(),
        };
        assert_eq!(msg.name, Some("John".to_string()));
    }

    #[test]
    fn test_message_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), serde_json::json!("value"));

        let msg = Message {
            role: MessageRole::User,
            content: MessageContent::Text("test".to_string()),
            name: None,
            tool_calls: Vec::new(),
            tool_call_id: None,
            metadata,
        };
        assert!(msg.metadata.contains_key("key"));
    }

    #[test]
    fn test_message_clone() {
        let msg = Message::user("test");
        let cloned = msg.clone();
        assert_eq!(cloned.role, MessageRole::User);
        assert_eq!(cloned.content.text(), "test");
    }

    #[test]
    fn test_message_debug() {
        let msg = Message::user("debug test");
        let debug = format!("{:?}", msg);
        assert!(debug.contains("Message"));
        assert!(debug.contains("User"));
    }

    #[test]
    fn test_full_message_serialization() {
        let msg = Message {
            role: MessageRole::Assistant,
            content: MessageContent::Text("response".to_string()),
            name: Some("bot".to_string()),
            tool_calls: vec![ToolCall {
                id: "c1".to_string(),
                name: "search".to_string(),
                arguments: serde_json::json!({"q": "test"}),
            }],
            tool_call_id: None,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("assistant"));
        assert!(json.contains("bot"));
        assert!(json.contains("search"));
    }

    #[test]
    fn test_message_deserialization() {
        let json = r#"{"role":"user","content":"Hello world"}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content.text(), "Hello world");
    }
}
