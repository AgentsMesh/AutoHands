    use super::*;

    #[test]
    fn test_extension_manifest() {
        let ext = ArkExtension::new();
        assert_eq!(ext.manifest().id, "provider-ark");
        assert!(ext.manifest().provides.providers.contains(&"ark".to_string()));
    }

    #[test]
    fn test_extension_with_api_key() {
        let ext = ArkExtension::new().with_api_key("test-key");
        assert_eq!(ext.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_extension_with_api_url() {
        let ext = ArkExtension::new().with_api_url("https://custom.api");
        assert_eq!(ext.api_url, Some("https://custom.api".to_string()));
    }

    #[test]
    fn test_extension_with_custom_model() {
        let model = ModelDefinition {
            id: "ep-test-endpoint".to_string(),
            name: "Test Endpoint".to_string(),
            description: None,
            context_length: 32768,
            max_output_tokens: 4096,
            supports_vision: false,
            supports_tools: true,
            supports_system: true,
            input_cost_per_million: None,
            output_cost_per_million: None,
            metadata: Default::default(),
        };
        let ext = ArkExtension::new().with_custom_model(model);
        assert_eq!(ext.custom_models.len(), 1);
        assert_eq!(ext.custom_models[0].id, "ep-test-endpoint");
    }

    #[test]
    fn test_default_extension() {
        let ext = ArkExtension::default();
        assert!(ext.api_key.is_none());
        assert!(ext.api_url.is_none());
    }

    #[test]
    fn test_extension_manifest_description() {
        let ext = ArkExtension::new();
        assert!(!ext.manifest().description.is_empty());
        assert!(ext.manifest().description.contains("Ark"));
    }

    #[test]
    fn test_extension_manifest_version() {
        let ext = ArkExtension::new();
        let version = &ext.manifest().version;
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 1);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_extension_builder_chain() {
        let ext = ArkExtension::new()
            .with_api_key("key123")
            .with_api_url("https://api.example.com");
        assert_eq!(ext.api_key, Some("key123".to_string()));
        assert_eq!(ext.api_url, Some("https://api.example.com".to_string()));
    }

    #[test]
    fn test_extension_as_any() {
        let ext = ArkExtension::new();
        let _any: &dyn Any = ext.as_any();
    }

    #[test]
    fn test_extension_as_any_mut() {
        let mut ext = ArkExtension::new();
        let _any: &mut dyn Any = ext.as_any_mut();
    }

    #[test]
    fn test_manifest_provides() {
        let ext = ArkExtension::new();
        let provides = &ext.manifest().provides;
        assert!(provides.tools.is_empty());
        assert!(!provides.providers.is_empty());
        assert!(provides.memory_backends.is_empty());
    }
