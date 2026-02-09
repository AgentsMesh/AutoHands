//! Channel registry for managing communication channels.
//!
//! Channels are bidirectional communication adapters (Web, Telegram, WeChat, etc.)
//! that enable users to interact with AutoHands agents.

use std::sync::Arc;

use autohands_protocols::channel::{Channel, OutboundMessage, ReplyAddress, SentMessage};
use autohands_protocols::error::{ChannelError, ExtensionError};

use super::base::{BaseRegistry, Registerable};

/// Wrapper to implement Registerable for Channel trait objects.
struct ChannelWrapper(Arc<dyn Channel>);

impl Registerable for ChannelWrapper {
    fn registry_id(&self) -> &str {
        self.0.id()
    }
}

/// Registry for managing channels.
///
/// Provides thread-safe registration, lookup, and messaging capabilities
/// for all registered channels.
pub struct ChannelRegistry {
    inner: BaseRegistry<ChannelWrapper>,
}

impl ChannelRegistry {
    /// Create a new channel registry.
    pub fn new() -> Self {
        Self {
            inner: BaseRegistry::new(),
        }
    }

    /// Register a channel.
    ///
    /// # Errors
    ///
    /// Returns an error if a channel with the same ID is already registered.
    pub fn register(&self, channel: Arc<dyn Channel>) -> Result<(), ExtensionError> {
        self.inner.register(Arc::new(ChannelWrapper(channel)))
    }

    /// Unregister a channel by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if no channel with the given ID exists.
    pub fn unregister(&self, id: &str) -> Result<(), ExtensionError> {
        self.inner.unregister(id)
    }

    /// Get a channel by ID.
    pub fn get(&self, id: &str) -> Option<Arc<dyn Channel>> {
        self.inner.get(id).map(|wrapper| wrapper.0.clone())
    }

    /// Check if a channel is registered.
    pub fn contains(&self, id: &str) -> bool {
        self.inner.contains(id)
    }

    /// List all registered channel IDs.
    pub fn list_ids(&self) -> Vec<String> {
        self.inner.list_ids()
    }

    /// Get the number of registered channels.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Send a message to the specified reply address.
    ///
    /// This method looks up the appropriate channel based on the reply address
    /// and sends the message through that channel.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The channel specified in the reply address is not found
    /// - The channel fails to send the message
    pub async fn send(
        &self,
        reply_to: &ReplyAddress,
        message: OutboundMessage,
    ) -> Result<SentMessage, ChannelError> {
        let channel = self
            .get(&reply_to.channel_id)
            .ok_or_else(|| ChannelError::NotFound(reply_to.channel_id.clone()))?;

        channel.send(reply_to, message).await
    }

    /// Start all registered channels.
    ///
    /// # Errors
    ///
    /// Returns an error if any channel fails to start.
    pub async fn start_all(&self) -> Result<(), ChannelError> {
        for wrapper in self.inner.iter() {
            wrapper.0.start().await?;
        }
        Ok(())
    }

    /// Stop all registered channels.
    ///
    /// # Errors
    ///
    /// Returns an error if any channel fails to stop.
    pub async fn stop_all(&self) -> Result<(), ChannelError> {
        for wrapper in self.inner.iter() {
            wrapper.0.stop().await?;
        }
        Ok(())
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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
}
