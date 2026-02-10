//! Provider registry for managing LLM providers.

use std::sync::Arc;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::ProviderRegistryAccess;
use autohands_protocols::provider::{LLMProvider, ModelDefinition};

use super::base::{BaseRegistry, Registerable};

// Implement Registerable for LLMProvider trait objects
impl Registerable for dyn LLMProvider {
    fn registry_id(&self) -> &str {
        self.id()
    }
}

/// Registry for managing LLM providers.
///
/// Built on `BaseRegistry` for consistent behavior.
pub struct ProviderRegistry {
    inner: BaseRegistry<dyn LLMProvider>,
}

impl ProviderRegistry {
    /// Create a new provider registry.
    pub fn new() -> Self {
        Self {
            inner: BaseRegistry::new(),
        }
    }

    /// Register a provider.
    pub fn register(&self, provider: Arc<dyn LLMProvider>) -> Result<(), ExtensionError> {
        self.inner.register(provider)
    }

    /// Unregister a provider.
    pub fn unregister(&self, id: &str) -> Result<(), ExtensionError> {
        self.inner.unregister(id)
    }

    /// Get a provider by ID.
    pub fn get(&self, id: &str) -> Option<Arc<dyn LLMProvider>> {
        self.inner.get(id)
    }

    /// List all provider IDs.
    pub fn list_ids(&self) -> Vec<String> {
        self.inner.list_ids()
    }

    /// List all available models across all providers.
    pub fn list_models(&self) -> Vec<(String, ModelDefinition)> {
        let mut result = Vec::new();
        for provider in self.inner.iter() {
            let provider_id = provider.id().to_string();
            for model in provider.models() {
                result.push((provider_id.clone(), model.clone()));
            }
        }
        result
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderRegistryAccess for ProviderRegistry {
    fn register_provider(&self, provider: Arc<dyn LLMProvider>) -> Result<(), ExtensionError> {
        self.register(provider)
    }

    fn unregister_provider(&self, provider_id: &str) -> Result<(), ExtensionError> {
        self.unregister(provider_id)
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
