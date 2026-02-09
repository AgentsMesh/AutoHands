//! Health check endpoint.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall status.
    pub status: HealthStatus,
    /// Version.
    pub version: String,
    /// Uptime in seconds.
    pub uptime_secs: u64,
    /// Component statuses.
    pub components: HashMap<String, ComponentHealth>,
}

/// Health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// System is healthy.
    Healthy,
    /// System is degraded but functional.
    Degraded,
    /// System is unhealthy.
    Unhealthy,
}

/// Component health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component status.
    pub status: HealthStatus,
    /// Optional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Health endpoint handler.
pub struct HealthEndpoint {
    version: String,
    start_time: std::time::Instant,
}

impl HealthEndpoint {
    /// Create a new health endpoint.
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            start_time: std::time::Instant::now(),
        }
    }

    /// Get uptime in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Generate health response.
    pub fn check(&self, components: HashMap<String, ComponentHealth>) -> HealthResponse {
        let status = components
            .values()
            .map(|c| c.status)
            .fold(HealthStatus::Healthy, |acc, s| {
                if s == HealthStatus::Unhealthy || acc == HealthStatus::Unhealthy {
                    HealthStatus::Unhealthy
                } else if s == HealthStatus::Degraded || acc == HealthStatus::Degraded {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                }
            });

        HealthResponse {
            status,
            version: self.version.clone(),
            uptime_secs: self.uptime_secs(),
            components,
        }
    }

    /// Axum handler for health check.
    pub async fn handler(
        &self,
        components: HashMap<String, ComponentHealth>,
    ) -> impl IntoResponse {
        let response = self.check(components);
        let status_code = match response.status {
            HealthStatus::Healthy => StatusCode::OK,
            HealthStatus::Degraded => StatusCode::OK,
            HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        };

        (status_code, Json(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_endpoint_new() {
        let endpoint = HealthEndpoint::new("1.0.0");
        assert!(endpoint.uptime_secs() < 1);
    }

    #[test]
    fn test_health_check_healthy() {
        let endpoint = HealthEndpoint::new("1.0.0");
        let mut components = HashMap::new();
        components.insert(
            "database".to_string(),
            ComponentHealth {
                status: HealthStatus::Healthy,
                details: None,
            },
        );

        let response = endpoint.check(components);
        assert_eq!(response.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_check_degraded() {
        let endpoint = HealthEndpoint::new("1.0.0");
        let mut components = HashMap::new();
        components.insert(
            "database".to_string(),
            ComponentHealth {
                status: HealthStatus::Healthy,
                details: None,
            },
        );
        components.insert(
            "cache".to_string(),
            ComponentHealth {
                status: HealthStatus::Degraded,
                details: Some("High latency".to_string()),
            },
        );

        let response = endpoint.check(components);
        assert_eq!(response.status, HealthStatus::Degraded);
    }

    #[test]
    fn test_health_check_unhealthy() {
        let endpoint = HealthEndpoint::new("1.0.0");
        let mut components = HashMap::new();
        components.insert(
            "database".to_string(),
            ComponentHealth {
                status: HealthStatus::Unhealthy,
                details: Some("Connection failed".to_string()),
            },
        );

        let response = endpoint.check(components);
        assert_eq!(response.status, HealthStatus::Unhealthy);
    }
}
