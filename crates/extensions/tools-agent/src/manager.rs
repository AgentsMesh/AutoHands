//! Sub-agent manager.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use autohands_protocols::tool::AbortSignal;
use autohands_protocols::types::Message;
use autohands_runtime::AgentRuntime;

/// Errors from agent management operations.
#[derive(Debug, Error)]
pub enum AgentManagerError {
    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Agent already exists: {0}")]
    AlreadyExists(String),

    #[error("Spawn failed: {0}")]
    SpawnFailed(String),

    #[error("Communication failed: {0}")]
    CommunicationFailed(String),

    #[error("Agent terminated: {0}")]
    Terminated(String),

    #[error("Runtime not available")]
    RuntimeNotAvailable,
}

/// Status of a spawned agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpawnedAgentStatus {
    /// Agent is starting up.
    Starting,
    /// Agent is running and processing.
    Running,
    /// Agent is idle, waiting for input.
    Idle,
    /// Agent completed successfully.
    Completed,
    /// Agent failed with an error.
    Failed,
    /// Agent was terminated.
    Terminated,
}

/// Information about a spawned sub-agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedAgent {
    /// Unique ID of this spawned agent instance.
    pub id: String,
    /// ID of the agent template used.
    pub agent_id: String,
    /// Session ID for this instance.
    pub session_id: String,
    /// Parent agent ID that spawned this agent.
    pub parent_id: Option<String>,
    /// Current status.
    pub status: SpawnedAgentStatus,
    /// Task description given to the agent.
    pub task: String,
    /// When the agent was spawned.
    pub spawned_at: DateTime<Utc>,
    /// When the agent completed (if applicable).
    pub completed_at: Option<DateTime<Utc>>,
    /// Last message received from the agent.
    pub last_message: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Tools available to this agent.
    pub tools: Vec<String>,
    /// Custom metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Internal state for a running agent.
struct RunningAgent {
    info: SpawnedAgent,
    abort_signal: Arc<AbortSignal>,
    result_sender: mpsc::Sender<AgentResult>,
}

/// Result from an agent execution.
#[derive(Debug)]
pub struct AgentResult {
    pub agent_id: String,
    pub success: bool,
    pub messages: Vec<Message>,
    pub error: Option<String>,
}

/// Manages spawned sub-agents.
pub struct AgentManager {
    runtime: RwLock<Option<Arc<AgentRuntime>>>,
    agents: Arc<DashMap<String, RunningAgent>>,
    results: Arc<DashMap<String, AgentResult>>,
    max_concurrent: usize,
}

