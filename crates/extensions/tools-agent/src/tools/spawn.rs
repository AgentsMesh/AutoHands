//! Agent spawn tool.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

use crate::manager::{AgentManager, SpawnedAgentStatus};

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
