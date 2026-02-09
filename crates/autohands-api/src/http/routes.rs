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
mod tests {
    use super::*;
    use crate::runloop_bridge::RunLoopState;
    use autohands_runloop::{TaskQueue, TaskQueueConfig};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tokio::sync::mpsc;
    use tower::ServiceExt;

    fn create_test_router() -> Router {
        let base = Arc::new(AppState::default());
        let (tx, _rx) = mpsc::channel(16);
        let config = TaskQueueConfig::default();
        let queue = Arc::new(TaskQueue::new(config, 100));
        let runloop = Arc::new(RunLoopState::new(tx, queue));
        let hybrid = Arc::new(HybridAppState::new(base, runloop));
        create_router_with_hybrid_state(hybrid)
    }

    #[tokio::test]
    async fn test_task_submit_endpoint() {
        let app = create_test_router();
        let body = serde_json::json!({
            "task": "Hello, what can you do?"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // 200 OK or other valid response
        assert!(response.status().is_success() || response.status().is_client_error());
    }

    #[tokio::test]
    async fn test_task_status_endpoint() {
        let app = create_test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/tasks/test-session-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // May be 200 or 404
        assert!(response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_task_abort_endpoint() {
        let app = create_test_router();
        let body = serde_json::json!({
            "session_id": "test-session-123"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/tasks/test-session-123/abort")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // May be 200 or 404
        assert!(response.status().is_success() || response.status().is_client_error());
    }

    #[tokio::test]
    async fn test_webhook_list_endpoint() {
        let app = create_test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/webhook/list")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_webhook_register_endpoint() {
        let app = create_test_router();
        let body = serde_json::json!({
            "id": "new-webhook",
            "description": "Test webhook",
            "enabled": true
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/register")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_webhook_post_endpoint() {
        let app = create_test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/test-hook")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"test": "data"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn test_webhook_github_endpoint() {
        let app = create_test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/github")
                    .header("content-type", "application/json")
                    .header("x-github-event", "push")
                    .body(Body::from(r#"{"ref": "refs/heads/main"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn test_webhook_delete_endpoint() {
        let app = create_test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/webhook/test-hook")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}
