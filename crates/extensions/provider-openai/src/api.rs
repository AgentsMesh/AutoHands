//! OpenAI API types.
//! Fields are required for serde deserialization of API responses.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// OpenAI API request.
#[derive(Debug, Serialize)]
pub struct ApiRequest {
    pub model: String,
    pub messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ApiTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

/// API message format.
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Message content (string or array).
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

/// Content part for multimodal messages.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

/// Image URL for vision.
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Tool call in response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// Function call details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// API tool definition.
#[derive(Debug, Serialize)]
pub struct ApiTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDef,
}

/// Function definition for tools.
#[derive(Debug, Serialize)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Response format specification.
#[derive(Debug, Serialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

/// API response.
#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<ApiUsage>,
}

/// Response choice.
#[derive(Debug, Deserialize)]
pub struct Choice {
    pub index: usize,
    pub message: ResponseMessage,
    pub finish_reason: Option<String>,
}

/// Response message.
#[derive(Debug, Deserialize)]
pub struct ResponseMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}

/// API usage statistics.
#[derive(Debug, Deserialize)]
pub struct ApiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Streaming chunk.
#[derive(Debug, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub choices: Vec<StreamChoice>,
    pub usage: Option<ApiUsage>,
}

/// Streaming choice.
#[derive(Debug, Deserialize)]
pub struct StreamChoice {
    pub index: usize,
    pub delta: StreamDelta,
    pub finish_reason: Option<String>,
}

/// Streaming delta content.
#[derive(Debug, Deserialize)]
pub struct StreamDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<StreamToolCall>>,
}

/// Streaming tool call (partial).
#[derive(Debug, Deserialize)]
pub struct StreamToolCall {
    pub index: usize,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub call_type: Option<String>,
    pub function: Option<StreamFunctionCall>,
}

/// Streaming function call (partial).
#[derive(Debug, Deserialize)]
pub struct StreamFunctionCall {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_request_serialization() {
        let request = ApiRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: Some(MessageContent::Text("Hello".to_string())),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            max_tokens: Some(1024),
            temperature: Some(0.5),
            tools: vec![],
            stream: Some(true),
            response_format: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["stream"], true);
    }

    #[test]
    fn test_api_request_skip_none_fields() {
        let request = ApiRequest {
            model: "gpt-4o".to_string(),
            messages: vec![],
            max_tokens: None,
            temperature: None,
            tools: vec![],
            stream: None,
            response_format: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("max_tokens").is_none());
        assert!(json.get("temperature").is_none());
        assert!(json.get("stream").is_none());
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn test_message_content_text() {
        let content = MessageContent::Text("Hello world".to_string());
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json, "Hello world");
    }

    #[test]
    fn test_message_content_parts() {
        let content = MessageContent::Parts(vec![
            ContentPart::Text { text: "Hello".to_string() },
        ]);
        let json = serde_json::to_value(&content).unwrap();
        assert!(json.is_array());
        assert_eq!(json[0]["type"], "text");
        assert_eq!(json[0]["text"], "Hello");
    }

    #[test]
    fn test_content_part_text() {
        let part = ContentPart::Text { text: "Hello".to_string() };
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "Hello");
    }

    #[test]
    fn test_content_part_image_url() {
        let part = ContentPart::ImageUrl {
            image_url: ImageUrl {
                url: "https://example.com/image.png".to_string(),
                detail: Some("high".to_string()),
            },
        };
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["type"], "image_url");
        assert_eq!(json["image_url"]["url"], "https://example.com/image.png");
        assert_eq!(json["image_url"]["detail"], "high");
    }

    #[test]
    fn test_tool_call_serialization() {
        let tc = ToolCall {
            id: "call_123".to_string(),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: "get_weather".to_string(),
                arguments: r#"{"city":"NYC"}"#.to_string(),
            },
        };
        let json = serde_json::to_value(&tc).unwrap();
        assert_eq!(json["id"], "call_123");
        assert_eq!(json["type"], "function");
        assert_eq!(json["function"]["name"], "get_weather");
    }

    #[test]
    fn test_api_tool_serialization() {
        let tool = ApiTool {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "read_file".to_string(),
                description: "Read a file".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    }
                }),
            },
        };
        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["type"], "function");
        assert_eq!(json["function"]["name"], "read_file");
    }

    #[test]
    fn test_response_format() {
        let format = ResponseFormat {
            format_type: "json_object".to_string(),
        };
        let json = serde_json::to_value(&format).unwrap();
        assert_eq!(json["type"], "json_object");
    }

    #[test]
    fn test_api_response_deserialization() {
        let json = serde_json::json!({
            "id": "chatcmpl-123",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        });

        let response: ApiResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.model, "gpt-4o");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.content, Some("Hello!".to_string()));
    }

    #[test]
    fn test_api_response_with_tool_calls() {
        let json = serde_json::json!({
            "id": "chatcmpl-456",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"city\":\"NYC\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": null
        });

        let response: ApiResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.choices[0].message.tool_calls.len(), 1);
        assert_eq!(response.choices[0].message.tool_calls[0].id, "call_abc");
    }

    #[test]
    fn test_stream_chunk_deserialization() {
        let json = serde_json::json!({
            "id": "chatcmpl-stream",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": "Hello"
                },
                "finish_reason": null
            }],
            "usage": null
        });

        let chunk: StreamChunk = serde_json::from_value(json).unwrap();
        assert_eq!(chunk.id, "chatcmpl-stream");
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_stream_chunk_with_tool_call() {
        let json = serde_json::json!({
            "id": "chatcmpl-stream",
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_xyz",
                        "type": "function",
                        "function": {
                            "name": "search",
                            "arguments": ""
                        }
                    }]
                },
                "finish_reason": null
            }],
            "usage": null
        });

        let chunk: StreamChunk = serde_json::from_value(json).unwrap();
        let tool_calls = chunk.choices[0].delta.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls[0].id, Some("call_xyz".to_string()));
    }

    #[test]
    fn test_stream_chunk_finish() {
        let json = serde_json::json!({
            "id": "chatcmpl-stream",
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 20,
                "completion_tokens": 10,
                "total_tokens": 30
            }
        });

        let chunk: StreamChunk = serde_json::from_value(json).unwrap();
        assert_eq!(chunk.choices[0].finish_reason, Some("stop".to_string()));
        assert!(chunk.usage.is_some());
        assert_eq!(chunk.usage.unwrap().total_tokens, 30);
    }

    #[test]
    fn test_api_message_roundtrip() {
        let message = ApiMessage {
            role: "user".to_string(),
            content: Some(MessageContent::Text("Test message".to_string())),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };

        let json = serde_json::to_value(&message).unwrap();
        let parsed: ApiMessage = serde_json::from_value(json).unwrap();

        assert_eq!(parsed.role, "user");
        match parsed.content {
            Some(MessageContent::Text(t)) => assert_eq!(t, "Test message"),
            _ => panic!("Expected text content"),
        }
    }
}
