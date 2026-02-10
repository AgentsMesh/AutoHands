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
#[path = "retry_tests.rs"]
mod tests;
