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
        // Note: max_turns and timeout_seconds are no longer enforced at the agent loop level.
        // Agents run indefinitely until they complete or are explicitly aborted, supporting
        // 7×24 autonomous long-running tasks.

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
        let valid_backends = ["sqlite", "memory", "vector", "markdown", "hybrid"];
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
#[path = "validator_tests.rs"]
mod tests;
