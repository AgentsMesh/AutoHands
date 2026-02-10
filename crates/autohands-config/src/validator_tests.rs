
    use super::*;

    #[test]
    fn test_validate_default_config() {
        let config = Config::default();
        let result = ConfigValidator::validate(&config).unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_invalid_port() {
        let mut config = Config::default();
        config.server.port = 0;

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.path == "server.port"));
    }

    #[test]
    fn test_validate_invalid_max_turns() {
        let mut config = Config::default();
        config.agent.max_turns = 0;

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.path == "agent.max_turns"));
    }

    #[test]
    fn test_validate_high_max_turns_warning() {
        let mut config = Config::default();
        config.agent.max_turns = 5000;

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(result.is_valid());
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_validate_invalid_base_url() {
        use crate::schema::ProviderConfig;

        let mut config = Config::default();
        config.providers.insert(
            "test".to_string(),
            ProviderConfig {
                api_key: Some("key".to_string()),
                base_url: Some("invalid-url".to_string()),
                default_model: None,
                extra: Default::default(),
            },
        );

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validate_extension_conflict() {
        let mut config = Config::default();
        config.extensions.enabled.push("ext1".to_string());
        config.extensions.disabled.push("ext1".to_string());

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validation_result_default() {
        let result = ValidationResult::default();
        assert!(result.is_valid());
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validation_error_new() {
        let err = ValidationError::new("server.port", "must be positive");
        assert_eq!(err.path, "server.port");
        assert_eq!(err.message, "must be positive");
    }

    #[test]
    fn test_validation_warning_new() {
        let warn = ValidationWarning::new("agent.max_turns", "value is high");
        assert_eq!(warn.path, "agent.max_turns");
        assert_eq!(warn.message, "value is high");
    }

    #[test]
    fn test_validation_result_add_error() {
        let mut result = ValidationResult::default();
        result.add_error(ValidationError::new("test", "error"));
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_validation_result_add_warning() {
        let mut result = ValidationResult::default();
        result.add_warning(ValidationWarning::new("test", "warning"));
        assert!(result.is_valid()); // Warnings don't make it invalid
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_validation_error_debug() {
        let err = ValidationError::new("path", "message");
        let debug = format!("{:?}", err);
        assert!(debug.contains("ValidationError"));
    }

    #[test]
    fn test_validation_warning_debug() {
        let warn = ValidationWarning::new("path", "message");
        let debug = format!("{:?}", warn);
        assert!(debug.contains("ValidationWarning"));
    }

    #[test]
    fn test_validation_result_debug() {
        let result = ValidationResult::default();
        let debug = format!("{:?}", result);
        assert!(debug.contains("ValidationResult"));
    }

    #[test]
    fn test_validate_empty_host() {
        let mut config = Config::default();
        config.server.host = String::new();

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.path == "server.host"));
    }

    #[test]
    fn test_validate_empty_default_agent() {
        let mut config = Config::default();
        config.agent.default = String::new();

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.path == "agent.default"));
    }

    #[test]
    fn test_validate_zero_timeout() {
        let mut config = Config::default();
        config.agent.timeout_seconds = 0;

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.path == "agent.timeout_seconds"));
    }

    #[test]
    fn test_validate_provider_without_api_key() {
        use crate::schema::ProviderConfig;

        let mut config = Config::default();
        config.providers.insert(
            "openai".to_string(),
            ProviderConfig {
                api_key: None,
                base_url: None,
                default_model: None,
                extra: Default::default(),
            },
        );

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(result.is_valid()); // It's just a warning
        assert!(result.warnings.iter().any(|w| w.path.contains("api_key")));
    }

    #[test]
    fn test_validate_provider_valid_base_url() {
        use crate::schema::ProviderConfig;

        let mut config = Config::default();
        config.providers.insert(
            "openai".to_string(),
            ProviderConfig {
                api_key: Some("key".to_string()),
                base_url: Some("https://api.openai.com".to_string()),
                default_model: None,
                extra: Default::default(),
            },
        );

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_unknown_memory_backend() {
        let mut config = Config::default();
        config.memory.backend = "unknown".to_string();

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(result.is_valid()); // It's a warning
        assert!(result.warnings.iter().any(|w| w.path == "memory.backend"));
    }

    #[test]
    fn test_validate_sqlite_without_path() {
        let mut config = Config::default();
        config.memory.backend = "sqlite".to_string();
        config.memory.path = None;

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(result.is_valid());
        assert!(result.warnings.iter().any(|w| w.path == "memory.path"));
    }

    #[test]
    fn test_validate_nonexistent_extension_path() {
        let mut config = Config::default();
        config.extensions.paths.push("/nonexistent/path".into());

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(result.is_valid()); // It's a warning
        assert!(result.warnings.iter().any(|w| w.path == "extensions.paths"));
    }

    #[test]
    fn test_validate_multiple_errors() {
        let mut config = Config::default();
        config.server.port = 0;
        config.server.host = String::new();
        config.agent.max_turns = 0;

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(!result.is_valid());
        assert!(result.errors.len() >= 3);
    }

    #[test]
    fn test_validate_http_base_url() {
        use crate::schema::ProviderConfig;

        let mut config = Config::default();
        config.providers.insert(
            "local".to_string(),
            ProviderConfig {
                api_key: Some("key".to_string()),
                base_url: Some("http://localhost:8000".to_string()),
                default_model: None,
                extra: Default::default(),
            },
        );

        let result = ConfigValidator::validate(&config).unwrap();
        assert!(result.is_valid());
    }
