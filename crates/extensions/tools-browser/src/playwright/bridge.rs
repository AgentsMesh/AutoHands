//! Node.js Playwright bridge for browser automation.
//!
//! This module manages a Node.js child process that runs Playwright commands.
//! Communication happens via JSON-RPC over stdin/stdout.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex, RwLock};
use tracing::{debug, error, info, warn};

use super::error::PlaywrightError;

/// Bridge configuration.
#[derive(Debug, Clone)]
pub struct PlaywrightBridgeConfig {
    /// Path to Node.js executable.
    pub node_path: Option<PathBuf>,
    /// Path to the bridge script (auto-generated if None).
    pub bridge_script_path: Option<PathBuf>,
    /// Timeout for bridge responses in milliseconds.
    pub response_timeout_ms: u64,
    /// Whether to install browsers automatically.
    pub auto_install_browsers: bool,
}

impl Default for PlaywrightBridgeConfig {
    fn default() -> Self {
        Self {
            node_path: None,
            bridge_script_path: None,
            response_timeout_ms: 30000,
            auto_install_browsers: false,
        }
    }
}

/// Request sent to the bridge.
#[derive(Debug, Serialize)]
struct BridgeRequest {
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// Response from the bridge.
#[derive(Debug, Deserialize)]
struct BridgeResponse {
    id: u64,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<BridgeErrorResponse>,
}

#[derive(Debug, Deserialize)]
struct BridgeErrorResponse {
    message: String,
    #[serde(default, rename = "code")]
    _code: Option<i32>,
}

/// Screenshot options.
#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotOptions {
    #[serde(rename = "fullPage", skip_serializing_if = "Option::is_none")]
    pub full_page: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip: Option<ClipRegion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<u8>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClipRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

type PendingRequests = HashMap<u64, oneshot::Sender<Result<serde_json::Value, PlaywrightError>>>;

/// Node.js Playwright bridge.
pub struct PlaywrightBridge {
    pub(super) config: PlaywrightBridgeConfig,
    process: Mutex<Option<Child>>,
    pub(super) stdin: Mutex<Option<tokio::process::ChildStdin>>,
    pub(super) request_id: AtomicU64,
    pub(super) pending_requests: Arc<RwLock<PendingRequests>>,
    bridge_script: String,
}

impl PlaywrightBridge {
    /// Create a new Playwright bridge.
    pub fn new(config: PlaywrightBridgeConfig) -> Self {
        Self {
            config,
            process: Mutex::new(None),
            stdin: Mutex::new(None),
            request_id: AtomicU64::new(1),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            bridge_script: super::bridge_script::generate_bridge_script(),
        }
    }

    /// Start the bridge process.
    pub async fn start(&self) -> Result<(), PlaywrightError> {
        let node_path = self.find_node()?;

        // Write bridge script to temp file
        let script_path = std::env::temp_dir().join("autohands_playwright_bridge.js");
        tokio::fs::write(&script_path, &self.bridge_script)
            .await
            .map_err(|e| PlaywrightError::BridgeStartFailed(format!("Failed to write bridge script: {}", e)))?;

        info!("Starting Playwright bridge at {:?}", script_path);

        let mut child = Command::new(&node_path)
            .arg(&script_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| PlaywrightError::BridgeStartFailed(e.to_string()))?;

        // Take stdin for writing
        let stdin = child.stdin.take().ok_or_else(|| {
            PlaywrightError::BridgeStartFailed("Failed to get stdin".to_string())
        })?;

        // Set up stderr logging
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    warn!("[Playwright Bridge] {}", line);
                }
            });
        }

        // Set up stdout response handler
        let stdout = child.stdout.take().ok_or_else(|| {
            PlaywrightError::BridgeStartFailed("Failed to get stdout".to_string())
        })?;

        let pending = self.pending_requests.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                debug!("Bridge response: {}", &line[..line.len().min(200)]);

                match serde_json::from_str::<BridgeResponse>(&line) {
                    Ok(response) => {
                        let mut pending = pending.write().await;
                        if let Some(sender) = pending.remove(&response.id) {
                            let result = if let Some(err) = response.error {
                                Err(PlaywrightError::BridgeError(err.message))
                            } else {
                                Ok(response.result.unwrap_or(serde_json::Value::Null))
                            };
                            let _ = sender.send(result);
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse bridge response: {} - {}", e, line);
                    }
                }
            }
        });

        *self.process.lock().await = Some(child);
        *self.stdin.lock().await = Some(stdin);

        // Wait for bridge to be ready
        let ready = self.call("ping", serde_json::json!({})).await?;
        if ready.as_str() != Some("pong") {
            return Err(PlaywrightError::BridgeStartFailed(
                "Bridge did not respond correctly to ping".to_string(),
            ));
        }

        info!("Playwright bridge started successfully");
        Ok(())
    }

    /// Stop the bridge process.
    pub async fn stop(&self) -> Result<(), PlaywrightError> {
        // Send shutdown command
        let _ = self.call("shutdown", serde_json::json!({})).await;

        // Kill process
        if let Some(mut child) = self.process.lock().await.take() {
            let _ = child.kill().await;
        }

        info!("Playwright bridge stopped");
        Ok(())
    }

    /// Call a bridge method.
    pub async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, PlaywrightError> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);

        let request = BridgeRequest {
            id,
            method: method.to_string(),
            params,
        };

        let request_json =
            serde_json::to_string(&request).map_err(|e| PlaywrightError::CommunicationError(e.to_string()))?;

        debug!("Bridge request: {}", &request_json[..request_json.len().min(200)]);

        // Create response channel
        let (tx, rx) = oneshot::channel();
        self.pending_requests.write().await.insert(id, tx);

        // Send request
        {
            let mut stdin_guard = self.stdin.lock().await;
            let stdin = stdin_guard
                .as_mut()
                .ok_or(PlaywrightError::NotInitialized)?;

            stdin.write_all(request_json.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }

        // Wait for response with timeout
        let timeout = tokio::time::Duration::from_millis(self.config.response_timeout_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(PlaywrightError::CommunicationError(
                "Response channel closed".to_string(),
            )),
            Err(_) => {
                self.pending_requests.write().await.remove(&id);
                Err(PlaywrightError::Timeout(format!(
                    "Method {} timed out after {}ms",
                    method, self.config.response_timeout_ms
                )))
            }
        }
    }

    // ============================================================================
    // Internal
    // ============================================================================

    /// Find Node.js executable.
    fn find_node(&self) -> Result<PathBuf, PlaywrightError> {
        if let Some(ref path) = self.config.node_path {
            return Ok(path.clone());
        }

        // Try common locations
        let candidates = [
            "node",
            "/usr/local/bin/node",
            "/usr/bin/node",
            "/opt/homebrew/bin/node",
        ];

        for candidate in candidates {
            if which::which(candidate).is_ok() {
                return Ok(PathBuf::from(candidate));
            }
        }

        Err(PlaywrightError::NodeNotFound)
    }
}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod tests;
