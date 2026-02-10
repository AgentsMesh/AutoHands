//! Agent list tool.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::manager::{AgentManager, SpawnedAgent, SpawnedAgentStatus};

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
