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
            client: reqwest::ClientBuilder::new()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("Failed to build HTTP client"),
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
#[path = "provider_tests.rs"]
mod tests;
