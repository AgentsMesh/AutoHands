//! LLM Provider errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Provider not found: {0}")]
    NotFound(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Rate limited: retry after {retry_after_seconds} seconds")]
    RateLimited { retry_after_seconds: u64 },

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Context length exceeded: {used} tokens used, {max} tokens allowed")]
    ContextLengthExceeded { used: usize, max: usize },

    #[error("Content filtered: {0}")]
    ContentFiltered(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_error_not_found() {
        let err = ProviderError::NotFound("test_provider".to_string());
        assert!(err.to_string().contains("Provider not found"));
    }

    #[test]
    fn test_provider_error_model_not_found() {
        let err = ProviderError::ModelNotFound("gpt-5".to_string());
        assert!(err.to_string().contains("Model not found"));
    }

    #[test]
    fn test_provider_error_api_error() {
        let err = ProviderError::ApiError {
            status: 500,
            message: "Internal Server Error".to_string(),
        };
        assert!(err.to_string().contains("500"));
        assert!(err.to_string().contains("Internal Server Error"));
    }

    #[test]
    fn test_provider_error_rate_limited() {
        let err = ProviderError::RateLimited {
            retry_after_seconds: 60,
        };
        assert!(err.to_string().contains("Rate limited"));
        assert!(err.to_string().contains("60"));
    }

    #[test]
    fn test_provider_error_auth_failed() {
        let err = ProviderError::AuthenticationFailed("Invalid API key".to_string());
        assert!(err.to_string().contains("Authentication failed"));
    }

    #[test]
    fn test_provider_error_invalid_request() {
        let err = ProviderError::InvalidRequest("Missing model".to_string());
        assert!(err.to_string().contains("Invalid request"));
    }

    #[test]
    fn test_provider_error_context_length() {
        let err = ProviderError::ContextLengthExceeded {
            used: 150000,
            max: 128000,
        };
        assert!(err.to_string().contains("150000"));
        assert!(err.to_string().contains("128000"));
    }

    #[test]
    fn test_provider_error_content_filtered() {
        let err = ProviderError::ContentFiltered("Inappropriate content".to_string());
        assert!(err.to_string().contains("Content filtered"));
    }

    #[test]
    fn test_provider_error_network() {
        let err = ProviderError::Network("Connection refused".to_string());
        assert!(err.to_string().contains("Network error"));
    }

    #[test]
    fn test_provider_error_stream() {
        let err = ProviderError::StreamError("Stream closed unexpectedly".to_string());
        assert!(err.to_string().contains("Stream error"));
    }

    #[test]
    fn test_provider_error_timeout() {
        let err = ProviderError::Timeout(30);
        assert!(err.to_string().contains("Timeout"));
        assert!(err.to_string().contains("30"));
    }

    #[test]
    fn test_provider_error_debug() {
        let err = ProviderError::NotFound("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotFound"));
    }
}
