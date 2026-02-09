//! Interface server implementation.
//!
//! The server requires RunLoop for event processing.
//! All external requests flow through RunLoop.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::info;

use crate::http::routes::create_router_with_hybrid_state;
use crate::runloop_bridge::{HybridAppState, RunLoopState};
use crate::state::AppState;

/// Interface server configuration.
#[derive(Debug, Clone)]
pub struct InterfaceConfig {
    pub host: String,
    pub port: u16,
}

impl InterfaceConfig {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }
}

impl Default for InterfaceConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

/// The interface server.
///
/// Requires RunLoop state for event processing. All external requests
/// are converted to RunLoop events.
pub struct InterfaceServer {
    config: InterfaceConfig,
    state: Arc<HybridAppState>,
}

impl InterfaceServer {
    /// Create a new server with the required RunLoop state.
    pub fn new(config: InterfaceConfig, base: Arc<AppState>, runloop: Arc<RunLoopState>) -> Self {
        Self {
            config,
            state: Arc::new(HybridAppState::new(base, runloop)),
        }
    }

    /// Create a server with a pre-built HybridAppState.
    pub fn with_hybrid_state(config: InterfaceConfig, state: Arc<HybridAppState>) -> Self {
        Self { config, state }
    }

    /// Get the server address.
    pub fn addr(&self) -> String {
        format!("{}:{}", self.config.host, self.config.port)
    }

    /// Start the server.
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let app = create_router_with_hybrid_state(self.state.clone());

        let addr: SocketAddr = self.addr().parse()?;
        let listener = TcpListener::bind(addr).await?;

        info!("Interface server listening on {}", addr);
        axum::serve(listener, app).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use autohands_runloop::{TaskQueue, TaskQueueConfig};
    use tokio::sync::mpsc;

    fn create_test_state() -> (Arc<AppState>, Arc<RunLoopState>) {
        let base = Arc::new(AppState::default());
        let (tx, _rx) = mpsc::channel(16);
        let config = TaskQueueConfig::default();
        let queue = Arc::new(TaskQueue::new(config, 100));
        let runloop = Arc::new(RunLoopState::new(tx, queue));
        (base, runloop)
    }

    #[test]
    fn test_interface_config_default() {
        let config = InterfaceConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_interface_config_new() {
        let config = InterfaceConfig::new("0.0.0.0", 3000);
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
    }

    #[test]
    fn test_interface_server_creation() {
        let config = InterfaceConfig::default();
        let (base, runloop) = create_test_state();
        let server = InterfaceServer::new(config, base, runloop);
        assert_eq!(server.addr(), "127.0.0.1:8080");
    }

    #[test]
    fn test_interface_server_with_hybrid_state() {
        let config = InterfaceConfig::default();
        let (base, runloop) = create_test_state();
        let hybrid = Arc::new(HybridAppState::new(base, runloop));
        let server = InterfaceServer::with_hybrid_state(config, hybrid);
        assert_eq!(server.addr(), "127.0.0.1:8080");
    }

    #[test]
    fn test_interface_config_debug() {
        let config = InterfaceConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("InterfaceConfig"));
    }

    #[test]
    fn test_interface_config_clone() {
        let config = InterfaceConfig::new("localhost", 9000);
        let cloned = config.clone();
        assert_eq!(cloned.host, "localhost");
        assert_eq!(cloned.port, 9000);
    }

    #[test]
    fn test_interface_server_addr_format() {
        let config = InterfaceConfig::new("192.168.1.1", 443);
        let (base, runloop) = create_test_state();
        let server = InterfaceServer::new(config, base, runloop);
        assert_eq!(server.addr(), "192.168.1.1:443");
    }

    #[test]
    fn test_interface_config_string_host() {
        let config = InterfaceConfig::new(String::from("custom.host.com"), 8443);
        assert_eq!(config.host, "custom.host.com");
        assert_eq!(config.port, 8443);
    }
}
