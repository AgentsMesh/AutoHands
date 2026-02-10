//! History compression via summarization.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use autohands_protocols::error::ProviderError;
use autohands_protocols::provider::{CompletionRequest, LLMProvider};
use autohands_protocols::types::Message;

/// Configuration for history summarization.
#[derive(Debug, Clone)]
pub struct SummarizerConfig {
    /// Maximum messages before triggering summarization.
    pub max_messages: usize,
    /// Number of recent messages to keep unsummarized.
    pub keep_recent: usize,
    /// Model to use for summarization.
    pub model: String,
    /// Maximum tokens for summary.
    pub max_summary_tokens: u32,
}

impl Default for SummarizerConfig {
    fn default() -> Self {
        Self {
            max_messages: 50,
            keep_recent: 10,
            model: "claude-3-haiku-20240307".to_string(),
            max_summary_tokens: 1024,
        }
    }
}

/// Summary of conversation history.
#[derive(Debug, Clone)]
pub struct ConversationSummary {
    /// The summary text.
    pub content: String,
    /// Number of messages that were summarized.
    pub message_count: usize,
    /// Timestamp when summary was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ConversationSummary {
    pub fn new(content: String, message_count: usize) -> Self {
        Self {
            content,
            message_count,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Trait for conversation summarization.
#[async_trait]
pub trait Summarizer: Send + Sync {
    /// Summarize a list of messages.
    async fn summarize(&self, messages: &[Message]) -> Result<String, ProviderError>;

    /// Check if summarization is needed.
    fn needs_summarization(&self, message_count: usize) -> bool;
}

/// LLM-based summarizer.
pub struct LLMSummarizer {
    provider: Arc<dyn LLMProvider>,
    config: SummarizerConfig,
}

impl LLMSummarizer {
    /// Create a new LLM summarizer.
    pub fn new(provider: Arc<dyn LLMProvider>, config: SummarizerConfig) -> Self {
        Self { provider, config }
    }

    fn build_summarization_prompt(&self, messages: &[Message]) -> String {
        let mut conversation = String::new();

        for msg in messages {
            let role = match msg.role {
                autohands_protocols::types::MessageRole::User => "User",
                autohands_protocols::types::MessageRole::Assistant => "Assistant",
                autohands_protocols::types::MessageRole::System => "System",
                autohands_protocols::types::MessageRole::Tool => "Tool",
            };
            conversation.push_str(&format!("{}: {}\n", role, msg.content.text()));
        }

        conversation
    }
}

#[async_trait]
impl Summarizer for LLMSummarizer {
    async fn summarize(&self, messages: &[Message]) -> Result<String, ProviderError> {
        if messages.is_empty() {
            return Ok(String::new());
        }

        debug!("Summarizing {} messages", messages.len());

        let conversation = self.build_summarization_prompt(messages);
        let system = r#"You are a conversation summarizer. Summarize the following conversation in a concise way, preserving:
1. Key topics discussed
2. Important decisions made
3. Action items or tasks mentioned
4. Any important context for future reference

Be brief but comprehensive. Focus on information that would be useful to continue the conversation."#;

        let request = CompletionRequest::new(
            self.config.model.clone(),
            vec![Message::user(format!("Summarize this conversation:\n\n{}", conversation))],
        )
        .with_system(system)
        .with_max_tokens(self.config.max_summary_tokens);

        let response = self.provider.complete(request).await?;
        Ok(response.message.content.text())
    }

    fn needs_summarization(&self, message_count: usize) -> bool {
        message_count > self.config.max_messages
    }
}

/// History compressor that manages summarization.
pub struct HistoryCompressor {
    summarizer: Arc<dyn Summarizer>,
    config: SummarizerConfig,
}

impl HistoryCompressor {
    /// Create a new history compressor.
    pub fn new(summarizer: Arc<dyn Summarizer>, config: SummarizerConfig) -> Self {
        Self { summarizer, config }
    }

    /// Compress history if needed.
    pub async fn compress(
        &self,
        messages: Vec<Message>,
    ) -> Result<(Vec<Message>, Option<ConversationSummary>), ProviderError> {
        if !self.summarizer.needs_summarization(messages.len()) {
            return Ok((messages, None));
        }

        let split_point = messages.len().saturating_sub(self.config.keep_recent);
        let (to_summarize, to_keep) = messages.split_at(split_point);

        if to_summarize.is_empty() {
            return Ok((messages, None));
        }

        let summary_text = self.summarizer.summarize(to_summarize).await?;
        let summary = ConversationSummary::new(summary_text.clone(), to_summarize.len());

        // Create new message list with summary
        let mut result = vec![Message::system(format!(
            "[Previous conversation summary: {}]",
            summary_text
        ))];
        result.extend(to_keep.iter().cloned());

        debug!(
            "Compressed {} messages into summary + {} recent messages",
            to_summarize.len(),
            to_keep.len()
        );

        Ok((result, Some(summary)))
    }
}

#[cfg(test)]
#[path = "summarizer_tests.rs"]
mod tests;
