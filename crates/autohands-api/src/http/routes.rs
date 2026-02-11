//! HTTP route definitions.
//!
//! Interface provides core task capabilities:
//! - Task submission, status query, abort
//! - Webhook event triggers
//! - WebSocket real-time communication
//!
//! All routes require RunLoop integration. There is no "direct mode" anymore.

use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::http::admin;
use crate::http::handlers::{agent_abort, agent_run, agent_status};
use crate::http::monitoring;
use crate::job::routes as job_routes;
use crate::runloop_bridge::{self, HybridAppState};
use crate::webhook::{
    delete_webhook, get_webhook, handle_github_webhook, handle_webhook, list_webhooks,
    register_webhook,
};
use crate::websocket::ws_handler_with_runloop;
use crate::workflow::routes as workflow_routes;

/// Create the main router with HybridAppState for RunLoop support.
///
/// ## Route Structure
///
/// ```text
/// /tasks
///   POST   /tasks          - Submit task (sync, backward compat)
///   GET    /tasks/{id}     - Query task status
///   POST   /tasks/{id}/abort - Abort task
///
/// /v1/runloop
///   POST   /v1/runloop/task - Submit task via RunLoop (async)
///
/// /webhook
///   GET    /webhook/list   - List registered webhooks
///   POST   /webhook/register - Register new webhook
///   POST   /webhook/github - GitHub webhook (injects to RunLoop)
///   GET    /webhook/{id}   - Get webhook details
///   POST   /webhook/{id}   - Trigger webhook (injects to RunLoop)
///   DELETE /webhook/{id}   - Delete webhook
///
/// /admin
///   GET    /admin/extensions      - List extensions
///   GET    /admin/extensions/{id} - Get extension details
///   GET    /admin/sessions        - List sessions
///   GET    /admin/sessions/{id}   - Get session details
///   DELETE /admin/sessions/{id}   - Delete session
///   GET    /admin/stats           - System statistics
///   POST   /admin/reload          - Reload configuration
///   POST   /admin/shutdown        - Graceful shutdown
///
/// /workflows
///   POST   /workflows           - Create workflow
///   GET    /workflows           - List workflows
///   GET    /workflows/{id}      - Get workflow
///   POST   /workflows/{id}/run  - Run workflow
///   DELETE /workflows/{id}      - Delete workflow
///
/// /jobs
///   POST   /jobs       - Create job
///   GET    /jobs       - List jobs
///   GET    /jobs/{id}  - Get job
///   DELETE /jobs/{id}  - Delete job
///
/// /health  - Detailed health check
/// /metrics - Prometheus metrics
/// /livez   - Liveness probe (Kubernetes)
/// /readyz  - Readiness probe (Kubernetes)
///
/// /ws      - WebSocket connection (injects to RunLoop)
/// ```
pub fn create_router_with_hybrid_state(state: Arc<HybridAppState>) -> Router {
    // Task routes need AppState for agent_runtime access (backward compat)
    let task_routes = Router::new()
        .route("/", post(agent_run))
        .route("/{session_id}", get(agent_status))
        .route("/{session_id}/abort", post(agent_abort))
        .with_state(state.base.clone());

    // RunLoop route group for async task submission
    let runloop_routes = Router::new()
        .route("/task", post(runloop_bridge::submit_task))
        .with_state(state.runloop.clone());

    // Webhook routes use HybridAppState for RunLoop integration
    let webhook_routes = Router::new()
        .route("/list", get(list_webhooks))
        .route("/register", post(register_webhook))
        .route("/github", post(handle_github_webhook))
        .route("/{id}", get(get_webhook))
        .route("/{id}", post(handle_webhook))
        .route("/{id}", delete(delete_webhook))
        .with_state(state.clone());

    // Admin routes for extension/session management
    let admin_routes = Router::new()
        .route("/extensions", get(admin::list_extensions))
        .route("/extensions/{id}", get(admin::get_extension))
        .route("/sessions", get(admin::list_sessions))
        .route("/sessions/{id}", get(admin::get_session))
        .route("/sessions/{id}", delete(admin::delete_session))
        .route("/stats", get(admin::system_stats))
        .route("/reload", post(admin::reload_config))
        .route("/shutdown", post(admin::shutdown))
        .with_state(state.base.clone());

    // Monitoring routes (health, metrics, probes)
    let monitoring_routes = Router::new()
        .route("/health", get(monitoring::health_check_detailed))
        .route("/metrics", get(monitoring::prometheus_metrics))
        .route("/readyz", get(monitoring::readiness_probe))
        .with_state(state.base.clone());

    // Liveness probe has no state dependency
    let liveness_route = Router::new()
        .route("/livez", get(monitoring::liveness_probe));

    // Workflow routes for workflow CRUD and execution
    let workflow_router = Router::new()
        .route("/", post(workflow_routes::create_workflow))
        .route("/", get(workflow_routes::list_workflows))
        .route("/{id}", get(workflow_routes::get_workflow))
        .route("/{id}/run", post(workflow_routes::run_workflow))
        .route("/{id}", delete(workflow_routes::delete_workflow))
        .with_state(state.clone());

    // Job routes for job CRUD
    let job_router = Router::new()
        .route("/", post(job_routes::create_job))
        .route("/", get(job_routes::list_jobs))
        .route("/{id}", get(job_routes::get_job))
        .route("/{id}", delete(job_routes::delete_job))
        .with_state(state.clone());

    // WebSocket route uses HybridAppState for RunLoop integration
    let ws_route = Router::new()
        .route("/ws", get(ws_handler_with_runloop))
        .with_state(state);

    // Combine all routes
    Router::new()
        .nest("/tasks", task_routes)
        .nest("/v1/runloop", runloop_routes)
        .nest("/webhook", webhook_routes)
        .nest("/workflows", workflow_router)
        .nest("/jobs", job_router)
        .nest("/admin", admin_routes)
        .merge(monitoring_routes)
        .merge(liveness_route)
        .merge(ws_route)
}

#[cfg(test)]
#[path = "routes_tests.rs"]
mod tests;
