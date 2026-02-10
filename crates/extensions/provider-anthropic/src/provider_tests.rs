    use super::*;
    use autohands_protocols::types::Message;

    #[test]
    fn test_provider_id() {
        let provider = AnthropicProvider::new("test-key".to_string());
        assert_eq!(provider.id(), "anthropic");
    }

    #[test]
    fn test_provider_capabilities() {
        let provider = AnthropicProvider::new("test-key".to_string());
        let caps = provider.capabilities();
        assert!(caps.streaming);
        assert!(caps.tool_calling);
        assert!(caps.vision);
        assert!(!caps.json_mode);
        assert!(caps.prompt_caching);
        assert!(caps.batching);
        assert_eq!(caps.max_concurrent, Some(50));
    }

    #[test]
    fn test_models_not_empty() {
        let provider = AnthropicProvider::new("test-key".to_string());
        assert!(!provider.models().is_empty());
    }

    #[test]
    fn test_models_contain_claude() {
        let provider = AnthropicProvider::new("test-key".to_string());
        let models = provider.models();
        assert!(models.iter().any(|m| m.id.contains("claude")));
    }

    #[test]
    fn test_build_request_basic() {
        let provider = AnthropicProvider::new("test-key".to_string());
        let mut request = CompletionRequest::new(
            "claude-3-5-sonnet-20241022".to_string(),
            vec![Message::user("Hello")],
        );
        request.system = Some("You are a helpful assistant.".to_string());
        request.max_tokens = Some(1024);
        request.temperature = Some(0.7);

        let api_request = provider.build_request(&request, false);
        assert_eq!(api_request.model, "claude-3-5-sonnet-20241022");
        assert_eq!(api_request.max_tokens, 1024);
        assert_eq!(api_request.temperature, Some(0.7));
        assert_eq!(api_request.system, Some("You are a helpful assistant.".to_string()));
        assert_eq!(api_request.stream, Some(false));
    }

    #[test]
    fn test_build_request_defaults() {
        let provider = AnthropicProvider::new("test-key".to_string());
        let request = CompletionRequest::new(
            "claude-3-5-sonnet-20241022".to_string(),
            vec![Message::user("Hello")],
        );

        let api_request = provider.build_request(&request, true);
        assert_eq!(api_request.max_tokens, 4096); // default
        assert_eq!(api_request.stream, Some(true));
    }

    #[test]
    fn test_build_request_with_messages() {
        let provider = AnthropicProvider::new("test-key".to_string());
        let request = CompletionRequest::new(
            "claude-3-5-sonnet-20241022".to_string(),
            vec![
                Message::user("Hello"),
                Message::assistant("Hi there!"),
                Message::user("How are you?"),
            ],
        );

        let api_request = provider.build_request(&request, false);
        assert_eq!(api_request.messages.len(), 3);
    }

    #[test]
    fn test_build_request_with_tools() {
        use autohands_protocols::tool::ToolDefinition;

        let provider = AnthropicProvider::new("test-key".to_string());
        let tools = vec![
            ToolDefinition::new("read_file", "Read File", "Read a file from disk"),
        ];
        let request = CompletionRequest::new(
            "claude-3-5-sonnet-20241022".to_string(),
            vec![Message::user("Read file.txt")],
        ).with_tools(tools);

        let api_request = provider.build_request(&request, false);
        assert!(!api_request.tools.is_empty());
    }

    #[test]
    fn test_build_request_no_tools() {
        let provider = AnthropicProvider::new("test-key".to_string());
        let request = CompletionRequest::new(
            "claude-3-5-sonnet-20241022".to_string(),
            vec![Message::user("Hello")],
        );

        let api_request = provider.build_request(&request, false);
        // Empty tools should be an empty vec
        assert!(api_request.tools.is_empty());
    }

    #[test]
    fn test_provider_new() {
        let provider = AnthropicProvider::new("my-api-key".to_string());
        assert_eq!(provider.api_key, "my-api-key");
    }

    #[test]
    fn test_api_url_constant() {
        assert_eq!(API_URL, "https://api.anthropic.com/v1/messages");
    }

    #[test]
    fn test_api_version_constant() {
        assert_eq!(API_VERSION, "2024-01-01");
    }

    #[test]
    fn test_models_have_claude_3_5_sonnet() {
        let provider = AnthropicProvider::new("test-key".to_string());
        let models = provider.models();
        assert!(models.iter().any(|m| m.id.contains("sonnet")));
    }

    #[test]
    fn test_models_have_vision_support() {
        let provider = AnthropicProvider::new("test-key".to_string());
        let models = provider.models();
        // At least some models should support vision
        let vision_models = models.iter().filter(|m| m.supports_vision).count();
        assert!(vision_models > 0);
    }

    #[test]
    fn test_capabilities_detail() {
        let provider = AnthropicProvider::new("test-key".to_string());
        let caps = provider.capabilities();
        // Anthropic should support batching and prompt caching
        assert!(caps.batching);
        assert!(caps.prompt_caching);
    }

    // Wiremock-based tests for actual HTTP calls
    mod http_tests {
        use super::*;
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        // Test helper provider with configurable base URL
        struct TestableAnthropicProvider {
            api_key: String,
            client: reqwest::Client,
            base_url: String,
        }

        impl TestableAnthropicProvider {
            fn new(api_key: String, base_url: String) -> Self {
                Self {
                    api_key,
                    client: reqwest::Client::new(),
                    base_url,
                }
            }

            fn build_request(&self, request: &CompletionRequest, stream: bool) -> ApiRequest {
                ApiRequest {
                    model: request.model.clone(),
                    messages: convert_messages(&request.messages),
                    system: request.system.clone(),
                    max_tokens: request.max_tokens.unwrap_or(4096),
                    temperature: request.temperature,
                    tools: convert_tools(request),
                    stream: Some(stream),
                }
            }

            async fn send_request(&self, api_request: &ApiRequest) -> Result<reqwest::Response, ProviderError> {
                let response = self
                    .client
                    .post(&format!("{}/messages", self.base_url))
                    .header("x-api-key", &self.api_key)
                    .header("anthropic-version", API_VERSION)
                    .header("content-type", "application/json")
                    .json(api_request)
                    .send()
                    .await
                    .map_err(|e| ProviderError::Network(e.to_string()))?;

                if !response.status().is_success() {
                    let status = response.status().as_u16();
                    let body = response.text().await.unwrap_or_default();
                    let message = serde_json::from_str::<serde_json::Value>(&body)
                        .ok()
                        .and_then(|v| v["error"]["message"].as_str().map(String::from))
                        .unwrap_or(body);
                    return Err(ProviderError::from_api_response(status, message));
                }

                Ok(response)
            }

            async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
                let api_request = self.build_request(&request, false);
                let response = self.send_request(&api_request).await?;
                let api_response: ApiResponse = response
                    .json()
                    .await
                    .map_err(|e| ProviderError::Network(e.to_string()))?;
                Ok(parse_response(api_response))
            }
        }

        #[tokio::test]
        async fn test_complete_success() {
            let mock_server = MockServer::start().await;

            // Use the exact format expected by ApiResponse struct
            let response_body = serde_json::json!({
                "id": "msg_01XFDUDYJgAACzvnptvVoYEL",
                "model": "claude-3-5-sonnet-20241022",
                "content": [{"type": "text", "text": "Hello back!"}],
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 5
                }
            }).to_string();
            let response_body: &str = &response_body;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/messages"))
                .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = TestableAnthropicProvider::new("test-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "claude-3-5-sonnet-20241022".to_string(),
                vec![Message::user("Hello")],
            );

            let result = provider.complete(request).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            assert!(response.message.content.text().contains("Hello back"));
        }

        #[tokio::test]
        async fn test_complete_api_error() {
            let mock_server = MockServer::start().await;

            let error_body = r#"{"error": {"type": "invalid_request_error", "message": "Invalid API key"}}"#;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/messages"))
                .respond_with(ResponseTemplate::new(401).set_body_string(error_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = TestableAnthropicProvider::new("bad-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "claude-3-5-sonnet-20241022".to_string(),
                vec![Message::user("Hello")],
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
        async fn test_complete_rate_limit() {
            let mock_server = MockServer::start().await;

            let error_body = r#"{"error": {"type": "rate_limit_error", "message": "Rate limit exceeded"}}"#;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/messages"))
                .respond_with(ResponseTemplate::new(429).set_body_string(error_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = TestableAnthropicProvider::new("test-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "claude-3-5-sonnet-20241022".to_string(),
                vec![Message::user("Hello")],
            );

            let result = provider.complete(request).await;
            assert!(result.is_err());
            match result.unwrap_err() {
                ProviderError::RateLimited { .. } => {}
                _ => panic!("Expected RateLimited"),
            }
        }

        #[tokio::test]
        async fn test_complete_server_error() {
            let mock_server = MockServer::start().await;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/messages"))
                .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = TestableAnthropicProvider::new("test-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "claude-3-5-sonnet-20241022".to_string(),
                vec![Message::user("Hello")],
            );

            let result = provider.complete(request).await;
            assert!(result.is_err());
            match result.unwrap_err() {
                ProviderError::ApiError { status, message } => {
                    assert_eq!(status, 500);
                    assert!(message.contains("Internal Server Error"));
                }
                _ => panic!("Expected ApiError"),
            }
        }

        #[tokio::test]
        async fn test_complete_with_tool_use() {
            let mock_server = MockServer::start().await;

            let response_body = serde_json::json!({
                "id": "msg_01",
                "model": "claude-3-5-sonnet-20241022",
                "content": [
                    {"type": "text", "text": "I'll read the file for you."},
                    {"type": "tool_use", "id": "toolu_01", "name": "read_file", "input": {"path": "test.txt"}}
                ],
                "stop_reason": "tool_use",
                "usage": {
                    "input_tokens": 20,
                    "output_tokens": 15
                }
            }).to_string();
            let response_body: &str = &response_body;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/messages"))
                .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = TestableAnthropicProvider::new("test-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "claude-3-5-sonnet-20241022".to_string(),
                vec![Message::user("Read test.txt")],
            );

            let result = provider.complete(request).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            assert!(!response.message.tool_calls.is_empty());
            assert_eq!(response.message.tool_calls[0].name, "read_file");
        }
    }
