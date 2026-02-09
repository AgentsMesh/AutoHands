//! Sub-agent management tools.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

use crate::manager::{AgentManager, SpawnedAgent, SpawnedAgentStatus};

// ============================================================================
// Agent Spawn Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AgentSpawnParams {
    /// ID of the registered agent to spawn.
    pub agent_id: String,
    /// Task description for the agent.
    pub task: String,
    /// Optional list of tools to make available (defaults to agent's configured tools).
    #[serde(default)]
    pub tools: Vec<String>,
    /// Optional model override.
    pub model: Option<String>,
    /// Custom metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct AgentSpawnResult {
    pub spawn_id: String,
    pub agent_id: String,
    pub session_id: String,
    pub status: SpawnedAgentStatus,
    pub message: String,
}

/// Spawn a new sub-agent to handle a task.
pub struct AgentSpawnTool {
    definition: ToolDefinition,
    manager: Arc<AgentManager>,
}

impl AgentSpawnTool {
    pub fn new(manager: Arc<AgentManager>) -> Self {
        let mut definition = ToolDefinition::new(
            "agent_spawn",
            "Agent Spawn",
            "Spawn a new sub-agent to handle a specific task. The sub-agent runs \
             asynchronously and can be monitored or terminated later.",
        );
        definition.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "agent_id": {
                    "type": "string",
                    "description": "ID of the registered agent type to spawn (e.g., 'general', 'researcher')"
                },
                "task": {
                    "type": "string",
                    "description": "Task description for the sub-agent to accomplish"
                },
                "tools": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional list of tool IDs to make available to the agent"
                },
                "model": {
                    "type": "string",
                    "description": "Optional model override for this agent instance"
                },
                "metadata": {
                    "type": "object",
                    "description": "Custom metadata to attach to this agent instance"
                }
            },
            "required": ["agent_id", "task"]
        }));
        definition.risk_level = RiskLevel::Medium;

        Self { definition, manager }
    }
}

#[async_trait]
impl Tool for AgentSpawnTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: AgentSpawnParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let mut metadata = params.metadata;
        if let Some(model) = params.model {
            metadata.insert("model_override".to_string(), serde_json::json!(model));
        }

        let spawned = self
            .manager
            .spawn(
                &params.agent_id,
                &params.task,
                ctx.data.get("agent_id").and_then(|v| v.as_str()),
                params.tools,
                metadata,
            )
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Spawned agent {} for task: {}", spawned.id, params.task);

        let result = AgentSpawnResult {
            spawn_id: spawned.id.clone(),
            agent_id: spawned.agent_id,
            session_id: spawned.session_id,
            status: spawned.status,
            message: format!(
                "Sub-agent '{}' spawned successfully. Use agent_status with spawn_id='{}' to check progress.",
                spawned.id, spawned.id
            ),
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap())
            .with_metadata("spawn_id", serde_json::json!(spawned.id)))
    }
}

// ============================================================================
// Agent Status Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AgentStatusParams {
    /// ID of the spawned agent instance.
    pub spawn_id: String,
    /// Whether to include the result if completed.
    #[serde(default = "default_include_result")]
    pub include_result: bool,
}

fn default_include_result() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct AgentStatusResult {
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<SpawnedAgent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

/// Query the status of a spawned agent.
pub struct AgentStatusTool {
    definition: ToolDefinition,
    manager: Arc<AgentManager>,
}

impl AgentStatusTool {
    pub fn new(manager: Arc<AgentManager>) -> Self {
        let mut definition = ToolDefinition::new(
            "agent_status",
            "Agent Status",
            "Query the status of a spawned sub-agent. Returns current status, \
             progress information, and result if completed.",
        );
        definition.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "spawn_id": {
                    "type": "string",
                    "description": "ID of the spawned agent instance"
                },
                "include_result": {
                    "type": "boolean",
                    "description": "Whether to include the result if the agent has completed (default: true)"
                }
            },
            "required": ["spawn_id"]
        }));

        Self { definition, manager }
    }
}

#[async_trait]
impl Tool for AgentStatusTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: AgentStatusParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let agent = self.manager.get_status(&params.spawn_id);

        let result_text = if params.include_result && agent.as_ref().map(|a| {
            matches!(a.status, SpawnedAgentStatus::Completed | SpawnedAgentStatus::Failed)
        }).unwrap_or(false) {
            self.manager.get_result(&params.spawn_id).map(|r| {
                r.messages.last().map(|m| m.content.text()).unwrap_or_default()
            })
        } else {
            None
        };

        let result = AgentStatusResult {
            found: agent.is_some(),
            agent,
            result: result_text,
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap()))
    }
}

// ============================================================================
// Agent Message Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AgentMessageParams {
    /// ID of the spawned agent instance.
    pub spawn_id: String,
    /// Message to send to the agent.
    pub message: String,
}

/// Send a message to a running agent.
pub struct AgentMessageTool {
    definition: ToolDefinition,
    manager: Arc<AgentManager>,
}

impl AgentMessageTool {
    pub fn new(manager: Arc<AgentManager>) -> Self {
        let mut definition = ToolDefinition::new(
            "agent_message",
            "Agent Message",
            "Send a message or instruction to a running sub-agent. Use this to \
             provide additional context or redirect the agent's work.",
        );
        definition.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "spawn_id": {
                    "type": "string",
                    "description": "ID of the spawned agent instance"
                },
                "message": {
                    "type": "string",
                    "description": "Message or instruction to send to the agent"
                }
            },
            "required": ["spawn_id", "message"]
        }));
        definition.risk_level = RiskLevel::Low;

        Self { definition, manager }
    }
}

