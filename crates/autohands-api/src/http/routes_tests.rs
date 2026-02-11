
    use super::*;
    use crate::runloop_bridge::RunLoopState;
    use crate::state::AppState;
    use autohands_runloop::{RunLoop, RunLoopConfig};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn create_test_router() -> Router {
        let base = Arc::new(AppState::default());
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
        let runloop = Arc::new(RunLoopState::from_runloop(run_loop));
        let api_ws_channel = Arc::new(crate::websocket::ApiWsChannel::new());
        let hybrid = Arc::new(HybridAppState::new(base, runloop, api_ws_channel));
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
        // Deleting a non-existent webhook returns NOT_FOUND
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

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