impl AgentManager {
    /// Create a new agent manager.
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            runtime: RwLock::new(None),
            agents: Arc::new(DashMap::new()),
            results: Arc::new(DashMap::new()),
            max_concurrent,
        }
    }

    /// Set the agent runtime to use for spawning agents.
    pub fn set_runtime(&self, runtime: Arc<AgentRuntime>) {
        *self.runtime.write() = Some(runtime);
    }

    /// Spawn a new sub-agent.
    pub async fn spawn(
        &self,
        agent_id: &str,
        task: &str,
        parent_id: Option<&str>,
        tools: Vec<String>,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<SpawnedAgent, AgentManagerError> {
        // Check concurrent limit
        let running_count = self
            .agents
            .iter()
            .filter(|a| {
                matches!(
                    a.info.status,
                    SpawnedAgentStatus::Starting | SpawnedAgentStatus::Running
                )
            })
            .count();

        if running_count >= self.max_concurrent {
            return Err(AgentManagerError::SpawnFailed(format!(
                "Max concurrent agents ({}) reached",
                self.max_concurrent
            )));
        }

        // Get runtime
        let runtime = self
            .runtime
            .read()
            .clone()
            .ok_or(AgentManagerError::RuntimeNotAvailable)?;

        // Verify agent exists
        if runtime.get_agent(agent_id).is_none() {
            return Err(AgentManagerError::NotFound(agent_id.to_string()));
        }

        // Generate IDs
        let spawn_id = uuid::Uuid::new_v4().to_string();
        let session_id = format!("spawn-{}", spawn_id);

        // Create agent info
        let info = SpawnedAgent {
            id: spawn_id.clone(),
            agent_id: agent_id.to_string(),
            session_id: session_id.clone(),
            parent_id: parent_id.map(|s| s.to_string()),
            status: SpawnedAgentStatus::Starting,
            task: task.to_string(),
            spawned_at: Utc::now(),
            completed_at: None,
            last_message: None,
            error: None,
            tools,
            metadata,
        };

        // Create communication channels
        let (result_tx, mut result_rx) = mpsc::channel::<AgentResult>(1);
        let abort_signal = Arc::new(AbortSignal::new());

        // Store running agent
        self.agents.insert(
            spawn_id.clone(),
            RunningAgent {
                info: info.clone(),
                abort_signal: abort_signal.clone(),
                result_sender: result_tx,
            },
        );

        // Spawn the agent execution task
        let manager_agents = self.agents.clone();
        let manager_results = self.results.clone();
        let spawn_id_clone = spawn_id.clone();
        let task_clone = task.to_string();
        let agent_id_clone = agent_id.to_string();
        let session_id_clone = session_id.clone();

        tokio::spawn(async move {
            // Update status to running
            if let Some(mut agent) = manager_agents.get_mut(&spawn_id_clone) {
                agent.info.status = SpawnedAgentStatus::Running;
            }

            info!(
                "Sub-agent {} started: {} (task: {})",
                spawn_id_clone, agent_id_clone, task_clone
            );

            // Execute agent
            let message = Message::user(&task_clone);
            let result = runtime
                .execute(&agent_id_clone, &session_id_clone, message)
                .await;

            // Process result
            let (success, messages, error) = match result {
                Ok(msgs) => (true, msgs, None),
                Err(e) => (false, vec![], Some(e.to_string())),
            };

            // Update agent status
            if let Some(mut agent) = manager_agents.get_mut(&spawn_id_clone) {
                agent.info.status = if success {
                    SpawnedAgentStatus::Completed
                } else {
                    SpawnedAgentStatus::Failed
                };
                agent.info.completed_at = Some(Utc::now());
                agent.info.error = error.clone();

                if let Some(last) = messages.last() {
                    agent.info.last_message = Some(last.content.text());
                }
            }

            // Store result
            let agent_result = AgentResult {
                agent_id: spawn_id_clone.clone(),
                success,
                messages,
                error,
            };

            manager_results.insert(spawn_id_clone.clone(), agent_result);

            info!("Sub-agent {} completed (success={})", spawn_id_clone, success);
        });

        // Start result listener
        let spawn_id_for_listener = spawn_id.clone();
        let agents_for_listener = self.agents.clone();
        tokio::spawn(async move {
            while let Some(result) = result_rx.recv().await {
                debug!("Received result for agent {}: {:?}", spawn_id_for_listener, result);
                if let Some(mut agent) = agents_for_listener.get_mut(&spawn_id_for_listener) {
                    if let Some(last) = result.messages.last() {
                        agent.info.last_message = Some(last.content.text());
                    }
                }
            }
        });

        info!("Spawned sub-agent {} from {}", spawn_id, agent_id);
        Ok(info)
    }

    /// Get the status of a spawned agent.
    pub fn get_status(&self, spawn_id: &str) -> Option<SpawnedAgent> {
        self.agents.get(spawn_id).map(|a| a.info.clone())
    }

    /// Get the result of a completed agent.
    pub fn get_result(&self, spawn_id: &str) -> Option<AgentResult> {
        self.results.remove(spawn_id).map(|(_, r)| r)
    }

    /// Send a message to a running agent.
    pub async fn send_message(
        &self,
        spawn_id: &str,
        message: &str,
    ) -> Result<(), AgentManagerError> {
        let agent = self
            .agents
            .get(spawn_id)
            .ok_or_else(|| AgentManagerError::NotFound(spawn_id.to_string()))?;

        // Check if agent is still running
        if !matches!(
            agent.info.status,
            SpawnedAgentStatus::Running | SpawnedAgentStatus::Idle
        ) {
            return Err(AgentManagerError::Terminated(spawn_id.to_string()));
        }

        // For now, we log the message - full bidirectional communication
        // would require more complex state management
        info!(
            "Message to agent {}: {}",
            spawn_id,
            message
        );

        // In a full implementation, this would:
        // 1. Queue the message for the agent
        // 2. Wake the agent if it's idle
        // 3. Return when the agent acknowledges

        Ok(())
    }

    /// Terminate a running agent.
    pub fn terminate(&self, spawn_id: &str) -> Result<(), AgentManagerError> {
        let mut agent = self
            .agents
            .get_mut(spawn_id)
            .ok_or_else(|| AgentManagerError::NotFound(spawn_id.to_string()))?;

        // Check if already terminated
        if matches!(
            agent.info.status,
            SpawnedAgentStatus::Completed
                | SpawnedAgentStatus::Failed
                | SpawnedAgentStatus::Terminated
        ) {
            warn!("Agent {} already terminated", spawn_id);
            return Ok(());
        }

        // Signal abort
        agent.abort_signal.abort();
        agent.info.status = SpawnedAgentStatus::Terminated;
        agent.info.completed_at = Some(Utc::now());

        info!("Terminated agent {}", spawn_id);
        Ok(())
    }

    /// List all spawned agents.
    pub fn list(&self) -> Vec<SpawnedAgent> {
        self.agents.iter().map(|a| a.info.clone()).collect()
    }

    /// List agents by parent.
    pub fn list_by_parent(&self, parent_id: &str) -> Vec<SpawnedAgent> {
        self.agents
            .iter()
            .filter(|a| a.info.parent_id.as_deref() == Some(parent_id))
            .map(|a| a.info.clone())
            .collect()
    }

    /// Clean up completed agents older than the given duration.
    pub fn cleanup_old(&self, max_age: chrono::Duration) {
        let cutoff = Utc::now() - max_age;
        let to_remove: Vec<String> = self
            .agents
            .iter()
            .filter(|a| {
                matches!(
                    a.info.status,
                    SpawnedAgentStatus::Completed
                        | SpawnedAgentStatus::Failed
                        | SpawnedAgentStatus::Terminated
                ) && a.info.completed_at.map(|t| t < cutoff).unwrap_or(false)
            })
            .map(|a| a.info.id.clone())
            .collect();

        for id in to_remove {
            self.agents.remove(&id);
            self.results.remove(&id);
            debug!("Cleaned up old agent {}", id);
        }
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawned_agent_status_serialize() {
        assert_eq!(
            serde_json::to_string(&SpawnedAgentStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&SpawnedAgentStatus::Completed).unwrap(),
            "\"completed\""
        );
    }

    #[test]
    fn test_spawned_agent_status_deserialize() {
        let status: SpawnedAgentStatus = serde_json::from_str("\"starting\"").unwrap();
        assert_eq!(status, SpawnedAgentStatus::Starting);
    }

    #[test]
    fn test_agent_manager_creation() {
        let manager = AgentManager::new(5);
        assert!(manager.list().is_empty());
    }

    #[test]
    fn test_agent_manager_default() {
        let manager = AgentManager::default();
        assert_eq!(manager.max_concurrent, 10);
    }

    #[test]
    fn test_get_status_not_found() {
        let manager = AgentManager::new(5);
        assert!(manager.get_status("nonexistent").is_none());
    }

    #[test]
    fn test_terminate_not_found() {
        let manager = AgentManager::new(5);
        let result = manager.terminate("nonexistent");
        assert!(matches!(result, Err(AgentManagerError::NotFound(_))));
    }

    #[test]
    fn test_agent_manager_error_display() {
        let err = AgentManagerError::NotFound("agent-1".to_string());
        assert_eq!(err.to_string(), "Agent not found: agent-1");

        let err = AgentManagerError::SpawnFailed("reason".to_string());
        assert_eq!(err.to_string(), "Spawn failed: reason");
    }

    #[test]
    fn test_spawned_agent_serialize() {
        let agent = SpawnedAgent {
            id: "spawn-1".to_string(),
            agent_id: "general".to_string(),
            session_id: "session-1".to_string(),
            parent_id: Some("parent-1".to_string()),
            status: SpawnedAgentStatus::Running,
            task: "test task".to_string(),
            spawned_at: Utc::now(),
            completed_at: None,
            last_message: None,
            error: None,
            tools: vec!["read_file".to_string()],
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&agent).unwrap();
        assert!(json.contains("spawn-1"));
        assert!(json.contains("running"));
    }
}
