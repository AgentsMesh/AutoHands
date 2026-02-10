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
#[path = "response_tests.rs"]
mod tests;
