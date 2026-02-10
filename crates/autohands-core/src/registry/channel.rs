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
#[path = "channel_tests.rs"]
mod tests;
