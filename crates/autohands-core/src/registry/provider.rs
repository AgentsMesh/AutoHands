//! Provider registry for managing LLM providers.

use dashmap::DashMap;
use std::sync::Arc;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::ProviderRegistryAccess;
use autohands_protocols::provider::{LLMProvider, ModelDefinition};

/// Registry for managing LLM providers.
pub struct ProviderRegistry {
    providers: DashMap<String, Arc<dyn LLMProvider>>,
}

impl ProviderRegistry {
    /// Create a new provider registry.
    pub fn new() -> Self {
        Self {
            providers: DashMap::new(),
        }
    }

    /// Register a provider.
    pub fn register(&self, provider: Arc<dyn LLMProvider>) -> Result<(), ExtensionError> {
        let id = provider.id().to_string();

        if self.providers.contains_key(&id) {
            return Err(ExtensionError::AlreadyRegistered(id));
        }

        self.providers.insert(id, provider);
        Ok(())
    }

    /// Unregister a provider.
    pub fn unregister(&self, id: &str) -> Result<(), ExtensionError> {
        self.providers
            .remove(id)
            .ok_or_else(|| ExtensionError::NotFound(id.to_string()))?;
        Ok(())
    }

    /// Get a provider by ID.
    pub fn get(&self, id: &str) -> Option<Arc<dyn LLMProvider>> {
        self.providers.get(id).map(|p| p.clone())
    }

    /// List all provider IDs.
    pub fn list_ids(&self) -> Vec<String> {
        self.providers.iter().map(|p| p.id().to_string()).collect()
    }

    /// List all available models across all providers.
    pub fn list_models(&self) -> Vec<(String, ModelDefinition)> {
        let mut result = Vec::new();
        for entry in self.providers.iter() {
            let provider_id = entry.id().to_string();
            for model in entry.models() {
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
mod tests {
    use super::*;
    use async_trait::async_trait;
    use autohands_protocols::error::ProviderError;
    use autohands_protocols::provider::{
        CompletionRequest, CompletionResponse, CompletionStream, ProviderCapabilities,
    };

    struct MockProvider {
        id: String,
        models: Vec<ModelDefinition>,
        capabilities: ProviderCapabilities,
    }

    impl MockProvider {
        fn new(id: &str) -> Self {
            Self {
                id: id.to_string(),
                models: vec![ModelDefinition::new("mock-model", "Mock Model")],
                capabilities: ProviderCapabilities::default(),
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        fn id(&self) -> &str {
            &self.id
        }

        fn models(&self) -> &[ModelDefinition] {
            &self.models
        }

        fn capabilities(&self) -> &ProviderCapabilities {
            &self.capabilities
        }

        async fn complete(
            &self,
            _request: CompletionRequest,
        ) -> Result<CompletionResponse, ProviderError> {
            unimplemented!()
        }

        async fn complete_stream(
            &self,
            _request: CompletionRequest,
        ) -> Result<CompletionStream, ProviderError> {
            unimplemented!()
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = ProviderRegistry::new();
        assert!(registry.list_ids().is_empty());
    }

    #[test]
    fn test_registry_default() {
        let registry = ProviderRegistry::default();
        assert!(registry.list_ids().is_empty());
    }

    #[test]
    fn test_register_provider() {
        let registry = ProviderRegistry::new();
        let provider = Arc::new(MockProvider::new("test-provider"));

        let result = registry.register(provider);
        assert!(result.is_ok());
        assert_eq!(registry.list_ids().len(), 1);
    }

    #[test]
    fn test_register_duplicate() {
        let registry = ProviderRegistry::new();
        let provider1 = Arc::new(MockProvider::new("test-provider"));
        let provider2 = Arc::new(MockProvider::new("test-provider"));

        registry.register(provider1).unwrap();
        let result = registry.register(provider2);
        assert!(result.is_err());
    }

    #[test]
    fn test_unregister_provider() {
        let registry = ProviderRegistry::new();
        let provider = Arc::new(MockProvider::new("test-provider"));

        registry.register(provider).unwrap();
        let result = registry.unregister("test-provider");
        assert!(result.is_ok());
        assert!(registry.list_ids().is_empty());
    }

    #[test]
    fn test_unregister_nonexistent() {
        let registry = ProviderRegistry::new();
        let result = registry.unregister("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_provider() {
        let registry = ProviderRegistry::new();
        let provider = Arc::new(MockProvider::new("test-provider"));

        registry.register(provider).unwrap();
        let retrieved = registry.get("test-provider");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), "test-provider");
    }

    #[test]
    fn test_get_nonexistent() {
        let registry = ProviderRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_list_models() {
        let registry = ProviderRegistry::new();
        registry.register(Arc::new(MockProvider::new("provider1"))).unwrap();
        registry.register(Arc::new(MockProvider::new("provider2"))).unwrap();

        let models = registry.list_models();
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn test_provider_registry_access_trait() {
        let registry = ProviderRegistry::new();
        let provider = Arc::new(MockProvider::new("test-provider"));

        registry.register_provider(provider).unwrap();
        registry.unregister_provider("test-provider").unwrap();
    }
}
