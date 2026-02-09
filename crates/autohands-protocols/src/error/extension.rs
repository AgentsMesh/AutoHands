//! Extension-related errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtensionError {
    #[error("Extension not found: {0}")]
    NotFound(String),

    #[error("Extension already registered: {0}")]
    AlreadyRegistered(String),

    #[error("Extension initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Extension dependency not satisfied: {extension} requires {dependency}")]
    DependencyNotSatisfied { extension: String, dependency: String },

    #[error("Extension shutdown failed: {0}")]
    ShutdownFailed(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("{0}")]
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = ExtensionError::NotFound("my-extension".to_string());
        let display = err.to_string();
        assert!(display.contains("not found"));
        assert!(display.contains("my-extension"));
    }

    #[test]
    fn test_already_registered_error() {
        let err = ExtensionError::AlreadyRegistered("ext".to_string());
        let display = err.to_string();
        assert!(display.contains("already registered"));
        assert!(display.contains("ext"));
    }

    #[test]
    fn test_initialization_failed_error() {
        let err = ExtensionError::InitializationFailed("connection refused".to_string());
        let display = err.to_string();
        assert!(display.contains("initialization failed"));
        assert!(display.contains("connection refused"));
    }

    #[test]
    fn test_dependency_not_satisfied_error() {
        let err = ExtensionError::DependencyNotSatisfied {
            extension: "tools-browser".to_string(),
            dependency: "chromium".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("tools-browser"));
        assert!(display.contains("chromium"));
        assert!(display.contains("requires"));
    }

    #[test]
    fn test_shutdown_failed_error() {
        let err = ExtensionError::ShutdownFailed("timeout".to_string());
        let display = err.to_string();
        assert!(display.contains("shutdown failed"));
        assert!(display.contains("timeout"));
    }

    #[test]
    fn test_timeout_error() {
        let err = ExtensionError::Timeout;
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_channel_closed_error() {
        let err = ExtensionError::ChannelClosed;
        assert!(err.to_string().contains("closed"));
    }

    #[test]
    fn test_custom_error() {
        let err = ExtensionError::Custom("custom error message".to_string());
        assert_eq!(err.to_string(), "custom error message");
    }

    #[test]
    fn test_error_debug() {
        let err = ExtensionError::NotFound("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
    }

    #[test]
    fn test_all_error_variants() {
        let errors: Vec<ExtensionError> = vec![
            ExtensionError::NotFound("a".to_string()),
            ExtensionError::AlreadyRegistered("b".to_string()),
            ExtensionError::InitializationFailed("c".to_string()),
            ExtensionError::DependencyNotSatisfied {
                extension: "d".to_string(),
                dependency: "e".to_string(),
            },
            ExtensionError::ShutdownFailed("f".to_string()),
            ExtensionError::Timeout,
            ExtensionError::ChannelClosed,
            ExtensionError::Custom("g".to_string()),
        ];

        for err in errors {
            let display = err.to_string();
            assert!(!display.is_empty());
        }
    }
}
