    use super::*;

    #[test]
    fn test_stream_event_variants() {
        let event = StreamEvent::TurnStart { turn: 1 };
        if let StreamEvent::TurnStart { turn } = event {
            assert_eq!(turn, 1);
        }

        let event = StreamEvent::TextDelta {
            content: "hello".to_string(),
        };
        if let StreamEvent::TextDelta { content } = event {
            assert_eq!(content, "hello");
        }
    }

    #[test]
    fn test_chunk_processor_new() {
        let processor = ChunkProcessor::new();
        assert!(processor.text().is_empty());
    }

    #[test]
    fn test_chunk_processor_default() {
        let processor = ChunkProcessor::default();
        assert!(processor.text().is_empty());
    }

    #[test]
    fn test_chunk_processor_text_delta() {
        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::ContentDelta,
            delta: Some("Hello".to_string()),
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert_eq!(events.len(), 1);

        if let StreamEvent::TextDelta { content } = &events[0] {
            assert_eq!(content, "Hello");
        } else {
            panic!("Expected TextDelta event");
        }

        assert_eq!(processor.text(), "Hello");
    }

    #[test]
    fn test_chunk_processor_accumulates_text() {
        let mut processor = ChunkProcessor::new();

        let chunks = vec![
            CompletionChunk {
                chunk_type: ChunkType::ContentDelta,
                delta: Some("Hello".to_string()),
                tool_call: None,
                stop_reason: None,
                usage: None,
            },
            CompletionChunk {
                chunk_type: ChunkType::ContentDelta,
                delta: Some(" World".to_string()),
                tool_call: None,
                stop_reason: None,
                usage: None,
            },
        ];

        for chunk in chunks {
            processor.process(&chunk);
        }

        assert_eq!(processor.text(), "Hello World");
    }

    #[test]
    fn test_chunk_processor_reset() {
        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::ContentDelta,
            delta: Some("Hello".to_string()),
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        processor.process(&chunk);
        assert_eq!(processor.text(), "Hello");

        processor.reset();
        assert!(processor.text().is_empty());
    }

    #[test]
    fn test_chunk_processor_tool_use() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        let start_chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: Some("test_tool".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&start_chunk);
        assert_eq!(events.len(), 1);

        if let StreamEvent::ToolCallStart { id, name } = &events[0] {
            assert_eq!(id, "call_1");
            assert_eq!(name, "test_tool");
        } else {
            panic!("Expected ToolCallStart event");
        }
    }

    #[test]
    fn test_streaming_agent_loop_creation() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let config = AgentLoopConfig::default();

        let _loop = StreamingAgentLoop::new(provider_registry, tool_registry, config);
    }

    #[test]
    fn test_stream_event_tool_call_start() {
        let event = StreamEvent::ToolCallStart {
            id: "call_1".to_string(),
            name: "test".to_string(),
        };
        if let StreamEvent::ToolCallStart { id, name } = event {
            assert_eq!(id, "call_1");
            assert_eq!(name, "test");
        }
    }

    #[test]
    fn test_stream_event_tool_call_delta() {
        let event = StreamEvent::ToolCallDelta {
            id: "call_1".to_string(),
            input_delta: "{\"key\":".to_string(),
        };
        if let StreamEvent::ToolCallDelta { id, input_delta } = event {
            assert_eq!(id, "call_1");
            assert_eq!(input_delta, "{\"key\":");
        }
    }

    #[test]
    fn test_stream_event_tool_call_complete() {
        let event = StreamEvent::ToolCallComplete {
            id: "call_1".to_string(),
            result: "success".to_string(),
        };
        if let StreamEvent::ToolCallComplete { id, result } = event {
            assert_eq!(id, "call_1");
            assert_eq!(result, "success");
        }
    }

    #[test]
    fn test_stream_event_turn_complete() {
        let event = StreamEvent::TurnComplete { turn: 5 };
        if let StreamEvent::TurnComplete { turn } = event {
            assert_eq!(turn, 5);
        }
    }

    #[test]
    fn test_stream_event_complete() {
        let msg = Message::assistant("Done");
        let event = StreamEvent::Complete { message: msg };
        if let StreamEvent::Complete { message } = event {
            assert_eq!(message.content.text(), "Done");
        }
    }

    #[test]
    fn test_stream_event_error() {
        let event = StreamEvent::Error {
            error: "Something went wrong".to_string(),
        };
        if let StreamEvent::Error { error } = event {
            assert_eq!(error, "Something went wrong");
        }
    }

    #[test]
    fn test_chunk_processor_tool_use_delta() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        // First start a tool use
        let start_chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: Some("test_tool".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };
        processor.process(&start_chunk);

        // Then add delta
        let delta_chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseDelta,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: None,
                name: None,
                input_delta: Some("{\"arg\":".to_string()),
            }),
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&delta_chunk);
        assert_eq!(events.len(), 1);

        if let StreamEvent::ToolCallDelta { id, input_delta } = &events[0] {
            assert_eq!(id, "call_1");
            assert_eq!(input_delta, "{\"arg\":");
        } else {
            panic!("Expected ToolCallDelta event");
        }
    }

    #[test]
    fn test_chunk_processor_other_chunk_types() {
        let mut processor = ChunkProcessor::new();

        // MessageStart should produce no events
        let chunk = CompletionChunk {
            chunk_type: ChunkType::MessageStart,
            delta: None,
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());

        // MessageDelta without delta should produce no events
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ContentDelta,
            delta: None,
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_stream_event_debug() {
        let event = StreamEvent::TurnStart { turn: 1 };
        let debug = format!("{:?}", event);
        assert!(debug.contains("TurnStart"));

        let event = StreamEvent::Error { error: "test".to_string() };
        let debug = format!("{:?}", event);
        assert!(debug.contains("Error"));
    }

    #[test]
    fn test_stream_event_clone() {
        let event = StreamEvent::TextDelta { content: "hello".to_string() };
        let cloned = event.clone();
        if let StreamEvent::TextDelta { content } = cloned {
            assert_eq!(content, "hello");
        }
    }

    #[test]
    fn test_chunk_processor_tool_use_start_without_id() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        // Tool use start without id should not produce event
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: None,
                name: Some("test".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_tool_use_start_without_name() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: None,
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_tool_use_delta_without_prior_start() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        // Delta without prior start should produce no event (no current_tool_id)
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseDelta,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: None,
                name: None,
                input_delta: Some("{\"key\":".to_string()),
            }),
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_tool_use_delta_without_input() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        // First start a tool
        let start_chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: Some("test".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };
        processor.process(&start_chunk);

        // Delta without input_delta
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseDelta,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: None,
                name: None,
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_tool_use_start_without_tool_call() {
        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_tool_use_delta_without_tool_call() {
        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseDelta,
            delta: None,
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_reset_clears_tool_state() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        // Start a tool
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: Some("test".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };
        processor.process(&chunk);

        // Add some input
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseDelta,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: None,
                name: None,
                input_delta: Some("{\"key\":".to_string()),
            }),
            stop_reason: None,
            usage: None,
        };
        processor.process(&chunk);

        // Reset and verify tool state is cleared
        processor.reset();

        // Now delta should produce no events since there's no active tool
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseDelta,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: None,
                name: None,
                input_delta: Some("value}".to_string()),
            }),
            stop_reason: None,
            usage: None,
        };
        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_message_end() {
        use autohands_protocols::types::StopReason;

        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::MessageEnd,
            delta: None,
            tool_call: None,
            stop_reason: Some(StopReason::EndTurn),
            usage: None,
        };

        let events = processor.process(&chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_chunk_processor_message_start() {
        let mut processor = ChunkProcessor::new();

        let chunk = CompletionChunk {
            chunk_type: ChunkType::MessageStart,
            delta: Some("ignored".to_string()),
            tool_call: None,
            stop_reason: None,
            usage: None,
        };

        let events = processor.process(&chunk);
        // MessageStart doesn't produce TextDelta events
        assert!(events.is_empty());
    }

    #[test]
    fn test_stream_event_all_variants_clone() {
        let events = vec![
            StreamEvent::TurnStart { turn: 1 },
            StreamEvent::TextDelta { content: "hi".to_string() },
            StreamEvent::ToolCallStart { id: "1".to_string(), name: "t".to_string() },
            StreamEvent::ToolCallDelta { id: "1".to_string(), input_delta: "x".to_string() },
            StreamEvent::ToolCallComplete { id: "1".to_string(), result: "r".to_string() },
            StreamEvent::TurnComplete { turn: 1 },
            StreamEvent::Complete { message: Message::assistant("done") },
            StreamEvent::Error { error: "err".to_string() },
        ];

        for event in events {
            let _cloned = event.clone();
        }
    }

    #[test]
    fn test_streaming_agent_loop_with_custom_config() {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let config = AgentLoopConfig {
            max_turns: 10,
            timeout_seconds: 60,
            checkpoint_enabled: false,
            ..Default::default()
        };

        let _loop = StreamingAgentLoop::new(provider_registry, tool_registry, config);
    }

    #[test]
    fn test_chunk_processor_multiple_tool_uses() {
        use autohands_protocols::provider::ToolCallChunk;

        let mut processor = ChunkProcessor::new();

        // First tool
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_1".to_string()),
                name: Some("tool1".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };
        let events = processor.process(&chunk);
        assert_eq!(events.len(), 1);

        // Second tool (overwrites first)
        let chunk = CompletionChunk {
            chunk_type: ChunkType::ToolUseStart,
            delta: None,
            tool_call: Some(ToolCallChunk {
                id: Some("call_2".to_string()),
                name: Some("tool2".to_string()),
                input_delta: None,
            }),
            stop_reason: None,
            usage: None,
        };
        let events = processor.process(&chunk);
        assert_eq!(events.len(), 1);

        if let StreamEvent::ToolCallStart { id, name } = &events[0] {
            assert_eq!(id, "call_2");
            assert_eq!(name, "tool2");
        }
    }
