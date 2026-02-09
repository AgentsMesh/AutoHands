//! Completion response types.

use serde::{Deserialize, Serialize};

use crate::types::{Message, Metadata, StopReason, Usage};

/// Response from a completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Unique ID for this completion.
    pub id: String,

    /// Model used.
    pub model: String,

    /// The assistant's response message.
    pub message: Message,

    /// Reason for stopping.
    pub stop_reason: StopReason,

    /// Token usage.
    pub usage: Usage,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: Metadata,
}

/// A chunk in a streaming completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionChunk {
    /// Chunk type.
    pub chunk_type: ChunkType,

    /// Delta content (for text chunks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,

    /// Tool call information (for tool use chunks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<ToolCallChunk>,

    /// Stop reason (for final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,

    /// Usage information (for final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// Type of streaming chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkType {
    /// Start of a message.
    MessageStart,

    /// Text content delta.
    ContentDelta,

    /// Tool use start.
    ToolUseStart,

    /// Tool use input delta.
    ToolUseDelta,

    /// End of a message.
    MessageEnd,
}

/// Tool call chunk in streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallChunk {
    /// Tool call ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Tool name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Partial input JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_delta: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_response() {
        let response = CompletionResponse {
            id: "test-id".to_string(),
            model: "gpt-4".to_string(),
            message: Message::assistant("Hello"),
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
            metadata: Default::default(),
        };
        assert_eq!(response.id, "test-id");
        assert_eq!(response.model, "gpt-4");
    }

    #[test]
    fn test_completion_response_serialization() {
        let response = CompletionResponse {
            id: "test".to_string(),
            model: "gpt-4".to_string(),
            message: Message::assistant("Hi"),
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
            metadata: Default::default(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("gpt-4"));
    }

    #[test]
    fn test_completion_chunk() {
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ContentDelta,
            delta: Some("Hello".to_string()),
            tool_call: None,
            stop_reason: None,
            usage: None,
        };
        assert_eq!(chunk.delta, Some("Hello".to_string()));
    }

    #[test]
    fn test_completion_chunk_tool_use() {
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: Some("read_file".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };
        assert!(chunk.tool_call.is_some());
    }

    #[test]
    fn test_completion_chunk_message_end() {
        let chunk = CompletionChunk {
            chunk_type: ChunkType::MessageEnd,
            delta: None,
            tool_call: None,
            stop_reason: Some(StopReason::EndTurn),
            usage: Some(Usage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            }),
        };
        assert!(chunk.stop_reason.is_some());
        assert!(chunk.usage.is_some());
    }

    #[test]
    fn test_chunk_type_variants() {
        assert!(matches!(ChunkType::MessageStart, ChunkType::MessageStart));
        assert!(matches!(ChunkType::ContentDelta, ChunkType::ContentDelta));
        assert!(matches!(ChunkType::ToolUseStart, ChunkType::ToolUseStart));
        assert!(matches!(ChunkType::ToolUseDelta, ChunkType::ToolUseDelta));
        assert!(matches!(ChunkType::MessageEnd, ChunkType::MessageEnd));
    }

    #[test]
    fn test_chunk_type_serialization() {
        let json = serde_json::to_string(&ChunkType::ContentDelta).unwrap();
        assert_eq!(json, "\"content_delta\"");

        let json = serde_json::to_string(&ChunkType::MessageStart).unwrap();
        assert_eq!(json, "\"message_start\"");
    }

    #[test]
    fn test_tool_call_chunk() {
        let chunk = ToolCallChunk {
            id: Some("call_1".to_string()),
            name: Some("test_tool".to_string()),
            input_delta: Some("{\"key\":".to_string()),
        };
        assert_eq!(chunk.id, Some("call_1".to_string()));
        assert_eq!(chunk.name, Some("test_tool".to_string()));
    }

    #[test]
    fn test_tool_call_chunk_empty() {
        let chunk = ToolCallChunk {
            id: None,
            name: None,
            input_delta: None,
        };
        assert!(chunk.id.is_none());
    }

    #[test]
    fn test_tool_call_chunk_serialization() {
        let chunk = ToolCallChunk {
            id: Some("call_1".to_string()),
            name: Some("tool".to_string()),
            input_delta: None,
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("call_1"));
        assert!(json.contains("tool"));
    }

    #[test]
    fn test_completion_chunk_clone() {
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ContentDelta,
            delta: Some("test".to_string()),
            tool_call: None,
            stop_reason: None,
            usage: None,
        };
        let cloned = chunk.clone();
        assert_eq!(cloned.delta, chunk.delta);
    }
}
