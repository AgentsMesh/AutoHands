//! Configuration validation.

use crate::error::ConfigError;
use crate::schema::Config;

/// Validation result.
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }
}

/// A validation error.
#[derive(Debug)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
}

impl ValidationError {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

/// A validation warning.
#[derive(Debug)]
pub struct ValidationWarning {
    pub path: String,
    pub message: String,
}

impl ValidationWarning {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

/// Configuration validator.
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate the configuration.
    pub fn validate(config: &Config) -> Result<ValidationResult, ConfigError> {
        let mut result = ValidationResult::default();

        // Validate server config
        Self::validate_server(config, &mut result);

        // Validate agent config
        Self::validate_agent(config, &mut result);

        // Validate providers
        Self::validate_providers(config, &mut result);

        // Validate memory config
        Self::validate_memory(config, &mut result);

        // Validate extensions
        Self::validate_extensions(config, &mut result);

        Ok(result)
    }

    fn validate_server(config: &Config, result: &mut ValidationResult) {
        // Validate port range
        if config.server.port == 0 {
            result.add_error(ValidationError::new(
                "server.port",
                "Port cannot be 0",
            ));
        }

        // Validate host
        if config.server.host.is_empty() {
            result.add_error(ValidationError::new(
                "server.host",
                "Host cannot be empty",
            ));
        }
    }

    fn validate_agent(config: &Config, result: &mut ValidationResult) {
        // Validate max_turns
        if config.agent.max_turns == 0 {
            result.add_error(ValidationError::new(
                "agent.max_turns",
                "max_turns must be greater than 0",
            ));
        }

        if config.agent.max_turns > 1000 {
            result.add_warning(ValidationWarning::new(
                "agent.max_turns",
                "max_turns is very high (>1000), this may lead to long-running agents",
            ));
        }

        // Validate timeout
        if config.agent.timeout_seconds == 0 {
            result.add_error(ValidationError::new(
                "agent.timeout_seconds",
                "timeout_seconds must be greater than 0",
            ));
        }

        // Validate default agent
        if config.agent.default.is_empty() {
            result.add_error(ValidationError::new(
                "agent.default",
                "Default agent cannot be empty",
            ));
        }
    }

    fn validate_providers(config: &Config, result: &mut ValidationResult) {
        for (name, provider) in &config.providers {
            // Check for API key
            if provider.api_key.is_none() {
                result.add_warning(ValidationWarning::new(
                    format!("providers.{}.api_key", name),
                    "API key is not set, may need to be set via environment variable",
                ));
            }

            // Validate base_url if set
            if let Some(ref url) = provider.base_url {
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    result.add_error(ValidationError::new(
                        format!("providers.{}.base_url", name),
                        "base_url must start with http:// or https://",
                    ));
                }
            }
        }
    }

    fn validate_memory(config: &Config, result: &mut ValidationResult) {
        // Validate backend
        let valid_backends = ["sqlite", "memory", "vector"];
        if !valid_backends.contains(&config.memory.backend.as_str()) {
            result.add_warning(ValidationWarning::new(
                "memory.backend",
                format!(
                    "Unknown memory backend '{}', valid values: {:?}",
                    config.memory.backend, valid_backends
                ),
            ));
        }

        // For sqlite, check path
        if config.memory.backend == "sqlite" && config.memory.path.is_none() {
            result.add_warning(ValidationWarning::new(
                "memory.path",
                "SQLite backend path not set, will use default location",
            ));
        }
    }

    fn validate_extensions(config: &Config, result: &mut ValidationResult) {
        // Check for conflicts between enabled and disabled
        for ext in &config.extensions.enabled {
            if config.extensions.disabled.contains(ext) {
                result.add_error(ValidationError::new(
                    "extensions",
                    format!("Extension '{}' is both enabled and disabled", ext),
                ));
            }
        }

        // Check extension paths exist
        for path in &config.extensions.paths {
            if !path.exists() {
                result.add_warning(ValidationWarning::new(
                    "extensions.paths",
                    format!("Extension path does not exist: {:?}", path),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
}
