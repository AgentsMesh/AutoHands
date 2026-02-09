//! Ark API types (OpenAI-compatible format).

use serde::{Deserialize, Serialize};

/// Ark API request (OpenAI-compatible).
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
    /// Reasoning content for Seed models (推理过程)
    pub reasoning_content: Option<String>,
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
            model: "doubao-pro-32k".to_string(),
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
        assert_eq!(json["model"], "doubao-pro-32k");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["stream"], true);
    }

    #[test]
    fn test_api_response_deserialization() {
        let json = serde_json::json!({
            "id": "chatcmpl-123",
            "model": "doubao-pro-32k",
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
        assert_eq!(response.model, "doubao-pro-32k");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.content, Some("Hello!".to_string()));
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
}
