//! Admin management endpoints.
#![allow(dead_code)]

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

/// Extension info response.
#[derive(Debug, Serialize)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Session info response.
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: i64,
    pub last_active: i64,
    pub message_count: usize,
}

/// System stats response.
#[derive(Debug, Serialize)]
pub struct SystemStats {
    pub uptime_seconds: u64,
    pub active_sessions: usize,
    pub total_requests: u64,
    pub loaded_extensions: usize,
    pub memory_usage_bytes: Option<u64>,
}

/// Error response.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
        }
    }
}

/// List all loaded extensions.
pub async fn list_extensions(State(state): State<Arc<AppState>>) -> Json<Vec<ExtensionInfo>> {
    let extensions = state.kernel.list_extensions();
    let info: Vec<ExtensionInfo> = extensions
        .iter()
        .map(|m| ExtensionInfo {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version.to_string(),
            description: m.description.clone(),
        })
        .collect();

    Json(info)
}

/// Get extension by ID.
pub async fn get_extension(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ExtensionInfo>, (StatusCode, Json<ErrorResponse>)> {
    let extensions = state.kernel.list_extensions();
    let ext = extensions.iter().find(|e| e.id == id);

    match ext {
        Some(m) => Ok(Json(ExtensionInfo {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version.to_string(),
            description: m.description.clone(),
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                format!("Extension not found: {}", id),
                "extension_not_found",
            )),
        )),
    }
}

/// List all active sessions.
pub async fn list_sessions(State(state): State<Arc<AppState>>) -> Json<Vec<SessionInfo>> {
    let sessions = state.list_sessions();
    let info: Vec<SessionInfo> = sessions
        .iter()
        .map(|s| SessionInfo {
            id: s.id.clone(),
            created_at: s.created_at.timestamp(),
            last_active: s.last_active.timestamp(),
            message_count: s.data.len(),
        })
        .collect();

    Json(info)
}

/// Get session by ID.
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionInfo>, (StatusCode, Json<ErrorResponse>)> {
    let session = state.session_manager.get(&id);

    match session {
        Some(s) => Ok(Json(SessionInfo {
            id: s.id.clone(),
            created_at: s.created_at.timestamp(),
            last_active: s.last_active.timestamp(),
            message_count: s.data.len(),
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                format!("Session not found: {}", id),
                "session_not_found",
            )),
        )),
    }
}

/// Delete a session.
pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.session_manager.remove(&id) {
        Some(_) => StatusCode::NO_CONTENT,
        None => StatusCode::NOT_FOUND,
    }
}

/// Get system statistics.
pub async fn system_stats(State(state): State<Arc<AppState>>) -> Json<SystemStats> {
    let uptime = state.uptime().as_secs();
    let sessions = state.list_sessions();
    let extensions = state.kernel.list_extensions();

    Json(SystemStats {
        uptime_seconds: uptime,
        active_sessions: sessions.len(),
        total_requests: state.request_count(),
        loaded_extensions: extensions.len(),
        memory_usage_bytes: get_memory_usage(),
    })
}

/// Get memory usage (platform-specific).
fn get_memory_usage() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            let parts: Vec<&str> = statm.split_whitespace().collect();
            if let Some(rss) = parts.get(1) {
                if let Ok(pages) = rss.parse::<u64>() {
                    return Some(pages * 4096);
                }
            }
        }
    }
    None
}

/// Reload configuration request.
#[derive(Debug, Deserialize)]
pub struct ReloadConfigRequest {
    #[serde(default)]
    pub config_path: Option<String>,
}

/// Reload configuration.
pub async fn reload_config(
    State(_state): State<Arc<AppState>>,
    Json(_request): Json<ReloadConfigRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement actual config reload
    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Configuration reload requested"
    })))
}

/// Shutdown request.
#[derive(Debug, Deserialize)]
pub struct ShutdownRequest {
    #[serde(default)]
    pub graceful: bool,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

/// Request graceful shutdown.
pub async fn shutdown(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ShutdownRequest>,
) -> Json<serde_json::Value> {
    let timeout = request.timeout_seconds.unwrap_or(30);

    // Signal shutdown
    state.request_shutdown();

    Json(serde_json::json!({
        "status": "shutdown_initiated",
        "graceful": request.graceful,
        "timeout_seconds": timeout
    }))
}

#[cfg(test)]
#[path = "admin_tests.rs"]
mod tests;