#[async_trait]
impl Tool for AgentMessageTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: AgentMessageParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        self.manager
            .send_message(&params.spawn_id, &params.message)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success(format!(
            "Message sent to agent {}",
            params.spawn_id
        )))
    }
}

// ============================================================================
// Agent Terminate Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AgentTerminateParams {
    /// ID of the spawned agent instance.
    pub spawn_id: String,
    /// Reason for termination (for logging).
    pub reason: Option<String>,
}

/// Terminate a running agent.
pub struct AgentTerminateTool {
    definition: ToolDefinition,
    manager: Arc<AgentManager>,
}

impl AgentTerminateTool {
    pub fn new(manager: Arc<AgentManager>) -> Self {
        let mut definition = ToolDefinition::new(
            "agent_terminate",
            "Agent Terminate",
            "Terminate a running sub-agent. Use this to stop an agent that is \
             no longer needed or is taking too long.",
        );
        definition.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "spawn_id": {
                    "type": "string",
                    "description": "ID of the spawned agent instance to terminate"
                },
                "reason": {
                    "type": "string",
                    "description": "Optional reason for termination (for logging)"
                }
            },
            "required": ["spawn_id"]
        }));
        definition.risk_level = RiskLevel::Medium;

        Self { definition, manager }
    }
}

#[async_trait]
impl Tool for AgentTerminateTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: AgentTerminateParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        if let Some(reason) = &params.reason {
            debug!("Terminating agent {} (reason: {})", params.spawn_id, reason);
        }

        self.manager
            .terminate(&params.spawn_id)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success(format!(
            "Agent {} terminated{}",
            params.spawn_id,
            params.reason.map(|r| format!(" ({})", r)).unwrap_or_default()
        )))
    }
}

// ============================================================================
// Agent List Tool
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AgentListParams {
    /// Filter by status.
    pub status: Option<SpawnedAgentStatus>,
    /// Filter by parent ID.
    pub parent_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AgentListResult {
    pub count: usize,
    pub agents: Vec<SpawnedAgent>,
}

/// List all spawned agents.
pub struct AgentListTool {
    definition: ToolDefinition,
    manager: Arc<AgentManager>,
}

impl AgentListTool {
    pub fn new(manager: Arc<AgentManager>) -> Self {
        let mut definition = ToolDefinition::new(
            "agent_list",
            "Agent List",
            "List all spawned sub-agents, optionally filtered by status or parent.",
        );
        definition.parameters_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["starting", "running", "idle", "completed", "failed", "terminated"],
                    "description": "Filter agents by status"
                },
                "parent_id": {
                    "type": "string",
                    "description": "Filter agents spawned by a specific parent"
                }
            }
        }));

        Self { definition, manager }
    }
}

#[async_trait]
impl Tool for AgentListTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: AgentListParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid params: {}", e)))?;

        let mut agents = if let Some(ref parent_id) = params.parent_id {
            self.manager.list_by_parent(parent_id)
        } else {
            self.manager.list()
        };

        // Filter by status if specified
        if let Some(status) = params.status {
            agents.retain(|a| a.status == status);
        }

        let result = AgentListResult {
            count: agents.len(),
            agents,
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result).unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_params_deserialize() {
        let json = r#"{"agent_id": "general", "task": "research AI"}"#;
        let params: AgentSpawnParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.agent_id, "general");
        assert_eq!(params.task, "research AI");
        assert!(params.tools.is_empty());
    }

    #[test]
    fn test_spawn_params_with_tools() {
        let json = r#"{"agent_id": "general", "task": "test", "tools": ["read_file", "write_file"]}"#;
        let params: AgentSpawnParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.tools.len(), 2);
    }

    #[test]
    fn test_status_params_deserialize() {
        let json = r#"{"spawn_id": "spawn-123"}"#;
        let params: AgentStatusParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.spawn_id, "spawn-123");
        assert!(params.include_result); // default
    }

    #[test]
    fn test_message_params_deserialize() {
        let json = r#"{"spawn_id": "spawn-123", "message": "update progress"}"#;
        let params: AgentMessageParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.spawn_id, "spawn-123");
        assert_eq!(params.message, "update progress");
    }

    #[test]
    fn test_terminate_params_deserialize() {
        let json = r#"{"spawn_id": "spawn-123", "reason": "no longer needed"}"#;
        let params: AgentTerminateParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.spawn_id, "spawn-123");
        assert_eq!(params.reason, Some("no longer needed".to_string()));
    }

    #[test]
    fn test_list_params_deserialize() {
        let json = r#"{"status": "running"}"#;
        let params: AgentListParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.status, Some(SpawnedAgentStatus::Running));
    }

    #[test]
    fn test_spawn_result_serialize() {
        let result = AgentSpawnResult {
            spawn_id: "spawn-123".to_string(),
            agent_id: "general".to_string(),
            session_id: "session-456".to_string(),
            status: SpawnedAgentStatus::Starting,
            message: "spawned".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("spawn-123"));
        assert!(json.contains("starting"));
    }

    #[test]
    fn test_list_result_serialize() {
        let result = AgentListResult {
            count: 0,
            agents: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"count\":0"));
    }
}
