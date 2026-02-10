//! Core session struct and CDP command dispatch.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::SinkExt;
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, trace};

use crate::cdp::client::{PendingRequest, WsSink};
use crate::cdp::error::CdpError;
use crate::cdp::protocol::{CdpRequest, CdpResponse, ScreenshotFormat, Viewport};

/// A session attached to a single page/target.
pub struct PageSession {
    /// Target ID.
    pub(super) target_id: String,
    /// Session ID for this target.
    pub(super) session_id: String,
    /// WebSocket sender (shared with client).
    pub(super) ws_tx: Arc<tokio::sync::Mutex<WsSink>>,
    /// Pending requests (shared with client).
    pub(super) pending: Arc<Mutex<HashMap<u64, PendingRequest>>>,
    /// Request ID counter (shared with client).
    pub(super) request_id: Arc<AtomicU64>,
    /// Event receiver (kept alive to prevent sender errors).
    pub(super) _event_rx: mpsc::UnboundedReceiver<CdpResponse>,
}

impl PageSession {
    /// Create a new page session.
    pub(crate) fn new(
        target_id: String,
        session_id: String,
        ws_tx: Arc<tokio::sync::Mutex<WsSink>>,
        pending: Arc<Mutex<HashMap<u64, PendingRequest>>>,
        request_id: Arc<AtomicU64>,
        event_rx: mpsc::UnboundedReceiver<CdpResponse>,
    ) -> Self {
        Self {
            target_id,
            session_id,
            ws_tx,
            pending,
            request_id,
            _event_rx: event_rx,
        }
    }

    /// Get target ID.
    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    /// Get session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Send a CDP command to this page session.
    pub async fn call(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);

        let request = CdpRequest {
            id,
            method: method.to_string(),
            params,
            session_id: Some(self.session_id.clone()),
        };

        let json = serde_json::to_string(&request)?;
        trace!("CDP session send: {}", json);

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.lock().insert(id, PendingRequest { tx });

        {
            let mut ws = self.ws_tx.lock().await;
            ws.send(Message::Text(json.into())).await?;
        }

        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(CdpError::SessionClosed),
            Err(_) => {
                self.pending.lock().remove(&id);
                Err(CdpError::Timeout(format!("Request {} timed out", method)))
            }
        }
    }

    /// Enable required CDP domains.
    pub(crate) async fn enable_domains(&self) -> Result<(), CdpError> {
        self.call("Page.enable", None).await?;
        self.call("DOM.enable", None).await?;
        self.call("Runtime.enable", None).await?;
        self.call("Network.enable", None).await?;
        self.call("CSS.enable", None).await?;

        debug!("Enabled CDP domains for session {}", self.session_id);
        Ok(())
    }

    /// Get page HTML content.
    pub async fn get_content(&self) -> Result<String, CdpError> {
        let result = self.evaluate("document.documentElement.outerHTML").await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// Take screenshot.
    pub async fn screenshot(
        &self,
        format: ScreenshotFormat,
        quality: Option<u8>,
        full_page: bool,
        clip: Option<Viewport>,
    ) -> Result<String, CdpError> {
        let mut params = json!({
            "format": format,
            "captureBeyondViewport": full_page,
        });

        if let Some(q) = quality {
            params["quality"] = json!(q);
        }

        if let Some(c) = clip {
            params["clip"] = serde_json::to_value(c)?;
        }

        let result = self.call("Page.captureScreenshot", Some(params)).await?;

        result["data"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| CdpError::InvalidResponse("Missing screenshot data".to_string()))
    }
}
