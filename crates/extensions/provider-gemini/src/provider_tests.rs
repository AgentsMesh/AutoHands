    use super::*;
    use autohands_protocols::types::ToolCall;

    #[test]
    fn test_provider_creation() {
        let provider = GeminiProvider::new("test-key".to_string());
        assert_eq!(provider.id(), "gemini");
        assert!(!provider.models().is_empty());
    }

    #[test]
    fn test_provider_capabilities() {
        let provider = GeminiProvider::new("test-key".to_string());
        let caps = provider.capabilities();
        assert!(caps.streaming);
        assert!(caps.tool_calling);
        assert!(caps.vision);
    }

    #[test]
    fn test_convert_messages() {
        let provider = GeminiProvider::new("test-key".to_string());
        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi there!"),
        ];

        let contents = provider.convert_messages(&messages);
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0].role, "user");
        assert_eq!(contents[1].role, "model");
    }

    #[test]
    fn test_convert_system() {
        let provider = GeminiProvider::new("test-key".to_string());
        let messages = vec![
            Message::system("You are helpful"),
            Message::user("Hello"),
        ];

        let system = provider.convert_system(&messages);
        assert!(system.is_some());
    }

    #[test]
    fn test_convert_messages_filters_system() {
        let provider = GeminiProvider::new("test-key".to_string());
        let messages = vec![
            Message::system("System prompt"),
            Message::user("Hello"),
            Message::assistant("Hi"),
        ];

        let contents = provider.convert_messages(&messages);
        // System message should be filtered out
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0].role, "user");
        assert_eq!(contents[1].role, "model");
    }

    #[test]
    fn test_convert_system_not_found() {
        let provider = GeminiProvider::new("test-key".to_string());
        let messages = vec![
            Message::user("Hello"),
        ];

        let system = provider.convert_system(&messages);
        assert!(system.is_none());
    }

    #[test]
    fn test_convert_tools_empty() {
        let provider = GeminiProvider::new("test-key".to_string());
        let request = CompletionRequest::new("gemini-1.5-flash".to_string(), vec![]);

        let tools = provider.convert_tools(&request);
        assert!(tools.is_none());
    }

    #[test]
    fn test_provider_models_count() {
        let provider = GeminiProvider::new("test-key".to_string());
        let models = provider.models();
        assert_eq!(models.len(), 3);
    }

    #[test]
    fn test_provider_capabilities_detail() {
        let provider = GeminiProvider::new("test-key".to_string());
        let caps = provider.capabilities();
        assert!(caps.json_mode);
        assert!(!caps.prompt_caching);
        assert!(!caps.batching);
        assert_eq!(caps.max_concurrent, Some(10));
    }

    #[test]
    fn test_convert_messages_tool_response() {
        let provider = GeminiProvider::new("test-key".to_string());
        let messages = vec![
            Message::tool("tool_call_1", "Tool result"),
        ];

        let contents = provider.convert_messages(&messages);
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, "user");
    }

    #[test]
    fn test_convert_messages_with_tool_calls() {
        let provider = GeminiProvider::new("test-key".to_string());
        let mut assistant_msg = Message::assistant("");
        assistant_msg.tool_calls = vec![ToolCall {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            arguments: serde_json::json!({"city": "NYC"}),
        }];

        let messages = vec![assistant_msg];
        let contents = provider.convert_messages(&messages);
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, "model");
    }

    #[test]
    fn test_models_have_vision() {
        let provider = GeminiProvider::new("test-key".to_string());
        let models = provider.models();
        // gemini-1.5-pro and gemini-1.5-flash should have vision
        let vision_models = models.iter().filter(|m| m.supports_vision).count();
        assert_eq!(vision_models, 2);
    }

    #[test]
    fn test_provider_id_is_gemini() {
        let provider = GeminiProvider::new("key".to_string());
        assert_eq!(provider.id(), "gemini");
    }

    #[test]
    fn test_convert_tools_with_tools() {
        use autohands_protocols::tool::ToolDefinition;

        let provider = GeminiProvider::new("test-key".to_string());
        let tools = vec![
            ToolDefinition::new("read_file", "Read File", "Read a file from disk")
                .with_parameters_schema(serde_json::json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}}
                })),
        ];
        let request = CompletionRequest::new("gemini-1.5-flash".to_string(), vec![])
            .with_tools(tools);

        let converted = provider.convert_tools(&request);
        assert!(converted.is_some());
        let gemini_tools = converted.unwrap();
        assert_eq!(gemini_tools.len(), 1);
        assert_eq!(gemini_tools[0].function_declarations.len(), 1);
        assert_eq!(gemini_tools[0].function_declarations[0].name, "read_file");
    }

    #[test]
    fn test_convert_response_basic() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "Hello!".to_string() }],
                },
                finish_reason: Some("STOP".to_string()),
                safety_ratings: vec![],
            }],
            usage_metadata: Some(UsageMetadata {
                prompt_token_count: 10,
                candidates_token_count: 5,
                total_token_count: 15,
            }),
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        assert!(result.message.content.text().contains("Hello!"));
        assert_eq!(result.stop_reason, StopReason::EndTurn);
        assert_eq!(result.usage.prompt_tokens, 10);
        assert_eq!(result.usage.completion_tokens, 5);
    }

    #[test]
    fn test_convert_response_max_tokens() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "Truncated...".to_string() }],
                },
                finish_reason: Some("MAX_TOKENS".to_string()),
                safety_ratings: vec![],
            }],
            usage_metadata: None,
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        assert_eq!(result.stop_reason, StopReason::MaxTokens);
    }

    #[test]
    fn test_convert_response_stop_sequence() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "text".to_string() }],
                },
                finish_reason: Some("STOP_SEQUENCE".to_string()),
                safety_ratings: vec![],
            }],
            usage_metadata: None,
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        assert_eq!(result.stop_reason, StopReason::StopSequence);
    }

    #[test]
    fn test_convert_response_with_function_call() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::FunctionCall {
                        function_call: FunctionCall {
                            name: "get_weather".to_string(),
                            args: serde_json::json!({"city": "NYC"}),
                        },
                    }],
                },
                finish_reason: Some("STOP".to_string()),
                safety_ratings: vec![],
            }],
            usage_metadata: None,
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        assert_eq!(result.message.tool_calls.len(), 1);
        assert_eq!(result.message.tool_calls[0].name, "get_weather");
    }

    #[test]
    fn test_convert_response_unknown_finish_reason() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "text".to_string() }],
                },
                finish_reason: Some("UNKNOWN_REASON".to_string()),
                safety_ratings: vec![],
            }],
            usage_metadata: None,
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        // Unknown reason defaults to EndTurn
        assert_eq!(result.stop_reason, StopReason::EndTurn);
    }

    #[test]
    fn test_convert_response_no_finish_reason() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![Part::Text { text: "text".to_string() }],
                },
                finish_reason: None,
                safety_ratings: vec![],
            }],
            usage_metadata: None,
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        assert_eq!(result.stop_reason, StopReason::EndTurn);
    }

    #[test]
    fn test_convert_response_mixed_parts() {
        let provider = GeminiProvider::new("test-key".to_string());
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: "model".to_string(),
                    parts: vec![
                        Part::Text { text: "Here's the weather: ".to_string() },
                        Part::FunctionCall {
                            function_call: FunctionCall {
                                name: "get_weather".to_string(),
                                args: serde_json::json!({}),
                            },
                        },
                    ],
                },
                finish_reason: Some("STOP".to_string()),
                safety_ratings: vec![],
            }],
            usage_metadata: None,
        };

        let result = provider.convert_response(response, "gemini-1.5-flash");
        assert!(result.message.content.text().contains("Here's the weather"));
        assert_eq!(result.message.tool_calls.len(), 1);
    }

    #[test]
    fn test_model_context_lengths() {
        let provider = GeminiProvider::new("test-key".to_string());
        let models = provider.models();

        // Verify context lengths are set correctly
        for model in models {
            if model.id == "gemini-1.5-pro" {
                assert_eq!(model.context_length, 2_000_000);
            } else if model.id == "gemini-2.0-flash" || model.id == "gemini-1.5-flash" {
                assert_eq!(model.context_length, 1_000_000);
            }
        }
    }
