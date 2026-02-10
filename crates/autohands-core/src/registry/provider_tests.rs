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
