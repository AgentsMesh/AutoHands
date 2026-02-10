//! RunLoop bridge for unified task submission.
//!
//! This module provides the bridge between external interfaces (HTTP, WebSocket, Webhook)
//! and the RunLoop task system. All external requests are converted to RunLoop tasks
//! for unified processing.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;

use autohands_runloop::{RunLoop, Task};

use crate::state::AppState;

/// Trait for components that can submit tasks to RunLoop.
pub trait RunLoopBridge: Send + Sync {
    /// Submit a task to the RunLoop.
    fn submit_task(
        &self,
        task_type: &str,
        payload: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<(), crate::error::InterfaceError>> + Send;
}

/// RunLoop-enabled application state extension.
///
/// This provides the necessary components for submitting tasks
/// to the RunLoop from HTTP handlers.
pub struct RunLoopState {
    /// Reference to the RunLoop.
    run_loop: Arc<RunLoop>,
}

impl RunLoopState {
    /// Create a new RunLoop state from a RunLoop instance.
    pub fn from_runloop(run_loop: Arc<RunLoop>) -> Self {
        Self { run_loop }
    }

    /// Get the RunLoop reference.
    pub fn run_loop(&self) -> &Arc<RunLoop> {
        &self.run_loop
    }

    /// Submit a task to the RunLoop.
    pub async fn submit_task(
        &self,
        task_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), crate::error::InterfaceError> {
        let task = Task::new(task_type, payload);
        self.run_loop.inject_task(task).await.map_err(|e| {
            crate::error::InterfaceError::RunLoopInjectionFailed(format!(
                "Failed to inject task: {}",
                e
            ))
        })?;

        // Wake up the RunLoop
        self.run_loop.wakeup(format!("New task: {}", task_type));

        Ok(())
    }
}

/// Request to submit a task via RunLoop.
#[derive(Debug, Deserialize)]
pub struct RunLoopTaskRequest {
    /// The task description for the agent to execute.
    pub task: String,

    /// Optional session ID for correlation.
    pub session_id: Option<String>,

    /// Optional agent ID to use. Defaults to "general".
    pub agent_id: Option<String>,
}

/// Response from submitting a task to RunLoop.
#[derive(Debug, Serialize)]
pub struct RunLoopTaskResponse {
    /// Session ID for tracking this task.
    pub session_id: String,

    /// Status of the submission.
    pub status: String,

    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Submit a task to the RunLoop event queue.
///
/// POST /v1/runloop/task
///
/// This endpoint injects a task into the RunLoop for asynchronous processing.
/// Unlike the direct agent execution endpoint, this returns immediately after
/// the task is queued, without waiting for execution to complete.
pub async fn submit_task(
    State(state): State<Arc<RunLoopState>>,
    Json(req): Json<RunLoopTaskRequest>,
) -> impl IntoResponse {
    let session_id = req
        .session_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let agent_id = req.agent_id;

    info!(
        "RunLoop task submission: session={}, task={}",
        session_id,
        req.task.chars().take(50).collect::<String>()
    );

    // Build task payload
    let payload = serde_json::json!({
        "prompt": req.task,
        "session_id": session_id.clone(),
        "agent_id": agent_id,
    });

    match state.submit_task("agent:execute", payload).await {
        Ok(()) => {
            info!("Task submitted to RunLoop: session={}", session_id);

            (
                StatusCode::ACCEPTED,
                Json(RunLoopTaskResponse {
                    session_id,
                    status: "queued".to_string(),
                    error: None,
                }),
            )
        }
        Err(e) => {
            error!("Failed to submit task to RunLoop: {}", e);

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RunLoopTaskResponse {
                    session_id,
                    status: "error".to_string(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Application state for RunLoop-based execution.
///
/// All external requests go through RunLoop for unified event processing.
/// The "direct mode" has been removed - RunLoop is the only execution path.
pub struct HybridAppState {
    /// Base application state.
    pub base: Arc<AppState>,

    /// RunLoop state for event injection.
    pub runloop: Arc<RunLoopState>,
}

impl HybridAppState {
    /// Create state with RunLoop.
    pub fn new(base: Arc<AppState>, runloop: Arc<RunLoopState>) -> Self {
        Self { base, runloop }
    }

    /// Get the RunLoop state.
    pub fn runloop_state(&self) -> &Arc<RunLoopState> {
        &self.runloop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use autohands_runloop::{RunLoopConfig};

    #[tokio::test]
    async fn test_runloop_state_creation() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let state = RunLoopState::from_runloop(run_loop);
        // Just verify it compiles and creates without panicking
        let _ = state;
    }

    #[test]
    fn test_runloop_task_request_deserialize() {
        let json = r#"{"task": "analyze code", "agent_id": "coder"}"#;
        let req: RunLoopTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task, "analyze code");
        assert_eq!(req.agent_id, Some("coder".to_string()));
        assert!(req.session_id.is_none());
    }

    #[test]
    fn test_runloop_task_response_serialize() {
        let resp = RunLoopTaskResponse {
            session_id: "test-session".to_string(),
            status: "queued".to_string(),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("test-session"));
        assert!(json.contains("queued"));
        assert!(!json.contains("error")); // Should be skipped when None
    }

    #[tokio::test]
    async fn test_hybrid_state_creation() {
        let base = Arc::new(AppState::default());
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let runloop = Arc::new(RunLoopState::from_runloop(run_loop));

        let hybrid = HybridAppState::new(base, runloop);
        assert!(Arc::strong_count(&hybrid.runloop) >= 1);
    }

    #[tokio::test]
    async fn test_runloop_state_submit_task() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let state = RunLoopState::from_runloop(run_loop.clone());

        // Inject a task via RunLoopState
        let result = state
            .submit_task("test:event", serde_json::json!({"data": "test"}))
            .await;
        assert!(result.is_ok());

        // Verify task was added to the RunLoop's queue
        assert_eq!(run_loop.pending_task_count().await, 1);
    }
}
