//! Message content types.

use serde::{Deserialize, Serialize};

/// Content of a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    /// Get the text content of the message.
    pub fn text(&self) -> String {
        match self {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    /// Create a text content.
    pub fn from_text(text: impl Into<String>) -> Self {
        MessageContent::Text(text.into())
    }
}

/// A part of a message content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text {
        text: String,
    },
    Image {
        source: ImageSource,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
}

/// Image source for multimodal content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Base64 { media_type: String, data: String },
    Url { url: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_content() {
        let content = MessageContent::Text("Hello".to_string());
        assert_eq!(content.text(), "Hello");
    }

    #[test]
    fn test_from_text() {
        let content = MessageContent::from_text("Test");
        assert_eq!(content.text(), "Test");
    }

    #[test]
    fn test_parts_content() {
        let content = MessageContent::Parts(vec![
            ContentPart::Text { text: "First".to_string() },
            ContentPart::Text { text: "Second".to_string() },
        ]);
        assert_eq!(content.text(), "First\nSecond");
    }

    #[test]
    fn test_content_part_serialization() {
        let part = ContentPart::Text { text: "Hello".to_string() };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("text"));
    }

    #[test]
    fn test_image_source_base64() {
        let source = ImageSource::Base64 {
            media_type: "image/png".to_string(),
            data: "abc123".to_string(),
        };
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("base64"));
    }

    #[test]
    fn test_image_source_url() {
        let source = ImageSource::Url {
            url: "https://example.com/image.png".to_string(),
        };
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("url"));
    }

    #[test]
    fn test_tool_use_part() {
        let part = ContentPart::ToolUse {
            id: "call_1".to_string(),
            name: "test".to_string(),
            input: serde_json::json!({}),
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("tool_use"));
    }

    #[test]
    fn test_tool_result_part() {
        let part = ContentPart::ToolResult {
            tool_use_id: "call_1".to_string(),
            content: "Result".to_string(),
            is_error: false,
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("tool_result"));
    }

    #[test]
    fn test_tool_result_part_error() {
        let part = ContentPart::ToolResult {
            tool_use_id: "call_2".to_string(),
            content: "Error message".to_string(),
            is_error: true,
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("is_error"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_content_part_deserialization() {
        let json = r#"{"type":"text","text":"Hello world"}"#;
        let part: ContentPart = serde_json::from_str(json).unwrap();
        match part {
            ContentPart::Text { text } => assert_eq!(text, "Hello world"),
            _ => panic!("Expected Text part"),
        }
    }

    #[test]
    fn test_image_source_deserialization() {
        let json = r#"{"type":"url","url":"https://example.com/img.png"}"#;
        let source: ImageSource = serde_json::from_str(json).unwrap();
        match source {
            ImageSource::Url { url } => assert_eq!(url, "https://example.com/img.png"),
            _ => panic!("Expected Url source"),
        }
    }

    #[test]
    fn test_message_content_text_clone() {
        let content = MessageContent::Text("test".to_string());
        let cloned = content.clone();
        assert_eq!(cloned.text(), "test");
    }

    #[test]
    fn test_message_content_parts_clone() {
        let content = MessageContent::Parts(vec![
            ContentPart::Text { text: "a".to_string() },
        ]);
        let cloned = content.clone();
        assert_eq!(cloned.text(), "a");
    }

    #[test]
    fn test_content_part_clone() {
        let part = ContentPart::Text { text: "hello".to_string() };
        let cloned = part.clone();
        match cloned {
            ContentPart::Text { text } => assert_eq!(text, "hello"),
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_image_source_clone() {
        let source = ImageSource::Base64 {
            media_type: "image/jpeg".to_string(),
            data: "data".to_string(),
        };
        let cloned = source.clone();
        match cloned {
            ImageSource::Base64 { media_type, data } => {
                assert_eq!(media_type, "image/jpeg");
                assert_eq!(data, "data");
            }
            _ => panic!("Expected Base64"),
        }
    }

    #[test]
    fn test_message_content_debug() {
        let content = MessageContent::Text("test".to_string());
        let debug = format!("{:?}", content);
        assert!(debug.contains("Text"));
    }

    #[test]
    fn test_content_part_debug() {
        let part = ContentPart::Image {
            source: ImageSource::Url { url: "http://example.com".to_string() },
        };
        let debug = format!("{:?}", part);
        assert!(debug.contains("Image"));
    }

    #[test]
    fn test_parts_with_mixed_content() {
        let content = MessageContent::Parts(vec![
            ContentPart::Text { text: "Hello".to_string() },
            ContentPart::Image { source: ImageSource::Url { url: "http://img.com".to_string() } },
            ContentPart::Text { text: "World".to_string() },
        ]);
        // text() should only extract text parts
        assert_eq!(content.text(), "Hello\nWorld");
    }

    #[test]
    fn test_parts_with_only_image() {
        let content = MessageContent::Parts(vec![
            ContentPart::Image { source: ImageSource::Url { url: "http://img.com".to_string() } },
        ]);
        assert_eq!(content.text(), "");
    }

    #[test]
    fn test_empty_text() {
        let content = MessageContent::Text(String::new());
        assert_eq!(content.text(), "");
    }

    #[test]
    fn test_empty_parts() {
        let content = MessageContent::Parts(vec![]);
        assert_eq!(content.text(), "");
    }

    #[test]
    fn test_tool_use_part_full() {
        let part = ContentPart::ToolUse {
            id: "call_abc".to_string(),
            name: "search".to_string(),
            input: serde_json::json!({"query": "rust programming", "limit": 10}),
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("call_abc"));
        assert!(json.contains("search"));
        assert!(json.contains("rust programming"));
    }

    #[test]
    fn test_message_content_serialization_text() {
        let content = MessageContent::Text("simple text".to_string());
        let json = serde_json::to_string(&content).unwrap();
        assert_eq!(json, "\"simple text\"");
    }

    #[test]
    fn test_message_content_serialization_parts() {
        let content = MessageContent::Parts(vec![
            ContentPart::Text { text: "part1".to_string() },
        ]);
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("["));
        assert!(json.contains("part1"));
    }

    #[test]
    fn test_content_roundtrip() {
        let original = MessageContent::Text("roundtrip test".to_string());
        let json = serde_json::to_string(&original).unwrap();
        let parsed: MessageContent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.text(), "roundtrip test");
    }
}
