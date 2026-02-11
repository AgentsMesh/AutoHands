//! Gemini API client.

use std::pin::Pin;

use futures::Stream;
use reqwest::Client;
use tracing::debug;

use autohands_protocols::error::ProviderError;

use crate::types::*;

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Gemini API client.
pub struct GeminiClient {
    client: Client,
    api_key: String,
}

impl GeminiClient {
    /// Create a new Gemini client.
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("Failed to build HTTP client"),
            api_key,
        }
    }

    /// Generate content (non-streaming).
    pub async fn generate_content(
        &self,
        model: &str,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse, ProviderError> {
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            BASE_URL, model, self.api_key
        );

        debug!("Gemini generate_content: model={}", model);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        if !status.is_success() {
            let error: Result<GeminiError, _> = serde_json::from_str(&body);
            return match error {
                Ok(e) => Err(ProviderError::from_api_response(
                    status.as_u16(),
                    e.error.message,
                )),
                Err(_) => Err(ProviderError::from_api_response(
                    status.as_u16(),
                    body,
                )),
            };
        }

        serde_json::from_str(&body).map_err(|e| {
            ProviderError::ApiError {
                status: 500,
                message: format!("Failed to parse response: {}", e),
            }
        })
    }

    /// Generate content (streaming).
    pub async fn generate_content_stream(
        &self,
        model: &str,
        request: GenerateContentRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>>, ProviderError>
    {
        let url = format!(
            "{}/models/{}:streamGenerateContent?key={}&alt=sse",
            BASE_URL, model, self.api_key
        );

        debug!("Gemini stream generate_content: model={}", model);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .map_err(|e| ProviderError::Network(e.to_string()))?;
            let error: Result<GeminiError, _> = serde_json::from_str(&body);
            return match error {
                Ok(e) => Err(ProviderError::from_api_response(
                    status.as_u16(),
                    e.error.message,
                )),
                Err(_) => Err(ProviderError::from_api_response(
                    status.as_u16(),
                    body,
                )),
            };
        }

        let stream = async_stream::stream! {
            let mut bytes_stream = response.bytes_stream();
            use futures::StreamExt;
            let mut buffer = String::new();

            while let Some(chunk) = bytes_stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&text);

                        // Process complete SSE events
                        while let Some(pos) = buffer.find("\n\n") {
                            let event = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();

                            if let Some(data) = event.strip_prefix("data: ") {
                                if data.trim() == "[DONE]" {
                                    continue;
                                }
                                match serde_json::from_str::<StreamChunk>(data) {
                                    Ok(chunk) => yield Ok(chunk),
                                    Err(e) => yield Err(ProviderError::StreamError(e.to_string())),
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(ProviderError::StreamError(e.to_string()));
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
