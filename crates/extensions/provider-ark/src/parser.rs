//! Response parsing for Ark API.

use autohands_protocols::provider::{
    ChunkType, CompletionChunk, CompletionResponse, ToolCallChunk,
};
use autohands_protocols::types::{Message, MessageContent, MessageRole, StopReason, ToolCall, Usage};

use crate::api::{ApiResponse, StreamChunk, StreamDelta};

/// Parse non-streaming response to protocol format.
pub fn parse_response(response: ApiResponse) -> CompletionResponse {
    let choice = response.choices.first();

    let content = choice
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();

    let tool_calls: Vec<ToolCall> = choice
        .map(|c| {
            c.message
                .tool_calls
                .iter()
                .map(|tc| ToolCall {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    arguments: serde_json::from_str(&tc.function.arguments)
                        .unwrap_or(serde_json::Value::Null),
                })
                .collect()
        })
        .unwrap_or_default();

    let stop_reason = choice
        .and_then(|c| c.finish_reason.as_ref())
        .map(|r| parse_stop_reason(r))
        .unwrap_or(StopReason::EndTurn);

    let usage = response
        .usage
        .map(|u| Usage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
            cache_creation_tokens: None,
            cache_read_tokens: None,
        })
        .unwrap_or_default();

    // Build the response message
    let message = Message {
        role: MessageRole::Assistant,
        content: MessageContent::Text(content),
        name: None,
        tool_calls,
        tool_call_id: None,
        metadata: Default::default(),
    };

    CompletionResponse {
        id: response.id,
        model: response.model,
        message,
        stop_reason,
        usage,
        metadata: Default::default(),
    }
}

fn parse_stop_reason(reason: &str) -> StopReason {
    match reason {
        "stop" => StopReason::EndTurn,
        "length" => StopReason::MaxTokens,
        "tool_calls" => StopReason::ToolUse,
        _ => StopReason::EndTurn,
    }
}

/// Parse streaming chunk to protocol format.
pub fn parse_stream_chunk(chunk: StreamChunk) -> CompletionChunk {
    let choice = chunk.choices.first();

    if let Some(choice) = choice {
        if choice.finish_reason.is_some() {
            return CompletionChunk {
                chunk_type: ChunkType::MessageEnd,
                delta: None,
                tool_call: None,
                stop_reason: choice.finish_reason.as_ref().map(|r| parse_stop_reason(r)),
                usage: chunk.usage.map(|u| Usage {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                    cache_creation_tokens: None,
                    cache_read_tokens: None,
                }),
            };
        }

        return parse_delta(&choice.delta);
    }

    CompletionChunk {
        chunk_type: ChunkType::ContentDelta,
        delta: None,
        tool_call: None,
        stop_reason: None,
        usage: None,
    }
}

