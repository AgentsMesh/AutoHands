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
