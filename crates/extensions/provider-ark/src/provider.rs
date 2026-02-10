//! Ark LLM provider implementation.

use async_trait::async_trait;

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

/// Default Ark API URL (火山引擎方舟平台).
const DEFAULT_API_URL: &str = "https://ark.cn-beijing.volces.com/api/v3/chat/completions";

/// Ark LLM provider.
///
/// Supports the Ark platform (火山引擎方舟) which hosts Doubao (豆包) models.
/// The API is compatible with OpenAI's chat completion format.
pub struct ArkProvider {
    api_key: String,
    api_url: String,
    client: reqwest::Client,
    models: Vec<ModelDefinition>,
    capabilities: ProviderCapabilities,
}

impl ArkProvider {
    /// Create a new Ark provider with the given API key.
    pub fn new(api_key: String) -> Self {
        Self::with_url(api_key, DEFAULT_API_URL.to_string())
    }

    /// Create provider with custom API URL.
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
                max_concurrent: Some(50),
            },
        }
    }

    /// Add a custom model definition.
    ///
    /// This is useful when you need to use an endpoint ID that isn't in the
    /// default model list (e.g., a specific deployed model endpoint).
    pub fn with_custom_model(mut self, model: ModelDefinition) -> Self {
        self.models.push(model);
        self
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
            let body = response.text().await.unwrap_or_default();
            // 解析 Ark 错误 JSON: {"error": {"message": "...", "type": "..."}}
            let message = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(String::from))
                .unwrap_or(body);
            return Err(ProviderError::from_api_response(status, message));
        }

        Ok(response)
    }
}

#[async_trait]
impl LLMProvider for ArkProvider {
    fn id(&self) -> &str {
        "ark"
    }

    fn models(&self) -> &[ModelDefinition] {
        &self.models
    }

    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        let api_request = self.build_request(&request, false);

        // Debug log the request
        tracing::info!(
            "Ark API request: model={}, tools={}, messages={}",
            api_request.model,
            api_request.tools.len(),
            api_request.messages.len()
        );
        if !api_request.tools.is_empty() {
            tracing::debug!("Tools in request: {:?}", api_request.tools.iter().map(|t| &t.function.name).collect::<Vec<_>>());
        }

        let response = self.send_request(&api_request).await?;
        let api_response: crate::api::ApiResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        // Debug log the response
        if let Some(choice) = api_response.choices.first() {
            tracing::info!(
                "Ark API response: finish_reason={:?}, has_tool_calls={}",
                choice.finish_reason,
                !choice.message.tool_calls.is_empty()
            );
        }

        Ok(parse_response(api_response))
    }

    async fn complete_stream(&self, request: CompletionRequest) -> Result<CompletionStream, ProviderError> {
        let api_request = self.build_request(&request, true);
        let response = self.send_request(&api_request).await?;

        let stream = async_stream::stream! {
            let mut byte_stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(result) = futures::StreamExt::next(&mut byte_stream).await {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&text);

                        // Process complete lines from buffer
                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer[..newline_pos].trim().to_string();
                            buffer = buffer[newline_pos + 1..].to_string();

                            if line.is_empty() {
                                continue;
                            }

                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    yield Ok(CompletionChunk {
                                        chunk_type: ChunkType::MessageEnd,
                                        delta: None,
                                        tool_call: None,
                                        stop_reason: Some(StopReason::EndTurn),
                                        usage: None,
                                    });
                                    return;
                                }

                                if let Ok(chunk) = serde_json::from_str::<crate::api::StreamChunk>(data) {
                                    let parsed = parse_stream_chunk(chunk);
                                    // Only yield if there's actual non-empty content or it's a meaningful event
                                    let has_content = parsed.delta.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
                                    if has_content || parsed.stop_reason.is_some() || parsed.tool_call.is_some() {
                                        yield Ok(parsed);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(ProviderError::StreamError(e.to_string()));
                        return;
                    }
                }
            }

            // Process any remaining data in buffer
            if !buffer.is_empty() {
                if let Some(data) = buffer.trim().strip_prefix("data: ") {
                    if data == "[DONE]" {
                        yield Ok(CompletionChunk {
                            chunk_type: ChunkType::MessageEnd,
                            delta: None,
                            tool_call: None,
                            stop_reason: Some(StopReason::EndTurn),
                            usage: None,
                        });
                    } else if let Ok(chunk) = serde_json::from_str::<crate::api::StreamChunk>(data) {
                        let parsed = parse_stream_chunk(chunk);
                        let has_content = parsed.delta.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
                        if has_content || parsed.stop_reason.is_some() || parsed.tool_call.is_some() {
                            yield Ok(parsed);
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
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
}
