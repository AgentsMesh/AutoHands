//! Application state.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Notify;

use autohands_core::registry::{ProviderRegistry, ToolRegistry};
use autohands_core::Kernel;
use autohands_runtime::{AgentLoopConfig, AgentRuntime, AgentRuntimeConfig, Session, SessionManager, TranscriptManager};

/// Application state shared across handlers.
pub struct AppState {
    pub provider_registry: Arc<ProviderRegistry>,
    pub tool_registry: Arc<ToolRegistry>,
    pub session_manager: Arc<SessionManager>,
    pub kernel: Arc<Kernel>,
    pub agent_runtime: Arc<AgentRuntime>,
    pub transcript_manager: Arc<TranscriptManager>,
    start_time: Instant,
    request_count: AtomicU64,
    shutdown_requested: AtomicBool,
    /// Notifier for API-triggered shutdown.
    pub shutdown_notify: Arc<Notify>,
}

impl AppState {
    pub fn new(
        provider_registry: Arc<ProviderRegistry>,
        tool_registry: Arc<ToolRegistry>,
        kernel: Arc<Kernel>,
        agent_runtime: Arc<AgentRuntime>,
        transcript_dir: PathBuf,
    ) -> Self {
        Self {
            provider_registry,
            tool_registry,
            session_manager: Arc::new(SessionManager::new()),
            kernel,
            agent_runtime,
            transcript_manager: Arc::new(TranscriptManager::new(transcript_dir)),
            start_time: Instant::now(),
            request_count: AtomicU64::new(0),
            shutdown_requested: AtomicBool::new(false),
            shutdown_notify: Arc::new(Notify::new()),
        }
    }

    /// Get uptime.
    pub fn uptime(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Get request count.
    pub fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }

    /// Increment request count.
    pub fn increment_requests(&self) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Request shutdown and notify the server signal handler.
    pub fn request_shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::SeqCst);
        self.shutdown_notify.notify_one();
    }

    /// Check if shutdown is requested.
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }

    /// List all sessions.
    pub fn list_sessions(&self) -> Vec<Session> {
        // This is a simple implementation - in production you'd want pagination
        // Note: SessionManager doesn't expose iteration, so we'd need to add that
        // For now, return empty
        Vec::new()
    }
}

impl Default for AppState {
    fn default() -> Self {
        let provider_registry = Arc::new(ProviderRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let runtime_config = AgentRuntimeConfig {
            max_concurrent: 10,
            default_loop_config: AgentLoopConfig::default(),
        };
        let agent_runtime = Arc::new(AgentRuntime::new(
            provider_registry.clone(),
            tool_registry.clone(),
            runtime_config,
        ));

        // Default transcript directory
        let transcript_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("autohands")
            .join("transcripts");

        Self {
            provider_registry,
            tool_registry,
            session_manager: Arc::new(SessionManager::new()),
            kernel: Arc::new(Kernel::new(PathBuf::from("."))),
            agent_runtime,
            transcript_manager: Arc::new(TranscriptManager::new(transcript_dir)),
            start_time: Instant::now(),
            request_count: AtomicU64::new(0),
            shutdown_requested: AtomicBool::new(false),
            shutdown_notify: Arc::new(Notify::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let state = AppState::default();
        assert!(state.provider_registry.list_ids().is_empty());
    }

    #[test]
    fn test_app_state_new() {
        let provider_reg = Arc::new(ProviderRegistry::new());
        let tool_reg = Arc::new(ToolRegistry::new());
        let kernel = Arc::new(Kernel::new(PathBuf::from(".")));
        let runtime_config = AgentRuntimeConfig {
            max_concurrent: 10,
            default_loop_config: AgentLoopConfig::default(),
        };
        let agent_runtime = Arc::new(AgentRuntime::new(
            provider_reg.clone(),
            tool_reg.clone(),
            runtime_config,
        ));
        let transcript_dir = PathBuf::from("/tmp/test-transcripts");
        let state = AppState::new(provider_reg, tool_reg, kernel, agent_runtime, transcript_dir);
        assert!(state.provider_registry.list_ids().is_empty());
    }

    #[test]
    fn test_request_count() {
        let state = AppState::default();
        assert_eq!(state.request_count(), 0);

        state.increment_requests();
        assert_eq!(state.request_count(), 1);

        state.increment_requests();
        assert_eq!(state.request_count(), 2);
    }

    #[test]
    fn test_shutdown_requested() {
        let state = AppState::default();
        assert!(!state.is_shutdown_requested());

        state.request_shutdown();
        assert!(state.is_shutdown_requested());
    }

    #[test]
    fn test_uptime() {
        let state = AppState::default();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(state.uptime().as_millis() >= 10);
    }
}
