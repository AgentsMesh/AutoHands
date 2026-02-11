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
            let body = response.text().await.unwrap_or_default();
            // 解析 OpenAI 错误 JSON: {"error": {"message": "...", "type": "..."}}
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
#[path = "provider_tests.rs"]
mod tests;
