//! Health check integration with RunLoop.
//!
//! Provides a HealthCheck Observer that runs at BeforeWaiting phase.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, error, warn};

use crate::mode::RunLoopPhase;
use crate::observer::RunLoopObserver;
use crate::RunLoop;

/// Health status.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Whether the component is healthy.
    pub is_healthy: bool,
    /// Status message.
    pub message: String,
}

impl HealthStatus {
    /// Create a healthy status.
    pub fn healthy() -> Self {
        Self {
            is_healthy: true,
            message: "OK".to_string(),
        }
    }

    /// Create an unhealthy status.
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            is_healthy: false,
            message: message.into(),
        }
    }

    /// Create a degraded status (healthy but with warning).
    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            is_healthy: true,
            message: message.into(),
        }
    }
}

/// Health checkable trait.
#[async_trait]
pub trait HealthCheckable: Send + Sync {
    /// Component name.
    fn name(&self) -> &str;

    /// Perform health check.
    async fn health_check(&self) -> Result<HealthStatus, HealthCheckError>;
}

/// Health check error.
#[derive(Debug, thiserror::Error)]
pub enum HealthCheckError {
    #[error("Health check timeout")]
    Timeout,

    #[error("Health check failed: {0}")]
    Failed(String),
}

/// Health check Observer.
///
/// Runs health checks at BeforeWaiting phase.
pub struct HealthCheckObserver {
    checks: parking_lot::RwLock<Vec<Arc<dyn HealthCheckable>>>,
    failure_threshold: u32,
    consecutive_failures: AtomicU32,
}

impl HealthCheckObserver {
    /// Create a new health check observer.
    pub fn new(failure_threshold: u32) -> Self {
        Self {
            checks: parking_lot::RwLock::new(Vec::new()),
            failure_threshold,
            consecutive_failures: AtomicU32::new(0),
        }
    }

    /// Register a health check component.
    pub fn register(&self, component: Arc<dyn HealthCheckable>) {
        self.checks.write().push(component);
    }

    /// Get the number of registered checks.
    pub fn check_count(&self) -> usize {
        self.checks.read().len()
    }

    /// Get consecutive failure count.
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl RunLoopObserver for HealthCheckObserver {
    fn activities(&self) -> u32 {
        RunLoopPhase::BeforeWaiting as u32
    }

    fn priority(&self) -> i32 {
        -100 // Run after other observers
    }

    async fn on_phase(&self, _phase: RunLoopPhase, run_loop: &RunLoop) {
        let checks = self.checks.read().clone();

        if checks.is_empty() {
            return;
        }

        let mut all_healthy = true;

        for check in &checks {
            match check.health_check().await {
                Ok(status) if status.is_healthy => {
                    debug!("Health check passed: {}", check.name());
                }
                Ok(status) => {
                    warn!(
                        "Health check degraded: {} - {}",
                        check.name(),
                        status.message
                    );
                    // Degraded is still considered healthy for failure counting
                }
                Err(e) => {
                    error!("Health check failed: {} - {}", check.name(), e);
                    all_healthy = false;
                }
            }
        }

        if all_healthy {
            self.consecutive_failures.store(0, Ordering::SeqCst);
        } else {
            let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;
            if failures >= self.failure_threshold {
                error!(
                    "Health check threshold exceeded ({}/{}), requesting shutdown",
                    failures, self.failure_threshold
                );
                run_loop.stop();
            }
        }
    }
}

/// Basic liveness check (always healthy).
pub struct LivenessCheck;

#[async_trait]
impl HealthCheckable for LivenessCheck {
    fn name(&self) -> &str {
        "liveness"
    }

    async fn health_check(&self) -> Result<HealthStatus, HealthCheckError> {
        Ok(HealthStatus::healthy())
    }
}

/// Memory check.
pub struct MemoryCheck {
    /// Memory usage threshold percentage (0-100).
    _threshold_percent: u32,
}

impl MemoryCheck {
    /// Create a new memory check.
    pub fn new(threshold_percent: u32) -> Self {
        Self { _threshold_percent: threshold_percent }
    }
}

#[async_trait]
impl HealthCheckable for MemoryCheck {
    fn name(&self) -> &str {
        "memory"
    }

    async fn health_check(&self) -> Result<HealthStatus, HealthCheckError> {
        // In a real implementation, this would check actual memory usage
        // For now, we just return healthy
        Ok(HealthStatus::healthy())
    }
}

/// Task queue check.
pub struct TaskQueueCheck {
    run_loop: Arc<RunLoop>,
    max_pending: usize,
}

impl TaskQueueCheck {
    /// Create a new task queue check.
    pub fn new(run_loop: Arc<RunLoop>, max_pending: usize) -> Self {
        Self {
            run_loop,
            max_pending,
        }
    }
}

#[async_trait]
impl HealthCheckable for TaskQueueCheck {
    fn name(&self) -> &str {
        "task_queue"
    }

    async fn health_check(&self) -> Result<HealthStatus, HealthCheckError> {
        let pending = self.run_loop.pending_task_count().await;

        if pending >= self.max_pending {
            return Ok(HealthStatus::unhealthy(format!(
                "Task queue is full: {} >= {}",
                pending, self.max_pending
            )));
        }

        if pending > self.max_pending / 2 {
            return Ok(HealthStatus::degraded(format!(
                "Task queue is over 50% full: {}/{}",
                pending, self.max_pending
            )));
        }

        Ok(HealthStatus::healthy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RunLoopConfig;

    #[test]
    fn test_health_status() {
        let healthy = HealthStatus::healthy();
        assert!(healthy.is_healthy);
        assert_eq!(healthy.message, "OK");

        let unhealthy = HealthStatus::unhealthy("error");
        assert!(!unhealthy.is_healthy);
        assert_eq!(unhealthy.message, "error");

        let degraded = HealthStatus::degraded("warning");
        assert!(degraded.is_healthy);
        assert_eq!(degraded.message, "warning");
    }

    #[tokio::test]
    async fn test_liveness_check() {
        let check = LivenessCheck;
        assert_eq!(check.name(), "liveness");

        let status = check.health_check().await.unwrap();
        assert!(status.is_healthy);
    }

    #[test]
    fn test_health_check_observer() {
        let observer = HealthCheckObserver::new(3);
        assert_eq!(observer.check_count(), 0);
        assert_eq!(observer.consecutive_failures(), 0);

        observer.register(Arc::new(LivenessCheck));
        assert_eq!(observer.check_count(), 1);
    }

    #[tokio::test]
    async fn test_task_queue_check() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let check = TaskQueueCheck::new(run_loop.clone(), 1000);

        assert_eq!(check.name(), "task_queue");

        let status = check.health_check().await.unwrap();
        assert!(status.is_healthy);
    }
}
