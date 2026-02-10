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
