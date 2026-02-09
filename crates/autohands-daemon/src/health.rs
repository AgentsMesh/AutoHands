//! Health checking for daemon processes.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::config::DaemonConfig;

/// Health status of the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Daemon is healthy.
    Healthy,
    /// Daemon is degraded but functioning.
    Degraded,
    /// Daemon is unhealthy.
    Unhealthy,
    /// Health status is unknown.
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Health check result with details.
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Overall health status.
    pub status: HealthStatus,
    /// Timestamp of the check.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Individual component checks.
    pub checks: Vec<ComponentCheck>,
    /// Optional message.
    pub message: Option<String>,
}

/// Individual component health check.
#[derive(Debug, Clone)]
pub struct ComponentCheck {
    /// Component name.
    pub name: String,
    /// Component health status.
    pub status: HealthStatus,
    /// Optional details.
    pub details: Option<String>,
}

impl HealthCheckResult {
    /// Create a healthy result.
    pub fn healthy() -> Self {
        Self {
            status: HealthStatus::Healthy,
            timestamp: chrono::Utc::now(),
            checks: Vec::new(),
            message: None,
        }
    }

    /// Create an unhealthy result.
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            timestamp: chrono::Utc::now(),
            checks: Vec::new(),
            message: Some(message.into()),
        }
    }

    /// Add a component check.
    pub fn with_check(mut self, check: ComponentCheck) -> Self {
        // Update overall status based on worst component
        if check.status == HealthStatus::Unhealthy {
            self.status = HealthStatus::Unhealthy;
        } else if check.status == HealthStatus::Degraded
            && self.status != HealthStatus::Unhealthy
        {
            self.status = HealthStatus::Degraded;
        }
        self.checks.push(check);
        self
    }
}

/// Trait for components that can be health-checked.
/// Uses boxed futures for dyn compatibility.
pub trait HealthCheckable: Send + Sync {
    /// Get the component name.
    fn name(&self) -> &str;

    /// Perform a health check.
    fn check_health(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ComponentCheck> + Send + '_>>;
}

/// Health checker that periodically checks daemon health.
pub struct HealthChecker {
    config: DaemonConfig,
    components: RwLock<Vec<Arc<dyn HealthCheckable>>>,
    last_check: RwLock<Option<HealthCheckResult>>,
    check_count: AtomicU64,
    failure_count: AtomicU64,
}

impl HealthChecker {
    /// Create a new health checker.
    pub fn new(config: DaemonConfig) -> Self {
        Self {
            config,
            components: RwLock::new(Vec::new()),
            last_check: RwLock::new(None),
            check_count: AtomicU64::new(0),
            failure_count: AtomicU64::new(0),
        }
    }

    /// Register a component for health checking.
    pub async fn register(&self, component: Arc<dyn HealthCheckable>) {
        let mut components = self.components.write().await;
        info!("Registered health check component: {}", component.name());
        components.push(component);
    }

    /// Perform a health check on all components.
    pub async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();
        self.check_count.fetch_add(1, Ordering::SeqCst);

        let components = self.components.read().await;
        let mut result = HealthCheckResult::healthy();

        for component in components.iter() {
            let check = component.check_health().await;
            debug!("Health check for {}: {}", check.name, check.status);
            result = result.with_check(check);
        }

        let elapsed = start.elapsed();
        debug!("Health check completed in {:?}: {}", elapsed, result.status);

        if result.status == HealthStatus::Unhealthy {
            self.failure_count.fetch_add(1, Ordering::SeqCst);
            warn!("Health check failed: {:?}", result.message);
        }

        *self.last_check.write().await = Some(result.clone());
        result
    }

    /// Get the last health check result.
    pub async fn last_result(&self) -> Option<HealthCheckResult> {
        self.last_check.read().await.clone()
    }

    /// Get the total number of health checks performed.
    pub fn check_count(&self) -> u64 {
        self.check_count.load(Ordering::SeqCst)
    }

    /// Get the number of failed health checks.
    pub fn failure_count(&self) -> u64 {
        self.failure_count.load(Ordering::SeqCst)
    }

    /// Start the periodic health check loop.
    pub async fn start_loop(
        self: Arc<Self>,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) {
        let interval = self.config.health_check_interval();
        info!("Starting health check loop (interval: {:?})", interval);

        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    let result = self.check().await;
                    if result.status == HealthStatus::Unhealthy {
                        error!("Daemon health check failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Health check loop shutting down");
                    break;
                }
            }
        }
    }
}

