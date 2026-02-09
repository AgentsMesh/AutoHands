//! Prometheus-style metrics endpoint.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::{http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Metric type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// Counter (monotonically increasing).
    Counter,
    /// Gauge (can go up and down).
    Gauge,
    /// Histogram.
    Histogram,
}

/// A metric definition.
#[derive(Debug, Clone)]
pub struct MetricDef {
    /// Metric name.
    pub name: String,
    /// Metric type.
    pub metric_type: MetricType,
    /// Help text.
    pub help: String,
    /// Labels.
    pub labels: Vec<String>,
}

/// Metric value with labels.
#[derive(Debug, Clone)]
pub struct MetricValue {
    /// Label values.
    pub labels: HashMap<String, String>,
    /// Current value.
    pub value: f64,
}

/// Metrics registry.
pub struct MetricsRegistry {
    definitions: RwLock<HashMap<String, MetricDef>>,
    counters: RwLock<HashMap<String, Arc<AtomicU64>>>,
    gauges: RwLock<HashMap<String, Arc<AtomicU64>>>,
}

impl MetricsRegistry {
    /// Create a new metrics registry.
    pub fn new() -> Self {
        Self {
            definitions: RwLock::new(HashMap::new()),
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
        }
    }

    /// Register a counter.
    pub async fn register_counter(&self, name: impl Into<String>, help: impl Into<String>) {
        let name = name.into();
        let mut defs = self.definitions.write().await;
        defs.insert(
            name.clone(),
            MetricDef {
                name: name.clone(),
                metric_type: MetricType::Counter,
                help: help.into(),
                labels: Vec::new(),
            },
        );

        let mut counters = self.counters.write().await;
        counters.insert(name, Arc::new(AtomicU64::new(0)));
    }

    /// Register a gauge.
    pub async fn register_gauge(&self, name: impl Into<String>, help: impl Into<String>) {
        let name = name.into();
        let mut defs = self.definitions.write().await;
        defs.insert(
            name.clone(),
            MetricDef {
                name: name.clone(),
                metric_type: MetricType::Gauge,
                help: help.into(),
                labels: Vec::new(),
            },
        );

        let mut gauges = self.gauges.write().await;
        gauges.insert(name, Arc::new(AtomicU64::new(0)));
    }

    /// Increment a counter.
    pub async fn inc_counter(&self, name: &str) {
        let counters = self.counters.read().await;
        if let Some(counter) = counters.get(name) {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Add to a counter.
    pub async fn add_counter(&self, name: &str, value: u64) {
        let counters = self.counters.read().await;
        if let Some(counter) = counters.get(name) {
            counter.fetch_add(value, Ordering::SeqCst);
        }
    }

    /// Set a gauge value.
    pub async fn set_gauge(&self, name: &str, value: u64) {
        let gauges = self.gauges.read().await;
        if let Some(gauge) = gauges.get(name) {
            gauge.store(value, Ordering::SeqCst);
        }
    }

    /// Get a counter value.
    pub async fn get_counter(&self, name: &str) -> Option<u64> {
        let counters = self.counters.read().await;
        counters.get(name).map(|c| c.load(Ordering::SeqCst))
    }

    /// Get a gauge value.
    pub async fn get_gauge(&self, name: &str) -> Option<u64> {
        let gauges = self.gauges.read().await;
        gauges.get(name).map(|g| g.load(Ordering::SeqCst))
    }

    /// Export metrics in Prometheus format.
    pub async fn export(&self) -> String {
        let defs = self.definitions.read().await;
        let counters = self.counters.read().await;
        let gauges = self.gauges.read().await;

        let mut output = String::new();

        for (name, def) in defs.iter() {
            let type_str = match def.metric_type {
                MetricType::Counter => "counter",
                MetricType::Gauge => "gauge",
                MetricType::Histogram => "histogram",
            };

            output.push_str(&format!("# HELP {} {}\n", name, def.help));
            output.push_str(&format!("# TYPE {} {}\n", name, type_str));

            let value = match def.metric_type {
                MetricType::Counter => counters.get(name).map(|c| c.load(Ordering::SeqCst)),
                MetricType::Gauge => gauges.get(name).map(|g| g.load(Ordering::SeqCst)),
                MetricType::Histogram => None, // Not implemented
            };

            if let Some(v) = value {
                output.push_str(&format!("{} {}\n", name, v));
            }
        }

        output
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics endpoint handler.
pub struct MetricsEndpoint {
    registry: Arc<MetricsRegistry>,
}

impl MetricsEndpoint {
    /// Create a new metrics endpoint.
    pub fn new(registry: Arc<MetricsRegistry>) -> Self {
        Self { registry }
    }

    /// Axum handler for metrics.
    pub async fn handler(&self) -> impl IntoResponse {
        let metrics = self.registry.export().await;
        (
            StatusCode::OK,
            [("content-type", "text/plain; charset=utf-8")],
            metrics,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_counter() {
        let registry = MetricsRegistry::new();
        registry.register_counter("requests_total", "Total requests").await;

        registry.inc_counter("requests_total").await;
        registry.inc_counter("requests_total").await;

        assert_eq!(registry.get_counter("requests_total").await, Some(2));
    }

    #[tokio::test]
    async fn test_registry_gauge() {
        let registry = MetricsRegistry::new();
        registry.register_gauge("active_connections", "Active connections").await;

        registry.set_gauge("active_connections", 5).await;
        assert_eq!(registry.get_gauge("active_connections").await, Some(5));

        registry.set_gauge("active_connections", 3).await;
        assert_eq!(registry.get_gauge("active_connections").await, Some(3));
    }

    #[tokio::test]
    async fn test_export() {
        let registry = MetricsRegistry::new();
        registry.register_counter("test_counter", "A test counter").await;
        registry.inc_counter("test_counter").await;

        let output = registry.export().await;
        assert!(output.contains("# HELP test_counter A test counter"));
        assert!(output.contains("# TYPE test_counter counter"));
        assert!(output.contains("test_counter 1"));
    }
}
