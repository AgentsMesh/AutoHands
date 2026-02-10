    use super::*;
    use autohands_protocols::types::Message;

    #[test]
    fn test_provider_id() {
        let provider = ArkProvider::new("test-key".to_string());
        assert_eq!(provider.id(), "ark");
    }

    #[test]
    fn test_provider_capabilities() {
        let provider = ArkProvider::new("test-key".to_string());
        let caps = provider.capabilities();
        assert!(caps.streaming);
        assert!(caps.tool_calling);
        assert!(caps.vision);
        assert!(caps.json_mode);
    }

    #[test]
    fn test_models_not_empty() {
        let provider = ArkProvider::new("test-key".to_string());
        assert!(!provider.models().is_empty());
    }

    #[test]
    fn test_custom_url() {
        let provider = ArkProvider::with_url(
            "test-key".to_string(),
            "https://custom.ark.api/v1".to_string(),
        );
        assert_eq!(provider.api_url, "https://custom.ark.api/v1");
    }

    #[test]
    fn test_default_api_url() {
        let provider = ArkProvider::new("test-key".to_string());
        assert_eq!(provider.api_url, DEFAULT_API_URL);
    }

    #[test]
    fn test_build_request_basic() {
        let provider = ArkProvider::new("key".to_string());
        let request = CompletionRequest::new(
            "doubao-pro-32k".to_string(),
            vec![Message::user("你好")],
        );
        let api_request = provider.build_request(&request, false);
        assert_eq!(api_request.model, "doubao-pro-32k");
        assert_eq!(api_request.stream, Some(false));
    }

    #[test]
    fn test_build_request_with_stream() {
        let provider = ArkProvider::new("key".to_string());
        let request = CompletionRequest::new(
            "doubao-pro-32k".to_string(),
            vec![Message::user("你好")],
        );
        let api_request = provider.build_request(&request, true);
        assert_eq!(api_request.stream, Some(true));
    }

    #[test]
    fn test_with_custom_model() {
        let custom_model = ModelDefinition {
            id: "custom-endpoint-id".to_string(),
            name: "Custom Model".to_string(),
            description: Some("A custom deployed model".to_string()),
            context_length: 32768,
            max_output_tokens: 4096,
            supports_vision: false,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: None,
            output_cost_per_million: None,
            metadata: Default::default(),
        };

        let provider = ArkProvider::new("key".to_string()).with_custom_model(custom_model);
        let models = provider.models();
        let has_custom = models.iter().any(|m| m.id == "custom-endpoint-id");
        assert!(has_custom);
    }

    #[test]
    fn test_provider_models_contain_doubao() {
        let provider = ArkProvider::new("key".to_string());
        let models = provider.models();
        let has_doubao = models.iter().any(|m| m.id.contains("doubao"));
        assert!(has_doubao);
    }

    // Wiremock-based tests for actual HTTP calls
    mod http_tests {
        use super::*;
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        #[tokio::test]
        async fn test_complete_success() {
            let mock_server = MockServer::start().await;

            let response_body = serde_json::json!({
                "id": "chatcmpl-123",
                "model": "doubao-pro-32k",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "你好！有什么可以帮助你的吗？"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 15,
                    "total_tokens": 25
                }
            })
            .to_string();

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(200).set_body_string(&response_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = ArkProvider::with_url("test-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "doubao-pro-32k".to_string(),
                vec![Message::user("你好")],
            );

            let result = provider.complete(request).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            assert!(response.message.content.text().contains("你好"));
        }

        #[tokio::test]
        async fn test_complete_api_error() {
            let mock_server = MockServer::start().await;

            let error_body = r#"{"error": {"message": "Invalid API key", "type": "invalid_request_error"}}"#;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(401).set_body_string(error_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = ArkProvider::with_url("bad-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "doubao-pro-32k".to_string(),
                vec![Message::user("你好")],
            );

            let result = provider.complete(request).await;
            assert!(result.is_err());
            match result.unwrap_err() {
                ProviderError::AuthenticationFailed(message) => {
                    assert!(message.contains("Invalid API key"));
                }
                _ => panic!("Expected AuthenticationFailed"),
            }
        }

        #[tokio::test]
        async fn test_complete_with_tool_use() {
            let mock_server = MockServer::start().await;

            let response_body = serde_json::json!({
                "id": "chatcmpl-tool",
                "model": "doubao-pro-32k",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": "call_123",
                            "type": "function",
                            "function": {
                                "name": "get_weather",
                                "arguments": "{\"city\": \"北京\"}"
                            }
                        }]
                    },
                    "finish_reason": "tool_calls"
                }],
                "usage": {
                    "prompt_tokens": 20,
                    "completion_tokens": 15,
                    "total_tokens": 35
                }
            })
            .to_string();

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(200).set_body_string(&response_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = ArkProvider::with_url("test-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "doubao-pro-32k".to_string(),
                vec![Message::user("北京今天天气怎么样？")],
            );

            let result = provider.complete(request).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            assert!(!response.message.tool_calls.is_empty());
            assert_eq!(response.message.tool_calls[0].name, "get_weather");
        }
    }
