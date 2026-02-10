    use super::*;

    // Simple test struct
    struct TestItem {
        id: String,
    }

    impl TestItem {
        fn new(id: &str) -> Self {
            Self { id: id.to_string() }
        }
    }

    impl Registerable for TestItem {
        fn registry_id(&self) -> &str {
            &self.id
        }
    }

    #[test]
    fn test_base_registry_new() {
        let registry: BaseRegistry<TestItem> = BaseRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_base_registry_default() {
        let registry: BaseRegistry<TestItem> = BaseRegistry::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_register() {
        let registry: BaseRegistry<TestItem> = BaseRegistry::new();
        let item = Arc::new(TestItem::new("test-item"));

        let result = registry.register(item);
        assert!(result.is_ok());
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_register_duplicate() {
        let registry: BaseRegistry<TestItem> = BaseRegistry::new();
        let item1 = Arc::new(TestItem::new("test-item"));
        let item2 = Arc::new(TestItem::new("test-item"));

        registry.register(item1).unwrap();
        let result = registry.register(item2);
        assert!(result.is_err());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_unregister() {
        let registry: BaseRegistry<TestItem> = BaseRegistry::new();
        let item = Arc::new(TestItem::new("test-item"));

        registry.register(item).unwrap();
        let result = registry.unregister("test-item");
        assert!(result.is_ok());
        assert!(registry.is_empty());
    }

    #[test]
    fn test_unregister_nonexistent() {
        let registry: BaseRegistry<TestItem> = BaseRegistry::new();
        let result = registry.unregister("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get() {
        let registry: BaseRegistry<TestItem> = BaseRegistry::new();
        let item = Arc::new(TestItem::new("test-item"));

        registry.register(item).unwrap();
        let retrieved = registry.get("test-item");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().registry_id(), "test-item");
    }

    #[test]
    fn test_get_nonexistent() {
        let registry: BaseRegistry<TestItem> = BaseRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_contains() {
        let registry: BaseRegistry<TestItem> = BaseRegistry::new();
        let item = Arc::new(TestItem::new("test-item"));

        assert!(!registry.contains("test-item"));
        registry.register(item).unwrap();
        assert!(registry.contains("test-item"));
    }

    #[test]
    fn test_list_ids() {
        let registry: BaseRegistry<TestItem> = BaseRegistry::new();
        registry.register(Arc::new(TestItem::new("item1"))).unwrap();
        registry.register(Arc::new(TestItem::new("item2"))).unwrap();

        let ids = registry.list_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"item1".to_string()));
        assert!(ids.contains(&"item2".to_string()));
    }

    #[test]
    fn test_iter() {
        let registry: BaseRegistry<TestItem> = BaseRegistry::new();
        registry.register(Arc::new(TestItem::new("item1"))).unwrap();
        registry.register(Arc::new(TestItem::new("item2"))).unwrap();

        let items: Vec<_> = registry.iter().collect();
        assert_eq!(items.len(), 2);
    }
