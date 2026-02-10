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
#[path = "provider_tests.rs"]
mod tests;
