//! AgentRuntime method implementations.

use std::sync::Arc;

use tracing::{info, warn};

use autohands_protocols::agent::AgentContext;
use autohands_protocols::error::AgentError;
use autohands_protocols::memory::MemoryBackend;
use autohands_protocols::tool::AbortSignal;
use autohands_protocols::types::Message;

use crate::agent_loop::AgentLoop;
use crate::checkpoint::CheckpointSupport;
use crate::summarizer::HistoryCompressor;
use crate::history::HistoryManager;
use crate::session::SessionManager;
use crate::transcript::TranscriptWriter;

use super::{AgentHandle, AgentRuntime, AgentRuntimeConfig};

impl AgentRuntime {
    /// Create a new agent runtime.
    pub fn new(
        provider_registry: Arc<autohands_core::registry::ProviderRegistry>,
        tool_registry: Arc<autohands_core::registry::ToolRegistry>,
        config: AgentRuntimeConfig,
    ) -> Self {
        Self {
            provider_registry,
            tool_registry,
            session_manager: Arc::new(SessionManager::new()),
            history_manager: Arc::new(HistoryManager::new()),
            agents: dashmap::DashMap::new(),
            running: dashmap::DashMap::new(),
            concurrency_semaphore: Arc::new(tokio::sync::Semaphore::new(config.max_concurrent)),
            config,
            checkpoint: None,
            compressor: None,
            memory_backend: None,
        }
    }

    /// Set checkpoint support for agent loops.
    pub fn with_checkpoint(mut self, checkpoint: Arc<dyn CheckpointSupport>) -> Self {
        self.checkpoint = Some(checkpoint);
        self
    }

    /// Set history compressor for context length recovery.
    pub fn with_compressor(mut self, compressor: Arc<HistoryCompressor>) -> Self {
        self.compressor = Some(compressor);
        self
    }

    /// Set memory backend for context injection and memory flush.
    pub fn with_memory(mut self, backend: Arc<dyn MemoryBackend>) -> Self {
        self.memory_backend = Some(backend);
        self
    }

    /// Get history manager.
    pub fn history_manager(&self) -> &Arc<HistoryManager> {
        &self.history_manager
    }

    /// Register an agent.
    pub fn register_agent(&self, agent: Arc<dyn autohands_protocols::agent::Agent>) {
        let id = agent.id().to_string();
        info!("Registering agent: {}", id);
        self.agents.insert(id, agent);
    }

    /// Unregister an agent.
    pub fn unregister_agent(&self, agent_id: &str) {
        self.agents.remove(agent_id);
    }

    /// Get a registered agent.
    pub fn get_agent(&self, agent_id: &str) -> Option<Arc<dyn autohands_protocols::agent::Agent>> {
        self.agents.get(agent_id).map(|a| a.clone())
    }

    /// List all registered agents.
    pub fn list_agents(&self) -> Vec<autohands_protocols::agent::AgentConfig> {
        self.agents.iter().map(|a| a.config().clone()).collect()
    }

    /// Execute an agent with a message.
    pub async fn execute(
        &self,
        agent_id: &str,
        session_id: &str,
        message: Message,
    ) -> Result<Vec<Message>, AgentError> {
        self.execute_with_transcript(agent_id, session_id, message, None).await
    }

    /// Execute an agent with a message and optional transcript recording.
    pub async fn execute_with_transcript(
        &self,
        agent_id: &str,
        session_id: &str,
        message: Message,
        transcript: Option<Arc<TranscriptWriter>>,
    ) -> Result<Vec<Message>, AgentError> {
        let agent = self
            .agents
            .get(agent_id)
            .ok_or_else(|| AgentError::NotFound(agent_id.to_string()))?
            .clone();

        // Acquire semaphore permit for concurrency control
        let _permit = self.concurrency_semaphore.acquire().await.map_err(|_| {
            AgentError::ExecutionFailed("Failed to acquire concurrency permit".to_string())
        })?;

        // Create abort signal
        let abort_signal = Arc::new(AbortSignal::new());

        // Register as running — use a RAII guard to ensure cleanup on all paths
        // (including panics, early returns, and errors).
        self.running.insert(
            session_id.to_string(),
            AgentHandle {
                session_id: session_id.to_string(),
                abort_signal: abort_signal.clone(),
            },
        );
        let running_ref = &self.running;
        let session_key = session_id.to_string();
        let _running_guard = RunningGuard {
            running: running_ref,
            key: &session_key,
        };

        // Get conversation history for this session
        let history = self.history_manager.get(session_id);
        let history_messages = history.messages().to_vec();

        // Create context with history from HistoryManager
        let ctx = AgentContext::new(session_id).with_history(history_messages);
        let ctx = AgentContext {
            abort_signal,
            ..ctx
        };

        // Record user message to history
        self.history_manager.push(session_id, message.clone());

        // Create and run agent loop with transcript and optional checkpoint
        let mut agent_loop = AgentLoop::new(
            self.provider_registry.clone(),
            self.tool_registry.clone(),
            self.config.default_loop_config.clone(),
        )
        .with_transcript(transcript);

        if let Some(ref checkpoint) = self.checkpoint {
            agent_loop = agent_loop.with_checkpoint(checkpoint.clone());
        }
        if let Some(ref compressor) = self.compressor {
            agent_loop = agent_loop.with_compressor(compressor.clone());
        }
        if let Some(ref memory) = self.memory_backend {
            agent_loop = agent_loop.with_memory(memory.clone());
        }

        let result = agent_loop.run_with_recovery(agent.as_ref(), ctx, message).await;

        // Record agent response messages to history
        if let Ok(ref messages) = result {
            for msg in messages {
                self.history_manager.push(session_id, msg.clone());
            }
        }

        // _running_guard drops here, removing from self.running on all paths
        result
    }

    /// Abort a running agent execution.
    ///
    /// Only sets the abort signal without removing from the running map.
    /// The `RunningGuard` RAII will clean up the map entry when execution completes.
    pub fn abort(&self, session_id: &str) -> bool {
        if let Some(handle) = self.running.get(session_id) {
            handle.abort_signal.abort();
            info!("Aborted agent execution: {}", session_id);
            true
        } else {
            warn!("No running agent found: {}", session_id);
            false
        }
    }

    /// Check if an agent is running.
    pub fn is_running(&self, session_id: &str) -> bool {
        self.running.contains_key(session_id)
    }

    /// Get the number of currently running agents.
    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    /// Get the set of currently running session IDs.
    pub fn running_sessions(&self) -> std::collections::HashSet<String> {
        self.running.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get session manager.
    pub fn session_manager(&self) -> &Arc<SessionManager> {
        &self.session_manager
    }

    /// Clear conversation history for a session.
    pub fn clear_history(&self, session_id: &str) {
        self.history_manager.clear(session_id);
    }
}

/// RAII guard that removes a session from the `running` DashMap on drop.
///
/// Ensures the running map is cleaned up even if the execution path panics
/// or returns early due to an error.
struct RunningGuard<'a> {
    running: &'a dashmap::DashMap<String, AgentHandle>,
    key: &'a str,
}

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.running.remove(self.key);
    }
}
