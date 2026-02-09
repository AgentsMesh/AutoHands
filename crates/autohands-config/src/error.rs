//! Configuration errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(String),

    #[error("Invalid config format: {0}")]
    InvalidFormat(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid value for {field}: {message}")]
    InvalidValue { field: String, message: String },

    #[error("Environment variable not set: {0}")]
    EnvVarNotSet(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = ConfigError::NotFound("config.toml".to_string());
        assert!(err.to_string().contains("config.toml"));
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_invalid_format_error() {
        let err = ConfigError::InvalidFormat("expected table".to_string());
        assert!(err.to_string().contains("expected table"));
        assert!(err.to_string().contains("Invalid"));
    }

    #[test]
    fn test_missing_field_error() {
        let err = ConfigError::MissingField("api_key".to_string());
        assert!(err.to_string().contains("api_key"));
        assert!(err.to_string().contains("Missing"));
    }

    #[test]
    fn test_invalid_value_error() {
        let err = ConfigError::InvalidValue {
            field: "port".to_string(),
            message: "must be positive".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("port"));
        assert!(display.contains("must be positive"));
    }

    #[test]
    fn test_env_var_not_set_error() {
        let err = ConfigError::EnvVarNotSet("API_KEY".to_string());
        assert!(err.to_string().contains("API_KEY"));
        assert!(err.to_string().contains("not set"));
    }

    #[test]
    fn test_io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = ConfigError::from(io_err);
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_error_debug() {
        let err = ConfigError::NotFound("test.toml".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
    }

    #[test]
    fn test_all_error_variants_display() {
        let errors: Vec<ConfigError> = vec![
            ConfigError::NotFound("path".to_string()),
            ConfigError::InvalidFormat("format".to_string()),
            ConfigError::MissingField("field".to_string()),
            ConfigError::InvalidValue {
                field: "f".to_string(),
                message: "m".to_string(),
            },
            ConfigError::EnvVarNotSet("VAR".to_string()),
        ];

        for err in errors {
            let display = err.to_string();
            assert!(!display.is_empty());
        }
    }
}
