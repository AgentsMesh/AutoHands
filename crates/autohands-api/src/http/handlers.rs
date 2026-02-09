//! Agent API handlers.
//!
//! Provides HTTP endpoints for executing agents and managing their lifecycle.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;

use autohands_protocols::agent::AgentConfig;
use autohands_protocols::types::Message;

use crate::state::AppState;

/// Request to run an agent task.
#[derive(Debug, Deserialize)]
pub struct AgentRunRequest {
    /// The task description for the agent to execute.
    pub task: String,

    /// Optional model to use (e.g., "ark:doubao-seed-1-8-251228").
    pub model: Option<String>,

    /// Optional session ID for continuing a conversation.
    pub session_id: Option<String>,

    /// Optional agent ID to use. Defaults to "general".
    pub agent_id: Option<String>,
}

/// Response from running an agent.
#[derive(Debug, Serialize)]
pub struct AgentRunResponse {
    /// Session ID for this execution.
    pub session_id: String,

    /// Messages generated during execution.
    pub messages: Vec<MessageResponse>,

    /// Execution status.
    pub status: String,

    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A message in the response.
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    /// Message role (user, assistant, tool).
    pub role: String,

    /// Message content.
    pub content: String,
}

impl From<&Message> for MessageResponse {
    fn from(msg: &Message) -> Self {
        Self {
            role: format!("{:?}", msg.role).to_lowercase(),
            content: msg.content.text().to_string(),
        }
    }
}

/// Request to abort an agent execution.
#[derive(Debug, Deserialize)]
pub struct AgentAbortRequest {
    /// Session ID to abort.
    pub session_id: String,
}

/// Response from aborting an agent.
#[derive(Debug, Serialize)]
pub struct AgentAbortResponse {
    /// Whether the abort was successful.
    pub success: bool,

    /// Message describing the result.
    pub message: String,
}

/// Agent status response.
#[derive(Debug, Serialize)]
pub struct AgentStatusResponse {
    /// Session ID.
    pub session_id: String,

    /// Whether the agent is currently running.
    pub is_running: bool,
}

/// Tool information.
#[derive(Debug, Serialize)]
pub struct ToolInfo {
    /// Tool name.
    pub name: String,

    /// Tool description.
    pub description: String,
}

/// List of available tools.
#[derive(Debug, Serialize)]
pub struct ToolsListResponse {
    /// Total number of tools.
    pub count: usize,

    /// Available tools.
    pub tools: Vec<ToolInfo>,
}

/// List of registered agents.
#[derive(Debug, Serialize)]
pub struct AgentsListResponse {
    /// Total number of agents.
    pub count: usize,

    /// Registered agents.
    pub agents: Vec<AgentInfo>,
}

/// Agent information.
#[derive(Debug, Serialize)]
pub struct AgentInfo {
    /// Agent ID.
    pub id: String,

    /// Agent name.
    pub name: String,

