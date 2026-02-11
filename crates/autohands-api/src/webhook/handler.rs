//! Webhook handlers implementation.
//!
//! **P0 FIX**: Webhook events are now properly converted to RunLoop events.

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde_json::Value;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::types::{WebhookEvent, WebhookRegistration, WebhookResponse};
use crate::runloop_bridge::HybridAppState;

type HmacSha256 = Hmac<Sha256>;

/// Verify a GitHub webhook signature using HMAC-SHA256.
///
/// The `signature_header` is expected in the format `sha256=<hex-digest>`.
fn verify_github_signature(secret: &str, signature_header: &str, body_bytes: &[u8]) -> bool {
    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        warn!("GitHub signature header missing 'sha256=' prefix");
        return false;
    };

    let Ok(expected_bytes) = hex::decode(hex_sig) else {
        warn!("GitHub signature header contains invalid hex");
        return false;
    };

    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        warn!("Failed to create HMAC from secret");
        return false;
    };

    mac.update(body_bytes);
    mac.verify_slice(&expected_bytes).is_ok()
}

/// List registered webhooks.
///
/// GET /webhook/list
pub async fn list_webhooks(State(state): State<Arc<HybridAppState>>) -> impl IntoResponse {
    let webhooks = state.webhook_registry().list();

    Json(serde_json::json!({
        "count": webhooks.len(),
        "webhooks": webhooks,
    }))
}

/// Register a new webhook.
///
/// POST /webhook/register
pub async fn register_webhook(
    State(state): State<Arc<HybridAppState>>,
    Json(registration): Json<WebhookRegistration>,
) -> impl IntoResponse {
    info!("Registering webhook: {}", registration.id);

    let webhook_id = registration.id.clone();
    state.webhook_registry().register(registration);

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "registered",
            "webhook_id": webhook_id,
            "message": "Webhook registered successfully",
        })),
    )
}

/// Get webhook details.
///
/// GET /webhook/{id}
pub async fn get_webhook(
    State(state): State<Arc<HybridAppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.webhook_registry().get(&id) {
        Some(webhook) => (StatusCode::OK, Json(serde_json::to_value(webhook).unwrap())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "not_found",
                "message": format!("Webhook '{}' not found", id),
            })),
        ),
    }
}

/// Delete a webhook.
///
/// DELETE /webhook/{id}
pub async fn delete_webhook(
    State(state): State<Arc<HybridAppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!("Deleting webhook: {}", id);

    match state.webhook_registry().remove(&id) {
        Some(_) => StatusCode::NO_CONTENT,
        None => StatusCode::NOT_FOUND,
    }
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

    // Verify webhook is registered and enabled
    let registration = state.webhook_registry().get(&id);
    if let Some(ref reg) = registration {
        if !reg.enabled {
            warn!("Webhook '{}' is disabled, rejecting event", id);
            return (
                StatusCode::FORBIDDEN,
                Json(WebhookResponse::rejected(
                    event_id,
                    format!("Webhook '{}' is disabled", id),
                )),
            );
        }
    }

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
    match runloop_state.submit_task("trigger:webhook", payload, None).await {
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
/// It extracts the raw body bytes for HMAC-SHA256 signature verification
/// before parsing as JSON.
pub async fn handle_github_webhook(
    State(state): State<Arc<HybridAppState>>,
    headers: HeaderMap,
    body_bytes: Bytes,
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

    // Verify signature if the "github" webhook has a secret configured
    if let Some(sig_header) = headers.get("x-hub-signature-256") {
        if let Ok(sig_str) = sig_header.to_str() {
            // Look up the GitHub webhook registration for its secret
            let secret = state
                .webhook_registry()
                .get("github")
                .and_then(|reg| reg.secret.clone());

            if let Some(secret) = secret {
                if !verify_github_signature(&secret, sig_str, &body_bytes) {
                    warn!("GitHub webhook signature verification failed");
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(WebhookResponse::rejected(
                            event_id,
                            "Signature verification failed".to_string(),
                        )),
                    );
                }
                debug!("GitHub webhook signature verified successfully");
            } else {
                debug!("No secret configured for GitHub webhook, skipping signature verification");
            }
        }
    }

    // Parse body as JSON after signature verification
    let body: Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to parse GitHub webhook body as JSON: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(WebhookResponse::rejected(
                    event_id,
                    format!("Invalid JSON body: {}", e),
                )),
            );
        }
    };

    // Build webhook event with GitHub context
    let event = WebhookEvent::new("github", body)
        .with_header("x-github-event", event_type)
        .with_header("x-github-delivery", delivery_id);

    // Convert to RunLoop event and inject
    let payload = event.to_runloop_payload();
    debug!("GitHub webhook payload: {:?}", payload);

    // Inject event into RunLoop
    let runloop_state = state.runloop_state();
    match runloop_state.submit_task("trigger:github", payload, None).await {
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
#[path = "handler_tests.rs"]
mod tests;
