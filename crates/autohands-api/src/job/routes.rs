//! Job HTTP route handlers.
//!
//! Provides CRUD operations for jobs:
//! - POST   /jobs       - Create job
//! - GET    /jobs       - List jobs
//! - GET    /jobs/{id}  - Get job
//! - DELETE /jobs/{id}  - Delete job

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use tracing::{error, info};

use super::definition::{Job, JobDefinition};
use crate::runloop_bridge::HybridAppState;

/// Response for listing jobs.
#[derive(Debug, Serialize)]
pub struct JobListResponse {
    pub count: usize,
    pub jobs: Vec<Job>,
}

/// Response for single job.
#[derive(Debug, Serialize)]
pub struct JobResponse {
    pub job: Job,
}

/// Create a new job.
///
/// POST /jobs
pub async fn create_job(
    State(state): State<Arc<HybridAppState>>,
    Json(definition): Json<JobDefinition>,
) -> impl IntoResponse {
    info!("Creating job: {} (schedule: {})", definition.id, definition.schedule);

    let job = Job::new(definition);
    let job_store = &state.job_store;

    if let Err(e) = job_store.save(&job).await {
        error!("Failed to save job: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        );
    }

    (
        StatusCode::CREATED,
        Json(serde_json::json!(JobResponse { job })),
    )
}

/// List all jobs.
///
/// GET /jobs
pub async fn list_jobs(State(state): State<Arc<HybridAppState>>) -> impl IntoResponse {
    let job_store = &state.job_store;
    match job_store.load_all().await {
        Ok(jobs) => {
            let count = jobs.len();
            Json(serde_json::to_value(JobListResponse { count, jobs }).unwrap())
        }
        Err(e) => {
            error!("Failed to list jobs: {}", e);
            Json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

/// Get a job by ID.
///
/// GET /jobs/{id}
pub async fn get_job(
    State(state): State<Arc<HybridAppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let job_store = &state.job_store;
    match job_store.load(&id).await {
        Ok(Some(job)) => (
            StatusCode::OK,
            Json(serde_json::json!(JobResponse { job })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Job '{}' not found", id)})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

/// Delete a job.
///
/// DELETE /jobs/{id}
pub async fn delete_job(
    State(state): State<Arc<HybridAppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!("Deleting job: {}", id);

    let job_store = &state.job_store;
    match job_store.delete(&id).await {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(e) => {
            error!("Failed to delete job: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[cfg(test)]
#[path = "routes_tests.rs"]
mod tests;
