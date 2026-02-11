
    use super::*;
    use crate::webhook::types::WebhookRegistration;

    #[test]
    fn test_registry_new_is_empty() {
        let registry = WebhookRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_register_and_get() {
        let registry = WebhookRegistry::new();
        let reg = WebhookRegistration::new("github")
            .with_description("GitHub events")
            .with_agent("gh-handler");

        registry.register(reg);

        let result = registry.get("github");
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.id, "github");
        assert_eq!(r.description, Some("GitHub events".to_string()));
        assert_eq!(r.agent, Some("gh-handler".to_string()));
    }

    #[test]
    fn test_get_nonexistent() {
        let registry = WebhookRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_register_overwrites() {
        let registry = WebhookRegistry::new();
        registry.register(WebhookRegistration::new("hook1").with_description("v1"));
        registry.register(WebhookRegistration::new("hook1").with_description("v2"));

        assert_eq!(registry.len(), 1);
        let r = registry.get("hook1").unwrap();
        assert_eq!(r.description, Some("v2".to_string()));
    }

    #[test]
    fn test_remove() {
        let registry = WebhookRegistry::new();
        registry.register(WebhookRegistration::new("to-delete"));

        let removed = registry.remove("to-delete");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, "to-delete");
        assert!(registry.is_empty());
    }

    #[test]
    fn test_remove_nonexistent() {
        let registry = WebhookRegistry::new();
        assert!(registry.remove("nope").is_none());
    }

    #[test]
    fn test_list() {
        let registry = WebhookRegistry::new();
        registry.register(WebhookRegistration::new("a"));
        registry.register(WebhookRegistration::new("b"));
        registry.register(WebhookRegistration::new("c"));

        let list = registry.list();
        assert_eq!(list.len(), 3);

        let mut ids: Vec<String> = list.into_iter().map(|r| r.id).collect();
        ids.sort();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_contains() {
        let registry = WebhookRegistry::new();
        registry.register(WebhookRegistration::new("exists"));

        assert!(registry.contains("exists"));
        assert!(!registry.contains("missing"));
    }

    #[test]
    fn test_default() {
        let registry = WebhookRegistry::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_register_with_secret() {
        let registry = WebhookRegistry::new();
        let reg = WebhookRegistration::new("github")
            .with_secret("my-secret-key");
        registry.register(reg);

        let r = registry.get("github").unwrap();
        assert_eq!(r.secret, Some("my-secret-key".to_string()));
    }
