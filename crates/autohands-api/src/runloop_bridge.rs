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
        reply_to: Option<autohands_protocols::channel::ReplyAddress>,
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
    ///
    /// If `reply_to` is provided, the RunLoop will route the agent's response
    /// back through the ChannelRegistry to the specified reply address.
    pub async fn submit_task(
        &self,
        task_type: &str,
        payload: serde_json::Value,
        reply_to: Option<autohands_protocols::channel::ReplyAddress>,
    ) -> Result<(), crate::error::InterfaceError> {
        let mut task = Task::new(task_type, payload);
        if let Some(addr) = reply_to {
            task = task.with_reply_to(addr);
        }

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

    match state.submit_task("agent:execute", payload, None).await {
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

    /// API WebSocket channel for routing responses to WebSocket connections.
    pub api_ws_channel: Arc<crate::websocket::ApiWsChannel>,

    /// Webhook registry for managing webhook registrations.
    pub webhook_registry: Arc<crate::webhook::WebhookRegistry>,

    /// Workflow executor for running workflows.
    pub workflow_executor: Arc<crate::workflow::WorkflowExecutor>,

    /// Workflow store for persistence.
    pub workflow_store: Arc<dyn crate::workflow::WorkflowStore>,

    /// Job store for persistence.
    pub job_store: Arc<dyn crate::job::JobStore>,
}

impl HybridAppState {
    /// Create state with RunLoop.
    pub fn new(
        base: Arc<AppState>,
        runloop: Arc<RunLoopState>,
        api_ws_channel: Arc<crate::websocket::ApiWsChannel>,
    ) -> Self {
        let workflow_executor = Arc::new(
            crate::workflow::WorkflowExecutor::new(
                Arc::new(crate::workflow::MockAgentExecutor::new()),
            ),
        );
        let workflow_store: Arc<dyn crate::workflow::WorkflowStore> =
            Arc::new(crate::workflow::MemoryWorkflowStore::new());
        let job_store: Arc<dyn crate::job::JobStore> =
            Arc::new(crate::job::MemoryJobStore::new());

        Self {
            base,
            runloop,
            api_ws_channel,
            webhook_registry: Arc::new(crate::webhook::WebhookRegistry::new()),
            workflow_executor,
            workflow_store,
            job_store,
        }
    }

    /// Create state with explicit workflow and job components.
    pub fn with_components(
        base: Arc<AppState>,
        runloop: Arc<RunLoopState>,
        api_ws_channel: Arc<crate::websocket::ApiWsChannel>,
        webhook_registry: Arc<crate::webhook::WebhookRegistry>,
        workflow_executor: Arc<crate::workflow::WorkflowExecutor>,
        workflow_store: Arc<dyn crate::workflow::WorkflowStore>,
        job_store: Arc<dyn crate::job::JobStore>,
    ) -> Self {
        Self {
            base,
            runloop,
            api_ws_channel,
            webhook_registry,
            workflow_executor,
            workflow_store,
            job_store,
        }
    }

    /// Get the RunLoop state.
    pub fn runloop_state(&self) -> &Arc<RunLoopState> {
        &self.runloop
    }

    /// Get the webhook registry.
    pub fn webhook_registry(&self) -> &Arc<crate::webhook::WebhookRegistry> {
        &self.webhook_registry
    }
}

#[cfg(test)]
#[path = "runloop_bridge_tests.rs"]
mod tests;