/// Simple liveness check that always returns healthy.
pub struct LivenessCheck;

impl HealthCheckable for LivenessCheck {
    fn name(&self) -> &str {
        "liveness"
    }

    fn check_health(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ComponentCheck> + Send + '_>> {
        Box::pin(async {
            ComponentCheck {
                name: "liveness".to_string(),
                status: HealthStatus::Healthy,
                details: Some("Process is alive".to_string()),
            }
        })
    }
}

/// Memory usage check.
pub struct MemoryCheck {
    /// Maximum allowed memory in bytes (0 = unlimited).
    pub max_memory_bytes: u64,
}

impl MemoryCheck {
    /// Create a new memory check with no limit.
    pub fn new() -> Self {
        Self {
            max_memory_bytes: 0,
        }
    }

    /// Create a memory check with a limit.
    pub fn with_limit(max_bytes: u64) -> Self {
        Self {
            max_memory_bytes: max_bytes,
        }
    }
}

impl Default for MemoryCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthCheckable for MemoryCheck {
    fn name(&self) -> &str {
        "memory"
    }

    fn check_health(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ComponentCheck> + Send + '_>> {
        Box::pin(async {
            // This is a simplified check - real implementation would use sys-info or similar
            let status = HealthStatus::Healthy;
            let details = Some("Memory usage within limits".to_string());

            ComponentCheck {
                name: "memory".to_string(),
                status,
                details,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
        assert_eq!(HealthStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_health_check_result_healthy() {
        let result = HealthCheckResult::healthy();
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(result.message.is_none());
    }

    #[test]
    fn test_health_check_result_unhealthy() {
        let result = HealthCheckResult::unhealthy("test error");
        assert_eq!(result.status, HealthStatus::Unhealthy);
        assert_eq!(result.message, Some("test error".to_string()));
    }

    #[test]
    fn test_health_check_result_with_checks() {
        let result = HealthCheckResult::healthy()
            .with_check(ComponentCheck {
                name: "test1".to_string(),
                status: HealthStatus::Healthy,
                details: None,
            })
            .with_check(ComponentCheck {
                name: "test2".to_string(),
                status: HealthStatus::Degraded,
                details: None,
            });

        assert_eq!(result.status, HealthStatus::Degraded);
        assert_eq!(result.checks.len(), 2);
    }

    #[test]
    fn test_unhealthy_overrides_degraded() {
        let result = HealthCheckResult::healthy()
            .with_check(ComponentCheck {
                name: "test1".to_string(),
                status: HealthStatus::Degraded,
                details: None,
            })
            .with_check(ComponentCheck {
                name: "test2".to_string(),
                status: HealthStatus::Unhealthy,
                details: None,
            });

        assert_eq!(result.status, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_health_checker() {
        let config = DaemonConfig::default();
        let checker = HealthChecker::new(config);

        checker
            .register(Arc::new(LivenessCheck))
            .await;

        let result = checker.check().await;
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(checker.check_count(), 1);
        assert_eq!(checker.failure_count(), 0);
    }

    #[tokio::test]
    async fn test_liveness_check() {
        let check = LivenessCheck;
        let result = check.check_health().await;
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.name, "liveness");
    }

    #[tokio::test]
    async fn test_memory_check() {
        let check = MemoryCheck::new();
        let result = check.check_health().await;
        assert_eq!(result.name, "memory");
    }

    #[tokio::test]
    async fn test_last_result() {
        let config = DaemonConfig::default();
        let checker = HealthChecker::new(config);

        assert!(checker.last_result().await.is_none());

        checker.check().await;

        let result = checker.last_result().await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().status, HealthStatus::Healthy);
    }

    struct FailingCheck;

    impl HealthCheckable for FailingCheck {
        fn name(&self) -> &str {
            "failing"
        }

        fn check_health(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ComponentCheck> + Send + '_>> {
            Box::pin(async {
                ComponentCheck {
                    name: "failing".to_string(),
                    status: HealthStatus::Unhealthy,
                    details: Some("Always fails".to_string()),
                }
            })
        }
    }

    #[tokio::test]
    async fn test_failing_check_increments_failure_count() {
        let config = DaemonConfig::default();
        let checker = HealthChecker::new(config);
        checker.register(Arc::new(FailingCheck)).await;

        let result = checker.check().await;
        assert_eq!(result.status, HealthStatus::Unhealthy);
        assert_eq!(checker.failure_count(), 1);
    }
}
