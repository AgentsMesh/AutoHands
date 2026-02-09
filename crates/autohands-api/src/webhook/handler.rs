//! Webhook handlers implementation.
//!
//! **P0 FIX**: Webhook events are now properly converted to RunLoop events.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::Value;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::types::{WebhookEvent, WebhookRegistration, WebhookResponse};
use crate::runloop_bridge::HybridAppState;

/// List registered webhooks.
///
/// GET /webhook/list
pub async fn list_webhooks(State(_state): State<Arc<HybridAppState>>) -> impl IntoResponse {
    // TODO: Implement webhook registry storage
    let webhooks: Vec<WebhookRegistration> = vec![
        WebhookRegistration::new("github")
            .with_description("GitHub push/PR events")
            .with_agent("github-handler"),
        WebhookRegistration::new("generic")
            .with_description("Generic webhook endpoint"),
    ];

    Json(serde_json::json!({
        "count": webhooks.len(),
        "webhooks": webhooks,
    }))
}

/// Register a new webhook.
///
/// POST /webhook/register
pub async fn register_webhook(
    State(_state): State<Arc<HybridAppState>>,
    Json(registration): Json<WebhookRegistration>,
) -> impl IntoResponse {
    info!("Registering webhook: {}", registration.id);

    // TODO: Store in webhook registry
    // For now, just acknowledge

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "registered",
            "webhook_id": registration.id,
            "message": "Webhook registered successfully",
        })),
    )
}

/// Get webhook details.
///
/// GET /webhook/{id}
pub async fn get_webhook(
    State(_state): State<Arc<HybridAppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // TODO: Look up in webhook registry
    let webhook = WebhookRegistration::new(&id)
        .with_description(format!("Webhook: {}", id));

    Json(webhook)
}

/// Delete a webhook.
///
/// DELETE /webhook/{id}
pub async fn delete_webhook(
    State(_state): State<Arc<HybridAppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!("Deleting webhook: {}", id);

    // TODO: Remove from webhook registry
    StatusCode::NO_CONTENT
}

/// Handle incoming webhook event.
///
/// POST /webhook/{id}
///
/// **P0 FIX**: This handler converts webhook payload to RunLoop event.
pub async fn handle_webhook(
    State(state): State<Arc<HybridAppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let event_id = Uuid::new_v4().to_string();
    info!("Webhook received: id={}, event_id={}", id, event_id);

    // Build webhook event
    let mut event = WebhookEvent::new(&id, body);

    // Extract relevant headers
    let header_keys = [
        "content-type",
        "user-agent",
        "x-request-id",
        "x-correlation-id",
    ];
    for key in header_keys {
        if let Some(value) = headers.get(key) {
            if let Ok(v) = value.to_str() {
                event = event.with_header(key, v);
            }
        }
    }

    // Convert to RunLoop event and inject
    let payload = event.to_runloop_payload();
    debug!("Webhook event payload: {:?}", payload);

    // Inject event into RunLoop
    let runloop_state = state.runloop_state();
    match runloop_state.submit_task("trigger:webhook", payload).await {
        Ok(()) => {
            info!("Webhook event injected into RunLoop: event_id={}", event_id);
            (
                StatusCode::ACCEPTED,
                Json(WebhookResponse::accepted_with_message(
                    event_id,
                    format!("Webhook {} event queued for processing", id),
                )),
            )
        }
        Err(e) => {
            warn!("Failed to inject webhook event: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(WebhookResponse::rejected(
                    event_id,
                    format!("Failed to queue event: {}", e),
                )),
            )
        }
    }
}

/// Handle GitHub webhook events.
///
/// POST /webhook/github
///
/// This handler specifically handles GitHub webhook format.
pub async fn handle_github_webhook(
    State(state): State<Arc<HybridAppState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let event_id = Uuid::new_v4().to_string();

    // Extract GitHub-specific headers
    let event_type = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let delivery_id = headers
        .get("x-github-delivery")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(&event_id);

    info!(
        "GitHub webhook received: event={}, delivery={}",
        event_type, delivery_id
    );

    // Build webhook event with GitHub context
    let mut event = WebhookEvent::new("github", body)
        .with_header("x-github-event", event_type)
        .with_header("x-github-delivery", delivery_id);

    // Extract signature for verification (if needed)
    if let Some(sig) = headers.get("x-hub-signature-256") {
        if let Ok(v) = sig.to_str() {
            event = event.with_header("x-hub-signature-256", v);
            // TODO: Implement signature verification
            debug!("GitHub signature present (verification not yet implemented)");
        }
    }

    // Convert to RunLoop event and inject
    let payload = event.to_runloop_payload();
    debug!("GitHub webhook payload: {:?}", payload);

    // Inject event into RunLoop
    let runloop_state = state.runloop_state();
    match runloop_state.submit_task("trigger:github", payload).await {
        Ok(()) => {
            info!(
                "GitHub webhook injected into RunLoop: event={}, event_id={}",
                event_type, event_id
            );
            (
                StatusCode::ACCEPTED,
                Json(WebhookResponse::accepted_with_message(
                    event_id,
                    format!("GitHub {} event queued for processing", event_type),
                )),
            )
        }
        Err(e) => {
            warn!("Failed to inject GitHub webhook event: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(WebhookResponse::rejected(
                    event_id,
                    format!("Failed to queue event: {}", e),
                )),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_response_creation() {
        let resp = WebhookResponse::accepted("test-event");
        assert!(resp.accepted);
        assert_eq!(resp.event_id, "test-event");
    }

    #[test]
    fn test_webhook_event_creation() {
        let event = WebhookEvent::new("test", serde_json::json!({"data": "value"}));
        assert_eq!(event.webhook_id, "test");
        assert_eq!(event.method, "POST");
    }

    #[test]
    fn test_webhook_event_with_headers() {
        let event = WebhookEvent::new("test", serde_json::json!(null))
            .with_header("X-Custom", "value")
            .with_header("Authorization", "Bearer token");

        assert_eq!(event.headers.len(), 2);
        assert_eq!(event.headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_webhook_registration() {
        let reg = WebhookRegistration::new("test-hook")
            .with_description("Test webhook")
            .with_agent("test-agent")
            .with_enabled(true);

        assert_eq!(reg.id, "test-hook");
        assert_eq!(reg.description, Some("Test webhook".to_string()));
        assert_eq!(reg.agent, Some("test-agent".to_string()));
        assert!(reg.enabled);
    }
}
