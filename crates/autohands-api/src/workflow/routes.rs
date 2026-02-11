//! Workflow HTTP route handlers.
//!
//! Provides CRUD operations and execution for workflows:
//! - POST   /workflows       - Create workflow
//! - GET    /workflows       - List workflows
//! - GET    /workflows/{id}  - Get workflow
//! - POST   /workflows/{id}/run - Run workflow
//! - DELETE /workflows/{id}  - Delete workflow

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use tracing::{error, info};

use super::definition::{Workflow, WorkflowExecution};
use crate::runloop_bridge::HybridAppState;

/// Response for workflow operations.
#[derive(Debug, Serialize)]
pub struct WorkflowResponse {
    pub workflow: Workflow,
}

/// Response for listing workflows.
#[derive(Debug, Serialize)]
pub struct WorkflowListResponse {
    pub count: usize,
    pub workflows: Vec<Workflow>,
}

/// Response for workflow execution.
#[derive(Debug, Serialize)]
pub struct WorkflowRunResponse {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub step_results: serde_json::Value,
}

/// Create a new workflow.
///
/// POST /workflows
pub async fn create_workflow(
    State(state): State<Arc<HybridAppState>>,
    Json(workflow): Json<Workflow>,
) -> impl IntoResponse {
    info!("Creating workflow: {} ({})", workflow.id, workflow.name);

    let workflow_store = &state.workflow_store;
    if let Err(e) = workflow_store.save(&workflow).await {
        error!("Failed to save workflow: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        );
    }

    (
        StatusCode::CREATED,
        Json(serde_json::json!(WorkflowResponse { workflow })),
    )
}

/// List all workflows.
///
/// GET /workflows
pub async fn list_workflows(State(state): State<Arc<HybridAppState>>) -> impl IntoResponse {
    let workflow_store = &state.workflow_store;
    match workflow_store.load_all().await {
        Ok(workflows) => {
            let count = workflows.len();
            Json(serde_json::to_value(WorkflowListResponse { count, workflows }).unwrap())
        }
        Err(e) => {
            error!("Failed to list workflows: {}", e);
            Json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

/// Get a workflow by ID.
///
/// GET /workflows/{id}
pub async fn get_workflow(
    State(state): State<Arc<HybridAppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let workflow_store = &state.workflow_store;
    match workflow_store.load(&id).await {
        Ok(Some(workflow)) => (
            StatusCode::OK,
            Json(serde_json::json!(WorkflowResponse { workflow })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Workflow '{}' not found", id)})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

/// Run a workflow.
///
/// POST /workflows/{id}/run
pub async fn run_workflow(
    State(state): State<Arc<HybridAppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let workflow_store = &state.workflow_store;

    // Load the workflow
    let workflow = match workflow_store.load(&id).await {
        Ok(Some(wf)) => wf,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Workflow '{}' not found", id)})),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            );
        }
    };

    info!("Running workflow: {} ({})", workflow.id, workflow.name);

    let mut execution = WorkflowExecution::new(&workflow.id);
    let execution_id = execution.id.to_string();

    match state
        .workflow_executor
        .execute_workflow(&workflow, &mut execution)
        .await
    {
        Ok(_context) => (
            StatusCode::OK,
            Json(serde_json::json!(WorkflowRunResponse {
                execution_id,
                workflow_id: workflow.id,
                status: format!("{:?}", execution.state),
                error: execution.error,
                step_results: execution.step_results,
            })),
        ),
        Err(e) => {
            error!("Workflow execution failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!(WorkflowRunResponse {
                    execution_id,
                    workflow_id: workflow.id,
                    status: format!("{:?}", execution.state),
                    error: Some(e.to_string()),
                    step_results: execution.step_results,
                })),
            )
        }
    }
}

/// Delete a workflow.
///
/// DELETE /workflows/{id}
pub async fn delete_workflow(
    State(state): State<Arc<HybridAppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!("Deleting workflow: {}", id);

    let workflow_store = &state.workflow_store;
    match workflow_store.delete(&id).await {
        Ok(true) => StatusCode::NO_CONTENT,
        Ok(false) => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
#[path = "routes_tests.rs"]
mod tests;
