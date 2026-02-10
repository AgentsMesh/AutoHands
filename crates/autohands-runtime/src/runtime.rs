//! Agent runtime for managing agent execution.

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::Semaphore;

use autohands_core::registry::{ProviderRegistry, ToolRegistry};
use autohands_protocols::agent::Agent;
use autohands_protocols::memory::MemoryBackend;
use autohands_protocols::tool::AbortSignal;

use crate::agent_loop::AgentLoopConfig;
use crate::checkpoint::CheckpointSupport;
use crate::summarizer::HistoryCompressor;
use crate::history::HistoryManager;
use crate::session::SessionManager;

#[path = "runtime_impl.rs"]
mod runtime_impl;

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;

/// Configuration for the agent runtime.
#[derive(Debug, Clone)]
pub struct AgentRuntimeConfig {
    /// Maximum concurrent agent executions.
    pub max_concurrent: usize,

    /// Default agent loop config.
    pub default_loop_config: AgentLoopConfig,
}

impl Default for AgentRuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            default_loop_config: AgentLoopConfig::default(),
        }
    }
}

/// Agent execution handle for tracking running agents.
pub struct AgentHandle {
    /// Session ID.
    pub session_id: String,

    /// Abort signal for cancellation.
    pub abort_signal: Arc<AbortSignal>,
}

/// The agent runtime manages agent execution.
pub struct AgentRuntime {
    provider_registry: Arc<ProviderRegistry>,
    tool_registry: Arc<ToolRegistry>,
    session_manager: Arc<SessionManager>,
    history_manager: Arc<HistoryManager>,
    agents: DashMap<String, Arc<dyn Agent>>,
    running: DashMap<String, AgentHandle>,
    concurrency_semaphore: Arc<Semaphore>,
    config: AgentRuntimeConfig,
    checkpoint: Option<Arc<dyn CheckpointSupport>>,
    compressor: Option<Arc<HistoryCompressor>>,
    memory_backend: Option<Arc<dyn MemoryBackend>>,
}
