//! Tool trait definition.

use async_trait::async_trait;

use super::{ToolContext, ToolDefinition, ToolResult};
use crate::error::ToolError;
use crate::types::RiskLevel;

/// Core trait for tools.
///
/// Tools are executable units that agents can invoke to perform actions.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the tool definition.
    fn definition(&self) -> &ToolDefinition;

    /// Execute the tool with the given parameters.
    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError>;

    /// Validate the parameters before execution.
    fn validate(&self, params: &serde_json::Value) -> Result<(), ToolError> {
        let definition = self.definition();
        if let Some(schema) = &definition.parameters_schema {
            if schema.get("type") == Some(&serde_json::json!("object")) && !params.is_object() {
                return Err(ToolError::ValidationFailed(
                    "Parameters must be an object".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Returns the risk level of this tool.
    fn risk_level(&self) -> RiskLevel {
        self.definition().risk_level
    }
}

#[cfg(test)]
#[path = "traits_tests.rs"]
mod tests;
