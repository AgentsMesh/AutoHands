//! Agent message tool.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

use crate::manager::AgentManager;

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
