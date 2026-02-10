//! Agent terminate tool.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use autohands_protocols::error::ToolError;
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::RiskLevel;

use crate::manager::AgentManager;

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