    /// Default model.
    pub default_model: String,

    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl From<&AgentConfig> for AgentInfo {
    fn from(config: &AgentConfig) -> Self {
        Self {
            id: config.id.clone(),
            name: config.name.clone(),
            default_model: config.default_model.clone(),
            description: config.system_prompt.clone(),
        }
    }
}

/// Run an agent to execute a task.
///
/// POST /tasks
pub async fn agent_run(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AgentRunRequest>,
) -> impl IntoResponse {
    info!("Agent run request: task={}", req.task);

    let session_id = req
        .session_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let agent_id = req.agent_id.unwrap_or_else(|| "general".to_string());

    // Check if agent exists
    let _agent = match state.agent_runtime.get_agent(&agent_id) {
        Some(a) => a,
        None => {
            // If no agent registered, return error with helpful message
            let agents = state.agent_runtime.list_agents();
            let available: Vec<_> = agents.iter().map(|a| a.id.as_str()).collect();

            return (
                StatusCode::NOT_FOUND,
                Json(AgentRunResponse {
                    session_id,
                    messages: vec![],
                    status: "error".to_string(),
                    error: Some(format!(
                        "Agent '{}' not found. Available agents: {:?}",
                        agent_id, available
                    )),
                }),
            );
        }
    };

    // Create user message
    let message = Message::user(&req.task);

    // Get transcript writer for this session
    let transcript = match state.transcript_manager.get_writer(&session_id).await {
        Ok(writer) => {
            // Record session start
            if let Err(e) = writer.record_session_start(Some(&req.task)).await {
                tracing::warn!("Failed to record session start: {}", e);
            }
            Some(writer)
        }
        Err(e) => {
            tracing::warn!("Failed to create transcript writer: {}", e);
            None
        }
    };

    // Execute agent with transcript
    match state
        .agent_runtime
        .execute_with_transcript(&agent_id, &session_id, message, transcript.clone())
        .await
    {
        Ok(messages) => {
            let msg_responses: Vec<MessageResponse> = messages.iter().map(|m| m.into()).collect();

            info!(
                "Agent execution completed: session={}, messages={}",
                session_id,
                msg_responses.len()
            );

            (
                StatusCode::OK,
                Json(AgentRunResponse {
                    session_id,
                    messages: msg_responses,
                    status: "completed".to_string(),
                    error: None,
                }),
            )
        }
        Err(e) => {
            error!("Agent execution failed: {}", e);

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AgentRunResponse {
                    session_id,
                    messages: vec![],
                    status: "error".to_string(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Get agent execution status.
///
/// GET /tasks/{session_id}
pub async fn agent_status(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let is_running = state.agent_runtime.is_running(&session_id);

    Json(AgentStatusResponse {
        session_id,
        is_running,
    })
}

/// Abort an agent execution.
///
/// POST /tasks/{session_id}/abort
pub async fn agent_abort(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AgentAbortRequest>,
) -> impl IntoResponse {
    info!("Agent abort request: session={}", req.session_id);

    let success = state.agent_runtime.abort(&req.session_id);

    let message = if success {
        format!("Agent execution {} aborted", req.session_id)
    } else {
        format!("No running agent found for session {}", req.session_id)
    };

    Json(AgentAbortResponse { success, message })
}

/// List available tools.
///
/// GET /tools
pub async fn list_tools(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tool_defs = state.tool_registry.list();
    let tools: Vec<ToolInfo> = tool_defs
        .iter()
        .map(|def| ToolInfo {
            name: def.name.clone(),
            description: def.description.clone(),
        })
        .collect();

    Json(ToolsListResponse {
        count: tools.len(),
        tools,
    })
}

/// List registered agents.
///
/// GET /agents
pub async fn list_agents(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let agent_configs = state.agent_runtime.list_agents();
    let agents: Vec<AgentInfo> = agent_configs.iter().map(|c| c.into()).collect();

    Json(AgentsListResponse {
        count: agents.len(),
        agents,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_response_from() {
        let msg = Message::user("Hello");
        let resp: MessageResponse = (&msg).into();
        assert_eq!(resp.role, "user");
        assert_eq!(resp.content, "Hello");
    }

    #[test]
    fn test_agent_run_request_deserialize() {
        let json = r#"{"task": "list files", "model": "test-model"}"#;
        let req: AgentRunRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task, "list files");
        assert_eq!(req.model, Some("test-model".to_string()));
        assert!(req.session_id.is_none());
    }

    #[test]
    fn test_agent_run_response_serialize() {
        let resp = AgentRunResponse {
            session_id: "test-session".to_string(),
            messages: vec![],
            status: "completed".to_string(),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("test-session"));
        assert!(json.contains("completed"));
        assert!(!json.contains("error")); // Should be skipped when None
    }

    #[test]
    fn test_tools_list_response_serialize() {
        let resp = ToolsListResponse {
            count: 2,
            tools: vec![
                ToolInfo {
                    name: "read_file".to_string(),
                    description: "Read a file".to_string(),
                },
                ToolInfo {
                    name: "write_file".to_string(),
                    description: "Write a file".to_string(),
                },
            ],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("read_file"));
        assert!(json.contains("write_file"));
    }

    #[test]
    fn test_agent_info_from_config() {
        let config = AgentConfig::new("test-agent", "Test Agent", "test-model");
        let info: AgentInfo = (&config).into();
        assert_eq!(info.id, "test-agent");
        assert_eq!(info.name, "Test Agent");
        assert_eq!(info.default_model, "test-model");
    }
}
