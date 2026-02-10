//! Mock agent executor for testing.

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::InterfaceError;

use super::executor_types::{AgentExecutor, ExecutionContext};

/// Mock agent executor that returns pre-configured responses.
pub struct MockAgentExecutor {
    responses: RwLock<HashMap<String, serde_json::Value>>,
}

impl MockAgentExecutor {
    pub fn new() -> Self {
        Self {
            responses: RwLock::new(HashMap::new()),
        }
    }

    pub async fn set_response(&self, agent: &str, response: serde_json::Value) {
        self.responses
            .write()
            .await
            .insert(agent.to_string(), response);
    }
}

impl Default for MockAgentExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentExecutor for MockAgentExecutor {
    async fn execute(
        &self,
        agent: &str,
        prompt: &str,
        _context: &ExecutionContext,
    ) -> Result<serde_json::Value, InterfaceError> {
        let responses = self.responses.read().await;

        if let Some(response) = responses.get(agent) {
            Ok(response.clone())
        } else {
            Ok(serde_json::json!({
                "agent": agent,
                "prompt": prompt,
                "status": "completed",
            }))
        }
    }
}
