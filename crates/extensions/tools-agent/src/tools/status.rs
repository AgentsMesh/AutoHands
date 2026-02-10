//! Agent status tool.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};

use crate::manager::{AgentManager, SpawnedAgent, SpawnedAgentStatus};

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
