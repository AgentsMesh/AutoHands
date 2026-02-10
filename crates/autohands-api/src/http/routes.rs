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

use crate::http::handlers::{agent_abort, agent_run, agent_status};
use crate::runloop_bridge::HybridAppState;
use crate::webhook::{
    delete_webhook, get_webhook, handle_github_webhook, handle_webhook, list_webhooks,
    register_webhook,
};
use crate::websocket::ws_handler_with_runloop;

/// Create the main router with HybridAppState for RunLoop support.
///
/// ## Route Structure
///
/// ```text
/// /tasks
///   POST   /tasks          - Submit task
///   GET    /tasks/{id}     - Query task status
///   POST   /tasks/{id}/abort - Abort task
///
/// /webhook
///   GET    /webhook/list   - List registered webhooks
///   POST   /webhook/register - Register new webhook
///   POST   /webhook/github - GitHub webhook (injects to RunLoop)
///   GET    /webhook/{id}   - Get webhook details
///   POST   /webhook/{id}   - Trigger webhook (injects to RunLoop)
///   DELETE /webhook/{id}   - Delete webhook
///
/// /ws                      - WebSocket connection (injects to RunLoop)
/// ```
pub fn create_router_with_hybrid_state(state: Arc<HybridAppState>) -> Router {
    // Task routes need AppState for agent_runtime access
    // They are mounted separately with their own state
    let task_routes = Router::new()
        .route("/", post(agent_run))
        .route("/{session_id}", get(agent_status))
        .route("/{session_id}/abort", post(agent_abort))
        .with_state(state.base.clone());

    // Webhook routes use HybridAppState for RunLoop integration
    let webhook_routes = Router::new()
        .route("/list", get(list_webhooks))
        .route("/register", post(register_webhook))
        .route("/github", post(handle_github_webhook))
        .route("/{id}", get(get_webhook))
        .route("/{id}", post(handle_webhook))
        .route("/{id}", delete(delete_webhook))
        .with_state(state.clone());

    // WebSocket route uses HybridAppState for RunLoop integration
    let ws_route = Router::new()
        .route("/ws", get(ws_handler_with_runloop))
        .with_state(state);

    // Combine all routes
    Router::new()
        .nest("/tasks", task_routes)
        .nest("/webhook", webhook_routes)
        .merge(ws_route)
}

#[cfg(test)]
#[path = "routes_tests.rs"]
mod tests;