fn parse_delta(delta: &StreamDelta) -> CompletionChunk {
    // Handle text content - output both reasoning and final content
    // For Seed models: reasoning_content is thinking process, content is final answer

    // First check for reasoning content (thinking process)
    if let Some(reasoning) = &delta.reasoning_content {
        if !reasoning.is_empty() {
            return CompletionChunk {
                chunk_type: ChunkType::ContentDelta,
                delta: Some(format!("<think>{}</think>", reasoning)),
                tool_call: None,
                stop_reason: None,
                usage: None,
            };
        }
    }

    // Then check for final content
    if let Some(content) = &delta.content {
        if !content.is_empty() {
            return CompletionChunk {
                chunk_type: ChunkType::ContentDelta,
                delta: Some(content.clone()),
                tool_call: None,
                stop_reason: None,
                usage: None,
            };
        }
    }

    // Handle tool calls
    if let Some(tool_calls) = &delta.tool_calls {
        if let Some(tc) = tool_calls.first() {
            if tc.id.is_some() {
                // New tool call started
                return CompletionChunk {
                    chunk_type: ChunkType::ToolUseStart,
                    delta: None,
                    tool_call: Some(ToolCallChunk {
                        id: tc.id.clone(),
                        name: tc.function.as_ref().and_then(|f| f.name.clone()),
                        input_delta: None,
                    }),
                    stop_reason: None,
                    usage: None,
                };
            } else if let Some(func) = &tc.function {
                // Tool call argument delta
                if let Some(args) = &func.arguments {
                    return CompletionChunk {
                        chunk_type: ChunkType::ToolUseDelta,
                        delta: None,
                        tool_call: Some(ToolCallChunk {
                            id: None,
                            name: None,
                            input_delta: Some(args.clone()),
                        }),
                        stop_reason: None,
                        usage: None,
                    };
                }
            }
        }
    }

    CompletionChunk {
        chunk_type: ChunkType::ContentDelta,
        delta: None,
        tool_call: None,
        stop_reason: None,
        usage: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ApiUsage, Choice, ResponseMessage, StreamChoice, StreamFunctionCall, StreamToolCall};

    #[test]
    fn test_parse_stop_reason() {
        assert!(matches!(parse_stop_reason("stop"), StopReason::EndTurn));
        assert!(matches!(parse_stop_reason("length"), StopReason::MaxTokens));
        assert!(matches!(parse_stop_reason("tool_calls"), StopReason::ToolUse));
        assert!(matches!(parse_stop_reason("unknown"), StopReason::EndTurn));
    }

    #[test]
    fn test_parse_response() {
        let response = ApiResponse {
            id: "test-id".to_string(),
            model: "doubao-pro-32k".to_string(),
            choices: vec![Choice {
                index: 0,
                message: ResponseMessage {
                    role: "assistant".to_string(),
                    content: Some("你好！".to_string()),
                    tool_calls: vec![],
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(ApiUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };
        let result = parse_response(response);
        assert_eq!(result.message.content.text(), "你好！");
        assert!(matches!(result.stop_reason, StopReason::EndTurn));
        assert_eq!(result.usage.prompt_tokens, 10);
    }

    #[test]
    fn test_parse_response_with_tool_calls() {
        let response = ApiResponse {
            id: "test-id".to_string(),
            model: "doubao-pro-32k".to_string(),
            choices: vec![Choice {
                index: 0,
                message: ResponseMessage {
                    role: "assistant".to_string(),
                    content: None,
                    tool_calls: vec![crate::api::ToolCall {
                        id: "call_123".to_string(),
                        call_type: "function".to_string(),
                        function: crate::api::FunctionCall {
                            name: "get_weather".to_string(),
                            arguments: r#"{"city":"北京"}"#.to_string(),
                        },
                    }],
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: None,
        };
        let result = parse_response(response);
        assert_eq!(result.message.tool_calls.len(), 1);
        assert_eq!(result.message.tool_calls[0].name, "get_weather");
    }

    #[test]
    fn test_parse_stream_chunk_content() {
        let chunk = StreamChunk {
            id: "test".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: Some("你好".to_string()),
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: None,
            }],
            usage: None,
        };
        let result = parse_stream_chunk(chunk);
        assert!(matches!(result.chunk_type, ChunkType::ContentDelta));
        assert_eq!(result.delta, Some("你好".to_string()));
    }

    #[test]
    fn test_parse_stream_chunk_end() {
        let chunk = StreamChunk {
            id: "test".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: None,
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(ApiUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };
        let result = parse_stream_chunk(chunk);
        assert!(matches!(result.chunk_type, ChunkType::MessageEnd));
        assert!(result.usage.is_some());
    }

    #[test]
    fn test_parse_stream_chunk_tool_start() {
        let chunk = StreamChunk {
            id: "test".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: None,
                    reasoning_content: None,
                    tool_calls: Some(vec![StreamToolCall {
                        index: 0,
                        id: Some("call_123".to_string()),
                        call_type: Some("function".to_string()),
                        function: Some(StreamFunctionCall {
                            name: Some("get_weather".to_string()),
                            arguments: None,
                        }),
                    }]),
                },
                finish_reason: None,
            }],
            usage: None,
        };
        let result = parse_stream_chunk(chunk);
        assert!(matches!(result.chunk_type, ChunkType::ToolUseStart));
        assert!(result.tool_call.is_some());
    }
}
