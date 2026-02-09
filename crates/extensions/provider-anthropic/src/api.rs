//! Anthropic API types.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Anthropic API request.
#[derive(Debug, Serialize)]
pub struct ApiRequest {
    pub model: String,
    pub messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ApiTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// API message format.
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: String,
    pub content: ApiContent,
}

/// API content (string or array).
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

/// Content block.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String },
}

/// API tool definition.
#[derive(Debug, Serialize)]
pub struct ApiTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// API response.
#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: String,
    pub usage: ApiUsage,
}

/// API usage.
#[derive(Debug, Deserialize)]
pub struct ApiUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Streaming event.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart { message: StreamMessage },
    ContentBlockStart { index: usize, content_block: ContentBlock },
    ContentBlockDelta { index: usize, delta: StreamDelta },
    ContentBlockStop { index: usize },
    MessageDelta { delta: MessageDelta, usage: Option<ApiUsage> },
    MessageStop,
    Ping,
    Error { error: ApiError },
}

#[derive(Debug, Deserialize)]
pub struct StreamMessage {
    pub id: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_request_serialization() {
        let request = ApiRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: ApiContent::Text("Hello".to_string()),
            }],
            system: Some("You are helpful".to_string()),
            max_tokens: 1024,
            temperature: Some(0.5),
            tools: vec![],
            stream: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "claude-sonnet-4-20250514");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["system"], "You are helpful");
        // Check temperature is present and approximately 0.5
        assert!(json["temperature"].as_f64().unwrap() > 0.49);
        assert!(json["temperature"].as_f64().unwrap() < 0.51);
    }

    #[test]
    fn test_api_request_skip_none_fields() {
        let request = ApiRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            messages: vec![],
            system: None,
            max_tokens: 1024,
            temperature: None,
            tools: vec![],
            stream: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("system").is_none());
        assert!(json.get("temperature").is_none());
        assert!(json.get("stream").is_none());
        // Empty tools should be skipped
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn test_api_content_text() {
        let content = ApiContent::Text("Hello world".to_string());
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json, "Hello world");
    }

    #[test]
    fn test_api_content_blocks() {
        let content = ApiContent::Blocks(vec![
            ContentBlock::Text { text: "Hello".to_string() },
        ]);
        let json = serde_json::to_value(&content).unwrap();
        assert!(json.is_array());
        assert_eq!(json[0]["type"], "text");
        assert_eq!(json[0]["text"], "Hello");
    }

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::Text { text: "Hello".to_string() };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "Hello");
    }

    #[test]
    fn test_content_block_tool_use() {
        let block = ContentBlock::ToolUse {
            id: "toolu_123".to_string(),
            name: "search".to_string(),
            input: serde_json::json!({"query": "rust"}),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "tool_use");
        assert_eq!(json["id"], "toolu_123");
        assert_eq!(json["name"], "search");
        assert_eq!(json["input"]["query"], "rust");
    }

    #[test]
    fn test_content_block_tool_result() {
        let block = ContentBlock::ToolResult {
            tool_use_id: "toolu_123".to_string(),
            content: "search results".to_string(),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "tool_result");
        assert_eq!(json["tool_use_id"], "toolu_123");
        assert_eq!(json["content"], "search results");
    }

    #[test]
    fn test_api_tool_serialization() {
        let tool = ApiTool {
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                }
            }),
        };
        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["name"], "read_file");
        assert_eq!(json["description"], "Read a file");
        assert_eq!(json["input_schema"]["type"], "object");
    }

    #[test]
    fn test_api_response_deserialization() {
        let json = serde_json::json!({
            "id": "msg_123",
            "model": "claude-sonnet-4-20250514",
            "content": [{"type": "text", "text": "Hello!"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });

        let response: ApiResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.id, "msg_123");
        assert_eq!(response.model, "claude-sonnet-4-20250514");
        assert_eq!(response.content.len(), 1);
        assert_eq!(response.stop_reason, "end_turn");
        assert_eq!(response.usage.input_tokens, 10);
        assert_eq!(response.usage.output_tokens, 5);
    }

    #[test]
    fn test_stream_event_message_start() {
        let json = serde_json::json!({
            "type": "message_start",
            "message": {
                "id": "msg_stream",
                "model": "claude-sonnet-4-20250514"
            }
        });

        let event: StreamEvent = serde_json::from_value(json).unwrap();
        match event {
            StreamEvent::MessageStart { message } => {
                assert_eq!(message.id, "msg_stream");
                assert_eq!(message.model, "claude-sonnet-4-20250514");
            }
            _ => panic!("Expected MessageStart"),
        }
    }

    #[test]
    fn test_stream_event_content_block_delta_text() {
        let json = serde_json::json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": "Hello"
            }
        });

        let event: StreamEvent = serde_json::from_value(json).unwrap();
        match event {
            StreamEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 0);
                match delta {
                    StreamDelta::TextDelta { text } => assert_eq!(text, "Hello"),
                    _ => panic!("Expected TextDelta"),
                }
            }
            _ => panic!("Expected ContentBlockDelta"),
        }
    }

    #[test]
    fn test_stream_event_content_block_delta_json() {
        let json = serde_json::json!({
            "type": "content_block_delta",
            "index": 1,
            "delta": {
                "type": "input_json_delta",
                "partial_json": "{\"query\":"
            }
        });

        let event: StreamEvent = serde_json::from_value(json).unwrap();
        match event {
            StreamEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 1);
                match delta {
                    StreamDelta::InputJsonDelta { partial_json } => {
                        assert_eq!(partial_json, "{\"query\":");
                    }
                    _ => panic!("Expected InputJsonDelta"),
                }
            }
            _ => panic!("Expected ContentBlockDelta"),
        }
    }

    #[test]
    fn test_stream_event_message_stop() {
        let json = serde_json::json!({
            "type": "message_stop"
        });

        let event: StreamEvent = serde_json::from_value(json).unwrap();
        assert!(matches!(event, StreamEvent::MessageStop));
    }

    #[test]
    fn test_stream_event_ping() {
        let json = serde_json::json!({
            "type": "ping"
        });

        let event: StreamEvent = serde_json::from_value(json).unwrap();
        assert!(matches!(event, StreamEvent::Ping));
    }

    #[test]
    fn test_stream_event_error() {
        let json = serde_json::json!({
            "type": "error",
            "error": {
                "type": "rate_limit_error",
                "message": "Too many requests"
            }
        });

        let event: StreamEvent = serde_json::from_value(json).unwrap();
        match event {
            StreamEvent::Error { error } => {
                assert_eq!(error.error_type, "rate_limit_error");
                assert_eq!(error.message, "Too many requests");
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_api_message_roundtrip() {
        let message = ApiMessage {
            role: "user".to_string(),
            content: ApiContent::Text("Hello".to_string()),
        };

        let json = serde_json::to_value(&message).unwrap();
        let parsed: ApiMessage = serde_json::from_value(json).unwrap();

        assert_eq!(parsed.role, "user");
        match parsed.content {
            ApiContent::Text(t) => assert_eq!(t, "Hello"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_content_block_deserialization() {
        let json = serde_json::json!({
            "type": "tool_use",
            "id": "toolu_abc",
            "name": "read_file",
            "input": {"path": "/tmp/file.txt"}
        });

        let block: ContentBlock = serde_json::from_value(json).unwrap();
        match block {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "toolu_abc");
                assert_eq!(name, "read_file");
                assert_eq!(input["path"], "/tmp/file.txt");
            }
            _ => panic!("Expected ToolUse"),
        }
    }
}
