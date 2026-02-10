    use super::*;
    use async_trait::async_trait;
    use autohands_protocols::channel::{ChannelCapabilities, ChannelId, InboundMessage};
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::broadcast;

    struct MockChannel {
        id: ChannelId,
        capabilities: ChannelCapabilities,
        started: AtomicBool,
        message_tx: broadcast::Sender<InboundMessage>,
    }

    impl MockChannel {
        fn new(id: &str) -> Self {
            let (message_tx, _) = broadcast::channel(16);
            Self {
                id: id.to_string(),
                capabilities: ChannelCapabilities::default(),
                started: AtomicBool::new(false),
                message_tx,
            }
        }
    }

    #[async_trait]
    impl Channel for MockChannel {
        fn id(&self) -> &ChannelId {
            &self.id
        }

        fn capabilities(&self) -> &ChannelCapabilities {
            &self.capabilities
        }

        async fn start(&self) -> Result<(), ChannelError> {
            self.started.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn stop(&self) -> Result<(), ChannelError> {
            self.started.store(false, Ordering::SeqCst);
            Ok(())
        }

        async fn send(
            &self,
            _target: &ReplyAddress,
            _message: OutboundMessage,
        ) -> Result<SentMessage, ChannelError> {
            if !self.started.load(Ordering::SeqCst) {
                return Err(ChannelError::Disconnected);
            }
            Ok(SentMessage {
                id: "mock-msg-id".to_string(),
                timestamp: chrono::Utc::now(),
            })
        }

        fn inbound(&self) -> broadcast::Receiver<InboundMessage> {
            self.message_tx.subscribe()
        }
    }

    #[test]
    fn test_channel_registry_new() {
        let registry = ChannelRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_channel_registry_default() {
        let registry = ChannelRegistry::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_register_channel() {
        let registry = ChannelRegistry::new();
        let channel = Arc::new(MockChannel::new("test-channel"));

        let result = registry.register(channel);
        assert!(result.is_ok());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test-channel"));
    }

    #[test]
    fn test_register_duplicate_channel() {
        let registry = ChannelRegistry::new();
        let channel1 = Arc::new(MockChannel::new("test-channel"));
        let channel2 = Arc::new(MockChannel::new("test-channel"));

        registry.register(channel1).unwrap();
        let result = registry.register(channel2);
        assert!(result.is_err());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_unregister_channel() {
        let registry = ChannelRegistry::new();
        let channel = Arc::new(MockChannel::new("test-channel"));

        registry.register(channel).unwrap();
        let result = registry.unregister("test-channel");
        assert!(result.is_ok());
        assert!(registry.is_empty());
    }

    #[test]
    fn test_unregister_nonexistent_channel() {
        let registry = ChannelRegistry::new();
        let result = registry.unregister("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_channel() {
        let registry = ChannelRegistry::new();
        let channel = Arc::new(MockChannel::new("test-channel"));

        registry.register(channel).unwrap();
        let retrieved = registry.get("test-channel");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), "test-channel");
    }

    #[test]
    fn test_get_nonexistent_channel() {
        let registry = ChannelRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_list_ids() {
        let registry = ChannelRegistry::new();
        registry
            .register(Arc::new(MockChannel::new("channel-1")))
            .unwrap();
        registry
            .register(Arc::new(MockChannel::new("channel-2")))
            .unwrap();

        let ids = registry.list_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"channel-1".to_string()));
        assert!(ids.contains(&"channel-2".to_string()));
    }

    #[tokio::test]
    async fn test_send_message() {
        let registry = ChannelRegistry::new();
        let channel = Arc::new(MockChannel::new("test-channel"));

        registry.register(channel).unwrap();

        // Start the channel first
        let channel_ref = registry.get("test-channel").unwrap();
        channel_ref.start().await.unwrap();

        let reply_to = ReplyAddress::new("test-channel", "user-123");
        let message = OutboundMessage::text("Hello!");

        let result = registry.send(&reply_to, message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_to_unknown_channel() {
        let registry = ChannelRegistry::new();
        let reply_to = ReplyAddress::new("unknown-channel", "user-123");
        let message = OutboundMessage::text("Hello!");

        let result = registry.send(&reply_to, message).await;
        assert!(matches!(result, Err(ChannelError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_start_all() {
        let registry = ChannelRegistry::new();
        let channel1 = Arc::new(MockChannel::new("channel-1"));
        let channel2 = Arc::new(MockChannel::new("channel-2"));

        registry.register(channel1.clone()).unwrap();
        registry.register(channel2.clone()).unwrap();

        let result = registry.start_all().await;
        assert!(result.is_ok());

        assert!(channel1.started.load(Ordering::SeqCst));
        assert!(channel2.started.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_stop_all() {
        let registry = ChannelRegistry::new();
        let channel1 = Arc::new(MockChannel::new("channel-1"));
        let channel2 = Arc::new(MockChannel::new("channel-2"));

        registry.register(channel1.clone()).unwrap();
        registry.register(channel2.clone()).unwrap();

        registry.start_all().await.unwrap();
        let result = registry.stop_all().await;
        assert!(result.is_ok());

        assert!(!channel1.started.load(Ordering::SeqCst));
        assert!(!channel2.started.load(Ordering::SeqCst));
    }
