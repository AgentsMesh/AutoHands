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
