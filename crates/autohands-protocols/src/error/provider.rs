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

impl ProviderError {
    /// 基于 HTTP 状态码和错误消息创建语义化错误。
    /// 各 Provider 应先解析平台特有的错误 JSON 提取 message，再调用此方法。
    pub fn from_api_response(status: u16, message: String) -> Self {
        match status {
            401 => ProviderError::AuthenticationFailed(message),
            429 => ProviderError::RateLimited {
                retry_after_seconds: 0,
            },
            _ => {
                let lower = message.to_lowercase();
                if lower.contains("context length")
                    || (lower.contains("token")
                        && (lower.contains("exceed") || lower.contains("limit")))
                    || lower.contains("maximum context")
                    || lower.contains("too many tokens")
                {
                    ProviderError::ContextLengthExceeded { used: 0, max: 0 }
                } else if lower.contains("content filter")
                    || lower.contains("safety")
                    || lower.contains("blocked")
                {
                    ProviderError::ContentFiltered(message)
                } else {
                    ProviderError::ApiError { status, message }
                }
            }
        }
    }

    /// 判断此错误是否可以通过重试或上下文压缩恢复。
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ProviderError::RateLimited { .. }
                | ProviderError::ContextLengthExceeded { .. }
                | ProviderError::Network(_)
                | ProviderError::Timeout(_)
        )
    }

    /// 判断此错误是否因上下文过长。
    pub fn is_context_length_error(&self) -> bool {
        matches!(self, ProviderError::ContextLengthExceeded { .. })
    }
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

    #[test]
    fn test_from_api_response_auth_failed() {
        let err = ProviderError::from_api_response(401, "Invalid API key".to_string());
        assert!(matches!(err, ProviderError::AuthenticationFailed(_)));
    }

    #[test]
    fn test_from_api_response_rate_limited() {
        let err = ProviderError::from_api_response(429, "Rate limit exceeded".to_string());
        assert!(matches!(err, ProviderError::RateLimited { .. }));
    }

    #[test]
    fn test_from_api_response_token_exceed() {
        let err = ProviderError::from_api_response(
            400,
            "Total tokens exceed the maximum limit".to_string(),
        );
        assert!(matches!(err, ProviderError::ContextLengthExceeded { .. }));
    }

    #[test]
    fn test_from_api_response_context_length() {
        let err = ProviderError::from_api_response(
            400,
            "This model's maximum context length is 128000".to_string(),
        );
        assert!(matches!(err, ProviderError::ContextLengthExceeded { .. }));
    }

    #[test]
    fn test_from_api_response_too_many_tokens() {
        let err = ProviderError::from_api_response(
            400,
            "Request has too many tokens".to_string(),
        );
        assert!(matches!(err, ProviderError::ContextLengthExceeded { .. }));
    }

    #[test]
    fn test_from_api_response_content_filtered() {
        let err = ProviderError::from_api_response(
            400,
            "Content filter triggered".to_string(),
        );
        assert!(matches!(err, ProviderError::ContentFiltered(_)));
    }

    #[test]
    fn test_from_api_response_safety_blocked() {
        let err = ProviderError::from_api_response(
            400,
            "Response blocked by safety settings".to_string(),
        );
        assert!(matches!(err, ProviderError::ContentFiltered(_)));
    }

    #[test]
    fn test_from_api_response_generic_error() {
        let err = ProviderError::from_api_response(
            500,
            "Internal Server Error".to_string(),
        );
        assert!(matches!(err, ProviderError::ApiError { status: 500, .. }));
    }

    #[test]
    fn test_is_retryable() {
        assert!(ProviderError::RateLimited { retry_after_seconds: 5 }.is_retryable());
        assert!(ProviderError::ContextLengthExceeded { used: 0, max: 0 }.is_retryable());
        assert!(ProviderError::Network("err".to_string()).is_retryable());
        assert!(ProviderError::Timeout(30).is_retryable());
        assert!(!ProviderError::AuthenticationFailed("err".to_string()).is_retryable());
        assert!(!ProviderError::ApiError { status: 500, message: "err".to_string() }.is_retryable());
    }

    #[test]
    fn test_is_context_length_error() {
        assert!(ProviderError::ContextLengthExceeded { used: 0, max: 0 }.is_context_length_error());
        assert!(!ProviderError::RateLimited { retry_after_seconds: 0 }.is_context_length_error());
        assert!(!ProviderError::Network("err".to_string()).is_context_length_error());
    }
}
