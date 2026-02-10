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
