//! Response parsing utilities.

use autohands_protocols::provider::{ChunkType, CompletionChunk, CompletionResponse};
use autohands_protocols::types::{Message, MessageContent, MessageRole, StopReason, ToolCall, Usage};

use crate::api::{ApiResponse, ContentBlock, StreamDelta, StreamEvent};

/// Parse API response to CompletionResponse.
pub fn parse_response(response: ApiResponse) -> CompletionResponse {
    let mut text = String::new();
    let mut tool_calls = Vec::new();

    for block in &response.content {
        match block {
            ContentBlock::Text { text: t } => text.push_str(t),
            ContentBlock::ToolUse { id, name, input } => {
                tool_calls.push(ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: input.clone(),
                });
            }
            _ => {}
        }
    }

    let stop_reason = parse_stop_reason(&response.stop_reason);

    CompletionResponse {
        id: response.id,
        model: response.model,
        message: Message {
            role: MessageRole::Assistant,
            content: MessageContent::Text(text),
            name: None,
            tool_calls,
            tool_call_id: None,
            metadata: Default::default(),
        },
        stop_reason,
        usage: Usage {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            cache_creation_tokens: None,
            cache_read_tokens: None,
        },
        metadata: Default::default(),
    }
}

/// Parse stop reason string.
pub fn parse_stop_reason(reason: &str) -> StopReason {
    match reason {
        "end_turn" => StopReason::EndTurn,
        "max_tokens" => StopReason::MaxTokens,
        "tool_use" => StopReason::ToolUse,
        "stop_sequence" => StopReason::StopSequence,
        _ => StopReason::EndTurn,
    }
}

/// Parse streaming event to CompletionChunk.
pub fn parse_stream_event(event: StreamEvent) -> CompletionChunk {
    match event {
        StreamEvent::MessageStart { .. } => CompletionChunk {
            chunk_type: ChunkType::MessageStart,
            delta: None,
            tool_call: None,
            stop_reason: None,
            usage: None,
        },
        StreamEvent::ContentBlockDelta { delta, .. } => match delta {
            StreamDelta::TextDelta { text } => CompletionChunk {
                chunk_type: ChunkType::ContentDelta,
                delta: Some(text),
                tool_call: None,
                stop_reason: None,
                usage: None,
            },
            StreamDelta::InputJsonDelta { partial_json } => CompletionChunk {
                chunk_type: ChunkType::ToolUseDelta,
                delta: None,
                tool_call: Some(autohands_protocols::provider::ToolCallChunk {
                    id: None,
                    name: None,
                    input_delta: Some(partial_json),
                }),
                stop_reason: None,
                usage: None,
            },
        },
        StreamEvent::MessageStop => CompletionChunk {
            chunk_type: ChunkType::MessageEnd,
            delta: None,
            tool_call: None,
            stop_reason: Some(StopReason::EndTurn),
            usage: None,
        },
        _ => CompletionChunk {
            chunk_type: ChunkType::ContentDelta,
            delta: None,
            tool_call: None,
            stop_reason: None,
            usage: None,
        },
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
