//! Monitoring and health check handlers.

use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::SystemTime;

use crate::state::AppState;

// ============================================================================
// Health Check Types
// ============================================================================

/// Health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Service is healthy.
    Healthy,
    /// Service is degraded but functional.
    Degraded,
    /// Service is unhealthy.
    Unhealthy,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall status.
    pub status: HealthStatus,
    /// Version information.
    pub version: String,
    /// Uptime in seconds.
    pub uptime_seconds: u64,
    /// Component health checks.
    pub components: Vec<ComponentHealth>,
}

/// Component health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name.
    pub name: String,
    /// Component status.
    pub status: HealthStatus,
    /// Optional message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ============================================================================
// Prometheus Metrics Types
// ============================================================================

/// Prometheus metrics response (text format).
#[derive(Debug)]
pub struct PrometheusMetrics {
    pub content: String,
}

impl IntoResponse for PrometheusMetrics {
    fn into_response(self) -> axum::response::Response {
        (
            [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
            self.content,
        )
            .into_response()
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// Start time for uptime calculation.
static START_TIME: std::sync::OnceLock<SystemTime> = std::sync::OnceLock::new();

/// Initialize start time (call on server start).
pub fn init_start_time() {
    START_TIME.get_or_init(SystemTime::now);
}

/// Get uptime in seconds.
fn get_uptime() -> u64 {
    START_TIME
        .get()
        .and_then(|start| start.elapsed().ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Enhanced health check handler.
pub async fn health_check_detailed(State(_state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let components = vec![
        ComponentHealth {
            name: "api".to_string(),
            status: HealthStatus::Healthy,
            message: None,
        },
        ComponentHealth {
            name: "websocket".to_string(),
            status: HealthStatus::Healthy,
            message: None,
        },
    ];

    // Determine overall status based on components
    let overall_status = if components.iter().any(|c| c.status == HealthStatus::Unhealthy) {
        HealthStatus::Unhealthy
    } else if components.iter().any(|c| c.status == HealthStatus::Degraded) {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    };

    Json(HealthResponse {
        status: overall_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: get_uptime(),
        components,
    })
}

/// Prometheus metrics endpoint.
pub async fn prometheus_metrics(State(_state): State<Arc<AppState>>) -> PrometheusMetrics {
    let uptime = get_uptime();

    let content = format!(
        r#"# HELP autohands_up Whether the AutoHands service is up
# TYPE autohands_up gauge
autohands_up 1

# HELP autohands_uptime_seconds Uptime in seconds
# TYPE autohands_uptime_seconds counter
autohands_uptime_seconds {}

# HELP autohands_info Service information
# TYPE autohands_info gauge
autohands_info{{version="{}"}} 1

# HELP autohands_http_requests_total Total HTTP requests
# TYPE autohands_http_requests_total counter
autohands_http_requests_total{{method="GET",endpoint="/health"}} 0
autohands_http_requests_total{{method="POST",endpoint="/tasks"}} 0

# HELP autohands_websocket_connections_active Active WebSocket connections
# TYPE autohands_websocket_connections_active gauge
autohands_websocket_connections_active 0

# HELP autohands_agents_active Active agent sessions
# TYPE autohands_agents_active gauge
autohands_agents_active 0
"#,
        uptime,
        env!("CARGO_PKG_VERSION")
    );

    PrometheusMetrics { content }
}

/// Liveness probe (Kubernetes).
pub async fn liveness_probe() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "alive"
    }))
}

/// Readiness probe (Kubernetes).
pub async fn readiness_probe(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // TODO: Check actual readiness (DB connections, etc.)
    Json(serde_json::json!({
        "status": "ready"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serialize() {
        assert_eq!(
            serde_json::to_string(&HealthStatus::Healthy).unwrap(),
            "\"healthy\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Degraded).unwrap(),
            "\"degraded\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Unhealthy).unwrap(),
            "\"unhealthy\""
        );
    }

    #[test]
    fn test_health_response_serialize() {
        let response = HealthResponse {
            status: HealthStatus::Healthy,
            version: "0.1.0".to_string(),
            uptime_seconds: 100,
            components: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("0.1.0"));
    }

    #[tokio::test]
    async fn test_liveness_probe() {
        let response = liveness_probe().await;
        assert_eq!(response.0["status"], "alive");
    }
}
