//! LLM Provider trait definition.

use async_trait::async_trait;
use std::pin::Pin;
use futures::Stream;

use super::{
    CompletionChunk, CompletionRequest, CompletionResponse, ModelDefinition, ProviderCapabilities,
};
use crate::error::ProviderError;
use crate::types::Message;

/// Core trait for LLM providers.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Returns the provider ID.
    fn id(&self) -> &str;

    /// Returns the available models.
    fn models(&self) -> &[ModelDefinition];

    /// Returns the provider capabilities.
    fn capabilities(&self) -> &ProviderCapabilities;

    /// Generate a completion (non-streaming).
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError>;

    /// Generate a streaming completion.
    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionStream, ProviderError>;

    /// Count tokens for a message (optional).
    async fn count_tokens(&self, messages: &[Message], model: &str) -> Result<u32, ProviderError> {
        let _ = model;
        // Default: rough estimate (4 chars per token)
        let text: String = messages.iter().map(|m| m.content.text()).collect();
        Ok((text.len() / 4) as u32)
    }
}

/// Stream of completion chunks.
pub type CompletionStream =
    Pin<Box<dyn Stream<Item = Result<CompletionChunk, ProviderError>> + Send>>;
