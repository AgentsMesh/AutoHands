//! Extension trait definition.

use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;

use super::{ExtensionContext, ExtensionManifest};
use crate::error::ExtensionError;

/// Core trait for all extensions.
///
/// Every extension must implement this trait. It provides:
/// - Metadata about the extension (via manifest)
/// - Lifecycle hooks (initialize, shutdown)
/// - Access to extension-specific state
#[async_trait]
pub trait Extension: Send + Sync + 'static {
    /// Returns the extension manifest.
    fn manifest(&self) -> &ExtensionManifest;

    /// Initialize the extension with the given context.
    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError>;

    /// Shutdown the extension.
    async fn shutdown(&self) -> Result<(), ExtensionError> {
        Ok(())
    }

    /// Returns a reference to the extension as `Any` for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Returns a mutable reference to the extension as `Any` for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Trait for accessing the tool registry from extensions.
pub trait ToolRegistryAccess: Send + Sync {
    /// Register a tool.
    fn register_tool(&self, tool: Arc<dyn crate::tool::Tool>) -> Result<(), ExtensionError>;

    /// Unregister a tool.
    fn unregister_tool(&self, tool_id: &str) -> Result<(), ExtensionError>;
}

/// Trait for accessing the provider registry from extensions.
pub trait ProviderRegistryAccess: Send + Sync {
    /// Register a provider.
    fn register_provider(
        &self,
        provider: Arc<dyn crate::provider::LLMProvider>,
    ) -> Result<(), ExtensionError>;

    /// Unregister a provider.
    fn unregister_provider(&self, provider_id: &str) -> Result<(), ExtensionError>;
}

/// Trait for accessing the memory registry from extensions.
pub trait MemoryRegistryAccess: Send + Sync {
    /// Register a memory backend.
    fn register_backend(
        &self,
        backend: Arc<dyn crate::memory::MemoryBackend>,
    ) -> Result<(), ExtensionError>;

    /// Unregister a memory backend.
    fn unregister_backend(&self, backend_id: &str) -> Result<(), ExtensionError>;
}
