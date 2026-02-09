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
mod tests {
    use super::*;
    use autohands_protocols::provider::{
        CompletionResponse, CompletionStream, ModelDefinition, ProviderCapabilities,
    };
    use autohands_protocols::types::{StopReason, Usage};

    struct MockSummarizer {
        max_messages: usize,
    }

    #[async_trait]
    impl Summarizer for MockSummarizer {
        async fn summarize(&self, messages: &[Message]) -> Result<String, ProviderError> {
            Ok(format!("Summary of {} messages", messages.len()))
        }

        fn needs_summarization(&self, message_count: usize) -> bool {
            message_count > self.max_messages
        }
    }

    struct MockProvider {
        capabilities: ProviderCapabilities,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                capabilities: ProviderCapabilities::default(),
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        fn id(&self) -> &str {
            "mock"
        }

        fn models(&self) -> &[ModelDefinition] {
            &[]
        }

        fn capabilities(&self) -> &ProviderCapabilities {
            &self.capabilities
        }

        async fn complete(&self, _: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
            Ok(CompletionResponse {
                id: "test".to_string(),
                model: "mock".to_string(),
                message: Message::assistant("Test summary of the conversation"),
                stop_reason: StopReason::EndTurn,
                usage: Usage::default(),
                metadata: Default::default(),
            })
        }

        async fn complete_stream(&self, _: CompletionRequest) -> Result<CompletionStream, ProviderError> {
            Err(ProviderError::Network("Not implemented".to_string()))
        }
    }

    #[test]
    fn test_summarizer_config_default() {
        let config = SummarizerConfig::default();
        assert_eq!(config.max_messages, 50);
        assert_eq!(config.keep_recent, 10);
    }

    #[test]
    fn test_conversation_summary_new() {
        let summary = ConversationSummary::new("Test summary".to_string(), 10);
        assert_eq!(summary.content, "Test summary");
        assert_eq!(summary.message_count, 10);
    }

    #[test]
    fn test_needs_summarization() {
        let summarizer = MockSummarizer { max_messages: 5 };
        assert!(!summarizer.needs_summarization(3));
        assert!(!summarizer.needs_summarization(5));
        assert!(summarizer.needs_summarization(6));
    }

    #[tokio::test]
    async fn test_mock_summarize() {
        let summarizer = MockSummarizer { max_messages: 5 };
        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi"),
        ];

        let result = summarizer.summarize(&messages).await.unwrap();
        assert_eq!(result, "Summary of 2 messages");
    }

    #[tokio::test]
    async fn test_compressor_no_compression_needed() {
        let summarizer = Arc::new(MockSummarizer { max_messages: 10 });
        let config = SummarizerConfig {
            max_messages: 10,
            keep_recent: 5,
            ..Default::default()
        };
        let compressor = HistoryCompressor::new(summarizer, config);

        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi"),
        ];

        let (result, summary) = compressor.compress(messages.clone()).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(summary.is_none());
    }

    #[tokio::test]
    async fn test_compressor_with_compression() {
        let summarizer = Arc::new(MockSummarizer { max_messages: 3 });
        let config = SummarizerConfig {
            max_messages: 3,
            keep_recent: 2,
            ..Default::default()
        };
        let compressor = HistoryCompressor::new(summarizer, config);

        let messages = vec![
            Message::user("Message 1"),
            Message::assistant("Response 1"),
            Message::user("Message 2"),
            Message::assistant("Response 2"),
            Message::user("Message 3"),
        ];

        let (result, summary) = compressor.compress(messages).await.unwrap();

        // Summary message + 2 recent messages
        assert_eq!(result.len(), 3);
        assert!(summary.is_some());
        assert_eq!(summary.unwrap().message_count, 3);
    }

    #[tokio::test]
    async fn test_llm_summarizer() {
        let provider = Arc::new(MockProvider::new());
        let config = SummarizerConfig::default();
        let summarizer = LLMSummarizer::new(provider, config);

        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi there!"),
        ];

        let result = summarizer.summarize(&messages).await.unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_build_summarization_prompt() {
        let provider = Arc::new(MockProvider::new());
        let config = SummarizerConfig::default();
        let summarizer = LLMSummarizer::new(provider, config);

        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi"),
        ];

        let prompt = summarizer.build_summarization_prompt(&messages);
        assert!(prompt.contains("User: Hello"));
        assert!(prompt.contains("Assistant: Hi"));
    }

    #[tokio::test]
    async fn test_llm_summarizer_empty_messages() {
        let provider = Arc::new(MockProvider::new());
        let config = SummarizerConfig::default();
        let summarizer = LLMSummarizer::new(provider, config);

        let result = summarizer.summarize(&[]).await.unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_llm_summarizer_needs_summarization() {
        let provider = Arc::new(MockProvider::new());
        let config = SummarizerConfig {
            max_messages: 10,
            ..Default::default()
        };
        let summarizer = LLMSummarizer::new(provider, config);

        assert!(!summarizer.needs_summarization(5));
        assert!(!summarizer.needs_summarization(10));
        assert!(summarizer.needs_summarization(11));
    }

    #[test]
    fn test_summarizer_config_clone() {
        let config = SummarizerConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.max_messages, config.max_messages);
        assert_eq!(cloned.keep_recent, config.keep_recent);
    }

    #[test]
    fn test_conversation_summary_clone() {
        let summary = ConversationSummary::new("Test".to_string(), 5);
        let cloned = summary.clone();
        assert_eq!(cloned.content, summary.content);
        assert_eq!(cloned.message_count, summary.message_count);
    }

    #[test]
    fn test_build_summarization_prompt_all_roles() {
        let provider = Arc::new(MockProvider::new());
        let config = SummarizerConfig::default();
        let summarizer = LLMSummarizer::new(provider, config);

        let messages = vec![
            Message::system("System prompt"),
            Message::user("User message"),
            Message::assistant("Assistant response"),
            Message::tool("call_1", "Tool result"),
        ];

        let prompt = summarizer.build_summarization_prompt(&messages);
        assert!(prompt.contains("System: System prompt"));
        assert!(prompt.contains("User: User message"));
        assert!(prompt.contains("Assistant: Assistant response"));
        assert!(prompt.contains("Tool: Tool result"));
    }

    #[tokio::test]
    async fn test_compressor_empty_to_summarize() {
        let summarizer = Arc::new(MockSummarizer { max_messages: 1 });
        let config = SummarizerConfig {
            max_messages: 1,
            keep_recent: 10, // More than messages, so nothing to summarize
            ..Default::default()
        };
        let compressor = HistoryCompressor::new(summarizer, config);

        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi"),
        ];

        let (result, summary) = compressor.compress(messages.clone()).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(summary.is_none());
    }
}
