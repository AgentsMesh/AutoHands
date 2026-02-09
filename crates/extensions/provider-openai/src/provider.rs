//! OpenAI provider implementation.

use async_trait::async_trait;
use futures::StreamExt;

use autohands_protocols::error::ProviderError;
use autohands_protocols::provider::{
    ChunkType, CompletionChunk, CompletionRequest, CompletionResponse, CompletionStream,
    LLMProvider, ModelDefinition, ProviderCapabilities,
};
use autohands_protocols::types::StopReason;

use crate::api::ApiRequest;
use crate::converter::{convert_messages, convert_tools};
use crate::models::get_models;
use crate::parser::{parse_response, parse_stream_chunk};

const DEFAULT_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI LLM provider.
pub struct OpenAIProvider {
    api_key: String,
    api_url: String,
    client: reqwest::Client,
    models: Vec<ModelDefinition>,
    capabilities: ProviderCapabilities,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_url(api_key, DEFAULT_API_URL.to_string())
    }

    /// Create provider with custom API URL (for OpenAI-compatible APIs).
    pub fn with_url(api_key: String, api_url: String) -> Self {
        Self {
            api_key,
            api_url,
            client: reqwest::Client::new(),
            models: get_models(),
            capabilities: ProviderCapabilities {
                streaming: true,
                tool_calling: true,
                vision: true,
                json_mode: true,
                prompt_caching: false,
                batching: true,
                max_concurrent: Some(100),
            },
        }
    }

    fn build_request(&self, request: &CompletionRequest, stream: bool) -> ApiRequest {
        ApiRequest {
            model: request.model.clone(),
            messages: convert_messages(&request.messages),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            tools: convert_tools(request),
            stream: Some(stream),
            response_format: None,
        }
    }

    async fn send_request(&self, api_request: &ApiRequest) -> Result<reqwest::Response, ProviderError> {
        let response = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(api_request)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError { status, message: text });
        }

        Ok(response)
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn id(&self) -> &str {
        "openai"
    }

    fn models(&self) -> &[ModelDefinition] {
        &self.models
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        let api_request = self.build_request(&request, false);
        let response = self.send_request(&api_request).await?;
        let api_response: crate::api::ApiResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;
        Ok(parse_response(api_response))
    }

    async fn complete_stream(&self, request: CompletionRequest) -> Result<CompletionStream, ProviderError> {
        let api_request = self.build_request(&request, true);
        let response = self.send_request(&api_request).await?;

        let stream = response.bytes_stream().map(move |result| {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                return Ok(CompletionChunk {
                                    chunk_type: ChunkType::MessageEnd,
                                    delta: None,
                                    tool_call: None,
                                    stop_reason: Some(StopReason::EndTurn),
                                    usage: None,
                                });
                            }
                            if let Ok(chunk) = serde_json::from_str::<crate::api::StreamChunk>(data) {
                                return Ok(parse_stream_chunk(chunk));
                            }
                        }
                    }
                    Ok(CompletionChunk {
                        chunk_type: ChunkType::ContentDelta,
                        delta: None,
                        tool_call: None,
                        stop_reason: None,
                        usage: None,
                    })
                }
                Err(e) => Err(ProviderError::StreamError(e.to_string())),
            }
        });

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use autohands_protocols::types::Message;

    #[test]
    fn test_provider_id() {
        let provider = OpenAIProvider::new("test-key".to_string());
        assert_eq!(provider.id(), "openai");
    }

    #[test]
    fn test_provider_capabilities() {
        let provider = OpenAIProvider::new("test-key".to_string());
        let caps = provider.capabilities();
        assert!(caps.streaming);
        assert!(caps.tool_calling);
        assert!(caps.vision);
        assert!(caps.json_mode);
    }

    #[test]
    fn test_models_not_empty() {
        let provider = OpenAIProvider::new("test-key".to_string());
        assert!(!provider.models().is_empty());
    }

    #[test]
    fn test_custom_url() {
        let provider = OpenAIProvider::with_url(
            "test-key".to_string(),
            "https://custom.api/v1".to_string(),
        );
        assert_eq!(provider.api_url, "https://custom.api/v1");
    }

    #[test]
    fn test_provider_capabilities_batching() {
        let provider = OpenAIProvider::new("test-key".to_string());
        let caps = provider.capabilities();
        assert!(caps.batching);
        assert!(!caps.prompt_caching);
        assert_eq!(caps.max_concurrent, Some(100));
    }

    #[test]
    fn test_provider_creation_with_empty_key() {
        let provider = OpenAIProvider::new(String::new());
        assert_eq!(provider.api_key, "");
        assert_eq!(provider.id(), "openai");
    }

    #[test]
    fn test_default_api_url_constant() {
        assert_eq!(DEFAULT_API_URL, "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn test_provider_default_url() {
        let provider = OpenAIProvider::new("key".to_string());
        assert_eq!(provider.api_url, DEFAULT_API_URL);
    }

    #[test]
    fn test_build_request_basic() {
        let provider = OpenAIProvider::new("key".to_string());
        let request = CompletionRequest::new(
            "gpt-4".to_string(),
            vec![Message::user("Hello")],
        );
        let api_request = provider.build_request(&request, false);
        assert_eq!(api_request.model, "gpt-4");
        assert_eq!(api_request.stream, Some(false));
    }

    #[test]
    fn test_build_request_with_stream() {
        let provider = OpenAIProvider::new("key".to_string());
        let request = CompletionRequest::new(
            "gpt-4".to_string(),
            vec![Message::user("Hello")],
        );
        let api_request = provider.build_request(&request, true);
        assert_eq!(api_request.stream, Some(true));
    }

    #[test]
    fn test_build_request_with_max_tokens() {
        let provider = OpenAIProvider::new("key".to_string());
        let request = CompletionRequest::new(
            "gpt-4".to_string(),
            vec![Message::user("Hello")],
        ).with_max_tokens(1000);
        let api_request = provider.build_request(&request, false);
        assert_eq!(api_request.max_tokens, Some(1000));
    }

    #[test]
    fn test_build_request_with_temperature() {
        let provider = OpenAIProvider::new("key".to_string());
        let request = CompletionRequest::new(
            "gpt-4".to_string(),
            vec![Message::user("Hello")],
        ).with_temperature(0.7);
        let api_request = provider.build_request(&request, false);
        assert_eq!(api_request.temperature, Some(0.7));
    }

    #[test]
    fn test_provider_models_contain_gpt4() {
        let provider = OpenAIProvider::new("key".to_string());
        let models = provider.models();
        let has_gpt4 = models.iter().any(|m| m.id.contains("gpt-4"));
        assert!(has_gpt4);
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
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello back!"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 5,
                    "total_tokens": 15
                }
            }).to_string();

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(200).set_body_string(&response_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = OpenAIProvider::with_url("test-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "gpt-4".to_string(),
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

            let error_body = r#"{"error": {"message": "Invalid API key", "type": "invalid_request_error"}}"#;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(401).set_body_string(error_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = OpenAIProvider::with_url("bad-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "gpt-4".to_string(),
                vec![Message::user("Hello")],
            );

            let result = provider.complete(request).await;
            assert!(result.is_err());
            match result.unwrap_err() {
                ProviderError::ApiError { status, message } => {
                    assert_eq!(status, 401);
                    assert!(message.contains("Invalid API key"));
                }
                _ => panic!("Expected ApiError"),
            }
        }

        #[tokio::test]
        async fn test_complete_rate_limit() {
            let mock_server = MockServer::start().await;

            let error_body = r#"{"error": {"message": "Rate limit exceeded", "type": "rate_limit_error"}}"#;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(429).set_body_string(error_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = OpenAIProvider::with_url("test-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "gpt-4".to_string(),
                vec![Message::user("Hello")],
            );

            let result = provider.complete(request).await;
            assert!(result.is_err());
            match result.unwrap_err() {
                ProviderError::ApiError { status, .. } => {
                    assert_eq!(status, 429);
                }
                _ => panic!("Expected ApiError"),
            }
        }

        #[tokio::test]
        async fn test_complete_server_error() {
            let mock_server = MockServer::start().await;

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = OpenAIProvider::with_url("test-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "gpt-4".to_string(),
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
                "id": "chatcmpl-tool",
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": "call_123",
                            "type": "function",
                            "function": {
                                "name": "read_file",
                                "arguments": "{\"path\": \"test.txt\"}"
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
            }).to_string();

            Mock::given(matchers::method("POST"))
                .and(matchers::path("/"))
                .respond_with(ResponseTemplate::new(200).set_body_string(&response_body))
                .expect(1)
                .mount(&mock_server)
                .await;

            let provider = OpenAIProvider::with_url("test-key".to_string(), mock_server.uri());
            let request = CompletionRequest::new(
                "gpt-4".to_string(),
                vec![Message::user("Read test.txt")],
            );

            let result = provider.complete(request).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            assert!(!response.message.tool_calls.is_empty());
            assert_eq!(response.message.tool_calls[0].name, "read_file");
        }
    }
}
