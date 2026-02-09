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
mod tests {
    use super::*;
    use crate::api::{ApiUsage, StreamMessage};

    #[test]
    fn test_parse_stop_reason() {
        assert_eq!(parse_stop_reason("end_turn"), StopReason::EndTurn);
        assert_eq!(parse_stop_reason("max_tokens"), StopReason::MaxTokens);
        assert_eq!(parse_stop_reason("tool_use"), StopReason::ToolUse);
        assert_eq!(parse_stop_reason("unknown"), StopReason::EndTurn);
    }

    #[test]
    fn test_parse_stop_reason_stop_sequence() {
        assert_eq!(parse_stop_reason("stop_sequence"), StopReason::StopSequence);
    }

    #[test]
    fn test_parse_response_text_only() {
        let response = ApiResponse {
            id: "msg_123".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            content: vec![ContentBlock::Text {
                text: "Hello, world!".to_string(),
            }],
            stop_reason: "end_turn".to_string(),
            usage: ApiUsage {
                input_tokens: 10,
                output_tokens: 5,
            },
        };

        let parsed = parse_response(response);
        assert_eq!(parsed.id, "msg_123");
        assert_eq!(parsed.model, "claude-sonnet-4-20250514");
        assert_eq!(parsed.message.content.text(), "Hello, world!");
        assert!(parsed.message.tool_calls.is_empty());
        assert_eq!(parsed.stop_reason, StopReason::EndTurn);
        assert_eq!(parsed.usage.prompt_tokens, 10);
        assert_eq!(parsed.usage.completion_tokens, 5);
        assert_eq!(parsed.usage.total_tokens, 15);
    }

    #[test]
    fn test_parse_response_with_tool_use() {
        let response = ApiResponse {
            id: "msg_456".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            content: vec![
                ContentBlock::Text {
                    text: "Let me search for that.".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "toolu_abc".to_string(),
                    name: "search".to_string(),
                    input: serde_json::json!({"query": "rust programming"}),
                },
            ],
            stop_reason: "tool_use".to_string(),
            usage: ApiUsage {
                input_tokens: 100,
                output_tokens: 50,
            },
        };

        let parsed = parse_response(response);
        assert_eq!(parsed.message.content.text(), "Let me search for that.");
        assert_eq!(parsed.message.tool_calls.len(), 1);
        assert_eq!(parsed.message.tool_calls[0].id, "toolu_abc");
        assert_eq!(parsed.message.tool_calls[0].name, "search");
        assert_eq!(parsed.message.tool_calls[0].arguments["query"], "rust programming");
        assert_eq!(parsed.stop_reason, StopReason::ToolUse);
    }

    #[test]
    fn test_parse_response_multiple_tool_calls() {
        let response = ApiResponse {
            id: "msg_789".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            content: vec![
                ContentBlock::ToolUse {
                    id: "toolu_1".to_string(),
                    name: "read_file".to_string(),
                    input: serde_json::json!({"path": "/tmp/a.txt"}),
                },
                ContentBlock::ToolUse {
                    id: "toolu_2".to_string(),
                    name: "read_file".to_string(),
                    input: serde_json::json!({"path": "/tmp/b.txt"}),
                },
            ],
            stop_reason: "tool_use".to_string(),
            usage: ApiUsage {
                input_tokens: 50,
                output_tokens: 30,
            },
        };

        let parsed = parse_response(response);
        assert_eq!(parsed.message.tool_calls.len(), 2);
        assert_eq!(parsed.message.tool_calls[0].name, "read_file");
        assert_eq!(parsed.message.tool_calls[1].name, "read_file");
    }

    #[test]
    fn test_parse_response_with_tool_result_ignored() {
        // ToolResult blocks in response are not typical but should be handled
        let response = ApiResponse {
            id: "msg_test".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            content: vec![ContentBlock::ToolResult {
                tool_use_id: "toolu_xyz".to_string(),
                content: "result data".to_string(),
            }],
            stop_reason: "end_turn".to_string(),
            usage: ApiUsage {
                input_tokens: 5,
                output_tokens: 5,
            },
        };

        let parsed = parse_response(response);
        // ToolResult is ignored in response parsing
        assert_eq!(parsed.message.content.text(), "");
        assert!(parsed.message.tool_calls.is_empty());
    }

    #[test]
    fn test_parse_stream_event_message_start() {
        let event = StreamEvent::MessageStart {
            message: StreamMessage {
                id: "msg_stream".to_string(),
                model: "claude-sonnet-4-20250514".to_string(),
            },
        };

        let chunk = parse_stream_event(event);
        assert_eq!(chunk.chunk_type, ChunkType::MessageStart);
        assert!(chunk.delta.is_none());
        assert!(chunk.tool_call.is_none());
    }

    #[test]
    fn test_parse_stream_event_text_delta() {
        let event = StreamEvent::ContentBlockDelta {
            index: 0,
            delta: StreamDelta::TextDelta {
                text: "Hello".to_string(),
            },
        };

        let chunk = parse_stream_event(event);
        assert_eq!(chunk.chunk_type, ChunkType::ContentDelta);
        assert_eq!(chunk.delta, Some("Hello".to_string()));
        assert!(chunk.tool_call.is_none());
    }

    #[test]
    fn test_parse_stream_event_tool_delta() {
        let event = StreamEvent::ContentBlockDelta {
            index: 0,
            delta: StreamDelta::InputJsonDelta {
                partial_json: r#"{"query":"#.to_string(),
            },
        };

        let chunk = parse_stream_event(event);
        assert_eq!(chunk.chunk_type, ChunkType::ToolUseDelta);
        assert!(chunk.delta.is_none());
        assert!(chunk.tool_call.is_some());
        let tool_call = chunk.tool_call.unwrap();
        assert_eq!(tool_call.input_delta, Some(r#"{"query":"#.to_string()));
    }

    #[test]
    fn test_parse_stream_event_message_stop() {
        let event = StreamEvent::MessageStop;

        let chunk = parse_stream_event(event);
        assert_eq!(chunk.chunk_type, ChunkType::MessageEnd);
        assert_eq!(chunk.stop_reason, Some(StopReason::EndTurn));
    }

    #[test]
    fn test_parse_stream_event_ping() {
        let event = StreamEvent::Ping;

        let chunk = parse_stream_event(event);
        assert_eq!(chunk.chunk_type, ChunkType::ContentDelta);
        assert!(chunk.delta.is_none());
    }

    #[test]
    fn test_parse_stream_event_content_block_start() {
        let event = StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlock::Text {
                text: "".to_string(),
            },
        };

        let chunk = parse_stream_event(event);
        // Falls through to default handler
        assert_eq!(chunk.chunk_type, ChunkType::ContentDelta);
    }

    #[test]
    fn test_parse_stream_event_content_block_stop() {
        let event = StreamEvent::ContentBlockStop { index: 0 };

        let chunk = parse_stream_event(event);
        // Falls through to default handler
        assert_eq!(chunk.chunk_type, ChunkType::ContentDelta);
    }

    #[test]
    fn test_parse_response_max_tokens_stop() {
        let response = ApiResponse {
            id: "msg_max".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            content: vec![ContentBlock::Text {
                text: "This is a truncated response...".to_string(),
            }],
            stop_reason: "max_tokens".to_string(),
            usage: ApiUsage {
                input_tokens: 1000,
                output_tokens: 4096,
            },
        };

        let parsed = parse_response(response);
        assert_eq!(parsed.stop_reason, StopReason::MaxTokens);
    }
}
