//! Provider retry and error handling.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;
use tracing::{debug, warn};

use autohands_protocols::error::ProviderError;
use autohands_protocols::provider::{CompletionRequest, CompletionResponse, CompletionStream};
use autohands_protocols::provider::LLMProvider;

/// Retry configuration.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Base delay between retries.
    pub base_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Exponential backoff multiplier.
    pub backoff_multiplier: f64,
    /// Add jitter to delays.
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for a given attempt.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay = self.base_delay.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32);
        let delay = delay.min(self.max_delay.as_millis() as f64);

        let delay_ms = if self.jitter {
            let jitter = rand_jitter(delay * 0.1);
            (delay + jitter) as u64
        } else {
            delay as u64
        };

        Duration::from_millis(delay_ms)
    }
}

/// Simple jitter using system time.
fn rand_jitter(max: f64) -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    (nanos as f64 / u32::MAX as f64) * max * 2.0 - max
}

/// Check if an error is retryable.
pub fn is_retryable(error: &ProviderError) -> bool {
    match error {
        ProviderError::RateLimited { .. } => true,
        ProviderError::Network(_) => true,
        ProviderError::Timeout(_) => true,
        ProviderError::ApiError { status, .. } => is_retryable_status(*status),
        _ => false,
    }
}

/// Check if HTTP status code is retryable.
fn is_retryable_status(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

/// Provider wrapper with retry capability.
pub struct RetryProvider {
    inner: Arc<dyn LLMProvider>,
    config: RetryConfig,
}

impl RetryProvider {
    /// Create a new retry provider.
    pub fn new(provider: Arc<dyn LLMProvider>, config: RetryConfig) -> Self {
        Self {
            inner: provider,
            config,
        }
    }

    /// Execute with retry.
    async fn with_retry<F, Fut, T>(&self, operation: F) -> Result<T, ProviderError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, ProviderError>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !is_retryable(&e) || attempt == self.config.max_retries {
                        return Err(e);
                    }

                    let delay = if let ProviderError::RateLimited { retry_after_seconds } = &e {
                        Duration::from_secs(*retry_after_seconds)
                    } else {
                        self.config.delay_for_attempt(attempt)
                    };

                    warn!(
                        "Provider error (attempt {}/{}): {}, retrying in {:?}",
                        attempt + 1,
                        self.config.max_retries + 1,
                        e,
                        delay
                    );

                    last_error = Some(e);
                    sleep(delay).await;
                }
            }
        }

        Err(last_error.unwrap_or(ProviderError::Network("Unknown error".to_string())))
    }

    /// Complete with retry.
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
        debug!("Completing with retry: model={}", request.model);
        self.with_retry(|| {
            let req = request.clone();
            let provider = self.inner.clone();
            async move { provider.complete(req).await }
        })
        .await
    }

    /// Stream complete with retry (only retries initial connection).
    pub async fn complete_stream(&self, request: CompletionRequest) -> Result<CompletionStream, ProviderError> {
        debug!("Stream completing with retry: model={}", request.model);
        self.with_retry(|| {
            let req = request.clone();
            let provider = self.inner.clone();
            async move { provider.complete_stream(req).await }
        })
        .await
    }

    /// Get inner provider.
    pub fn inner(&self) -> &Arc<dyn LLMProvider> {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use autohands_protocols::provider::{ModelDefinition, ProviderCapabilities};
    use autohands_protocols::types::{Message, StopReason, Usage};
    use std::sync::atomic::{AtomicU32, Ordering};

    struct MockProvider {
        fail_count: AtomicU32,
        fail_times: u32,
    }

    impl MockProvider {
        fn new(fail_times: u32) -> Self {
            Self {
                fail_count: AtomicU32::new(0),
                fail_times,
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        fn id(&self) -> &str {
            "mock"
        }

        fn models(&self) -> &[ModelDefinition] {
            &[]
        }

        fn capabilities(&self) -> &ProviderCapabilities {
            &ProviderCapabilities {
                streaming: true,
                tool_calling: true,
                vision: false,
                json_mode: false,
                prompt_caching: false,
                batching: false,
                max_concurrent: None,
            }
        }

        async fn complete(&self, _: CompletionRequest) -> Result<CompletionResponse, ProviderError> {
            let count = self.fail_count.fetch_add(1, Ordering::SeqCst);
            if count < self.fail_times {
                Err(ProviderError::Network("Connection failed".to_string()))
            } else {
                Ok(CompletionResponse {
                    id: "test".to_string(),
                    model: "mock".to_string(),
                    message: Message::assistant("Success"),
                    stop_reason: StopReason::EndTurn,
                    usage: Usage::default(),
                    metadata: Default::default(),
                })
            }
        }

        async fn complete_stream(&self, _: CompletionRequest) -> Result<CompletionStream, ProviderError> {
            Err(ProviderError::Network("Not implemented".to_string()))
        }
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay, Duration::from_millis(500));
    }

    #[test]
    fn test_delay_calculation() {
        let config = RetryConfig {
            base_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            jitter: false,
            ..Default::default()
        };

        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(400));
    }

    #[test]
    fn test_is_retryable() {
        assert!(is_retryable(&ProviderError::RateLimited {
            retry_after_seconds: 60
        }));
        assert!(is_retryable(&ProviderError::Network("error".to_string())));
        assert!(is_retryable(&ProviderError::Timeout(30)));
        assert!(!is_retryable(&ProviderError::AuthenticationFailed(
            "bad key".to_string()
        )));
        assert!(!is_retryable(&ProviderError::InvalidRequest(
            "bad request".to_string()
        )));
    }

    #[test]
    fn test_is_retryable_status() {
        assert!(is_retryable_status(429));
        assert!(is_retryable_status(500));
        assert!(is_retryable_status(502));
        assert!(is_retryable_status(503));
        assert!(is_retryable_status(504));
        assert!(!is_retryable_status(400));
        assert!(!is_retryable_status(401));
        assert!(!is_retryable_status(404));
    }

    #[tokio::test]
    async fn test_retry_success_on_first_try() {
        let provider = Arc::new(MockProvider::new(0));
        let retry = RetryProvider::new(provider, RetryConfig::default());

        let request = CompletionRequest::new("mock", vec![]);
        let result = retry.complete(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let provider = Arc::new(MockProvider::new(2));
        let config = RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(1),
            jitter: false,
            ..Default::default()
        };
        let retry = RetryProvider::new(provider, config);

        let request = CompletionRequest::new("mock", vec![]);
        let result = retry.complete(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let provider = Arc::new(MockProvider::new(10));
        let config = RetryConfig {
            max_retries: 2,
            base_delay: Duration::from_millis(1),
            jitter: false,
            ..Default::default()
        };
        let retry = RetryProvider::new(provider, config);

        let request = CompletionRequest::new("mock", vec![]);
        let result = retry.complete(request).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_retry_config_clone() {
        let config = RetryConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.max_retries, config.max_retries);
        assert_eq!(cloned.base_delay, config.base_delay);
        assert_eq!(cloned.max_delay, config.max_delay);
    }

    #[test]
    fn test_retry_config_debug() {
        let config = RetryConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("RetryConfig"));
        assert!(debug.contains("max_retries"));
    }

    #[test]
    fn test_delay_calculation_with_max() {
        let config = RetryConfig {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(500),
            backoff_multiplier: 2.0,
            jitter: false,
            ..Default::default()
        };

        // 100 * 2^3 = 800, but max is 500
        let delay = config.delay_for_attempt(3);
        assert_eq!(delay, Duration::from_millis(500));
    }

    #[test]
    fn test_delay_calculation_with_jitter() {
        let config = RetryConfig {
            base_delay: Duration::from_millis(100),
            backoff_multiplier: 1.0,
            jitter: true,
            ..Default::default()
        };

        // With jitter, delays should vary slightly
        let delay1 = config.delay_for_attempt(0);
        let delay2 = config.delay_for_attempt(0);
        // Both should be approximately 100ms
        assert!(delay1.as_millis() >= 80 && delay1.as_millis() <= 120);
        assert!(delay2.as_millis() >= 80 && delay2.as_millis() <= 120);
    }

    #[test]
    fn test_is_retryable_api_error_429() {
        assert!(is_retryable(&ProviderError::ApiError {
            status: 429,
            message: "Too many requests".to_string()
        }));
    }

    #[test]
    fn test_is_retryable_api_error_500() {
        assert!(is_retryable(&ProviderError::ApiError {
            status: 500,
            message: "Internal server error".to_string()
        }));
    }

    #[test]
    fn test_is_retryable_api_error_400() {
        assert!(!is_retryable(&ProviderError::ApiError {
            status: 400,
            message: "Bad request".to_string()
        }));
    }

    #[test]
    fn test_is_retryable_content_filtered() {
        assert!(!is_retryable(&ProviderError::ContentFiltered(
            "Inappropriate content".to_string()
        )));
    }

    #[test]
    fn test_is_retryable_context_length() {
        assert!(!is_retryable(&ProviderError::ContextLengthExceeded {
            used: 150000,
            max: 128000
        }));
    }

    #[test]
    fn test_retry_provider_inner() {
        let provider = Arc::new(MockProvider::new(0));
        let retry = RetryProvider::new(provider.clone(), RetryConfig::default());
        assert_eq!(retry.inner().id(), "mock");
    }

    #[tokio::test]
    async fn test_retry_provider_complete_stream() {
        let provider = Arc::new(MockProvider::new(0));
        let retry = RetryProvider::new(provider, RetryConfig::default());

        let request = CompletionRequest::new("mock", vec![]);
        let result = retry.complete_stream(request).await;
        // MockProvider always fails on stream
        assert!(result.is_err());
    }

    #[test]
    fn test_is_retryable_model_not_found() {
        assert!(!is_retryable(&ProviderError::ModelNotFound(
            "gpt-5".to_string()
        )));
    }

    #[test]
    fn test_is_retryable_not_found() {
        assert!(!is_retryable(&ProviderError::NotFound(
            "provider".to_string()
        )));
    }

    #[test]
    fn test_is_retryable_stream_error() {
        assert!(!is_retryable(&ProviderError::StreamError(
            "Stream closed".to_string()
        )));
    }
}
