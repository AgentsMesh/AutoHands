//! CDP page session for interacting with a single page.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::SinkExt;
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, trace};

use super::client::{PendingRequest, WsSink};
use super::error::CdpError;
use super::protocol::{
    AXNode, BoxModel, CdpRequest, CdpResponse, ComputedStyle, DomNode, EventListener,
    KeyEventType, MouseButton, MouseEventType, RemoteObject, ScreenshotFormat, Viewport,
};

/// A session attached to a single page/target.
pub struct PageSession {
    /// Target ID.
    target_id: String,
    /// Session ID for this target.
    session_id: String,
    /// WebSocket sender (shared with client).
    ws_tx: Arc<tokio::sync::Mutex<WsSink>>,
    /// Pending requests (shared with client).
    pending: Arc<Mutex<HashMap<u64, PendingRequest>>>,
    /// Request ID counter (shared with client).
    request_id: Arc<AtomicU64>,
    /// Event receiver.
    #[allow(dead_code)]
    event_rx: mpsc::UnboundedReceiver<CdpResponse>,
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
            event_rx,
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

        // Create response channel
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.lock().insert(id, PendingRequest { tx });

        // Send request
        {
            let mut ws = self.ws_tx.lock().await;
            ws.send(Message::Text(json.into())).await?;
        }

        // Wait for response
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
        // Enable Page domain
        self.call("Page.enable", None).await?;
        // Enable DOM domain
        self.call("DOM.enable", None).await?;
        // Enable Runtime domain
        self.call("Runtime.enable", None).await?;
        // Enable Network domain (for navigation events)
        self.call("Network.enable", None).await?;
        // Enable CSS domain (for computed styles)
        self.call("CSS.enable", None).await?;

        debug!("Enabled CDP domains for session {}", self.session_id);
        Ok(())
    }

    // ========================================================================
    // Navigation
    // ========================================================================

    /// Navigate to URL.
    pub async fn navigate(&self, url: &str) -> Result<String, CdpError> {
        let result = self
            .call("Page.navigate", Some(json!({"url": url})))
            .await?;

        if let Some(error) = result.get("errorText") {
            return Err(CdpError::NavigationFailed(
                error.as_str().unwrap_or("Unknown error").to_string(),
            ));
        }

        let frame_id = result["frameId"]
            .as_str()
            .unwrap_or("main")
            .to_string();

        // Wait for load event
        self.wait_for_load().await?;

        debug!("Navigated to {}", url);
        Ok(frame_id)
    }

    /// Wait for page load.
    pub async fn wait_for_load(&self) -> Result<(), CdpError> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(30);

        loop {
            let result = self.evaluate("document.readyState").await?;

            if let Some(state) = result.as_str() {
                if state == "complete" || state == "interactive" {
                    return Ok(());
                }
            }

            if start.elapsed() > timeout {
                return Err(CdpError::Timeout("Page load timeout".to_string()));
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// Reload page.
    pub async fn reload(&self) -> Result<(), CdpError> {
        self.call("Page.reload", None).await?;
        self.wait_for_load().await?;
        Ok(())
    }

    /// Go back.
    pub async fn go_back(&self) -> Result<(), CdpError> {
        let history = self.call("Page.getNavigationHistory", None).await?;
        let current_index = history["currentIndex"].as_i64().unwrap_or(0);

        if current_index > 0 {
            let entries = history["entries"].as_array();
            if let Some(entries) = entries {
                if let Some(entry) = entries.get((current_index - 1) as usize) {
                    let entry_id = entry["id"].as_i64().unwrap_or(0);
                    self.call(
                        "Page.navigateToHistoryEntry",
                        Some(json!({"entryId": entry_id})),
                    )
                    .await?;
                    self.wait_for_load().await?;
                }
            }
        }
        Ok(())
    }

    /// Go forward.
    pub async fn go_forward(&self) -> Result<(), CdpError> {
        let history = self.call("Page.getNavigationHistory", None).await?;
        let current_index = history["currentIndex"].as_i64().unwrap_or(0);
        let entries = history["entries"].as_array();

        if let Some(entries) = entries {
            if (current_index as usize) < entries.len() - 1 {
                if let Some(entry) = entries.get((current_index + 1) as usize) {
                    let entry_id = entry["id"].as_i64().unwrap_or(0);
                    self.call(
                        "Page.navigateToHistoryEntry",
                        Some(json!({"entryId": entry_id})),
                    )
                    .await?;
                    self.wait_for_load().await?;
                }
            }
        }
        Ok(())
    }

    /// Get current URL.
    pub async fn get_url(&self) -> Result<String, CdpError> {
        let result = self.evaluate("window.location.href").await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// Get page title.
    pub async fn get_title(&self) -> Result<String, CdpError> {
        let result = self.evaluate("document.title").await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    // ========================================================================
    // Content
    // ========================================================================

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

    // ========================================================================
    // JavaScript Execution
    // ========================================================================

    /// Evaluate JavaScript expression.
    pub async fn evaluate(&self, expression: &str) -> Result<Value, CdpError> {
        let result = self
            .call(
                "Runtime.evaluate",
                Some(json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true,
                })),
            )
            .await?;

        if let Some(exception) = result.get("exceptionDetails") {
            let text = exception["text"].as_str().unwrap_or("Unknown error");
            return Err(CdpError::JavaScript(text.to_string()));
        }

        Ok(result["result"]["value"].clone())
    }

    /// Evaluate JavaScript and return remote object.
    pub async fn evaluate_handle(&self, expression: &str) -> Result<RemoteObject, CdpError> {
        let result = self
            .call(
                "Runtime.evaluate",
                Some(json!({
                    "expression": expression,
                    "returnByValue": false,
                })),
            )
            .await?;

        if let Some(exception) = result.get("exceptionDetails") {
            let text = exception["text"].as_str().unwrap_or("Unknown error");
            return Err(CdpError::JavaScript(text.to_string()));
        }

        let remote_obj: RemoteObject = serde_json::from_value(result["result"].clone())?;
        Ok(remote_obj)
    }

    /// Call function on remote object.
    pub async fn call_function_on(
        &self,
        object_id: &str,
        function: &str,
        args: Option<Vec<Value>>,
    ) -> Result<Value, CdpError> {
        let mut params = json!({
            "objectId": object_id,
            "functionDeclaration": function,
            "returnByValue": true,
            "awaitPromise": true,
        });

        if let Some(a) = args {
            params["arguments"] = json!(a.into_iter().map(|v| json!({"value": v})).collect::<Vec<_>>());
        }

        let result = self.call("Runtime.callFunctionOn", Some(params)).await?;

        if let Some(exception) = result.get("exceptionDetails") {
            let text = exception["text"].as_str().unwrap_or("Unknown error");
            return Err(CdpError::JavaScript(text.to_string()));
        }

        Ok(result["result"]["value"].clone())
    }

    // ========================================================================
    // Input - Mouse
    // ========================================================================

    /// Click at coordinates.
    pub async fn click(&self, x: f64, y: f64) -> Result<(), CdpError> {
        // Mouse down
        self.call(
            "Input.dispatchMouseEvent",
            Some(json!({
                "type": MouseEventType::MousePressed,
                "x": x,
                "y": y,
                "button": MouseButton::Left,
                "clickCount": 1,
            })),
        )
        .await?;

        // Mouse up
        self.call(
            "Input.dispatchMouseEvent",
            Some(json!({
                "type": MouseEventType::MouseReleased,
                "x": x,
                "y": y,
                "button": MouseButton::Left,
                "clickCount": 1,
            })),
        )
        .await?;

        debug!("Clicked at ({}, {})", x, y);
        Ok(())
    }

    /// Double click at coordinates.
    pub async fn double_click(&self, x: f64, y: f64) -> Result<(), CdpError> {
        for click_count in [1, 2] {
            self.call(
                "Input.dispatchMouseEvent",
                Some(json!({
                    "type": MouseEventType::MousePressed,
                    "x": x,
                    "y": y,
                    "button": MouseButton::Left,
                    "clickCount": click_count,
                })),
            )
            .await?;

            self.call(
                "Input.dispatchMouseEvent",
                Some(json!({
                    "type": MouseEventType::MouseReleased,
                    "x": x,
                    "y": y,
                    "button": MouseButton::Left,
                    "clickCount": click_count,
                })),
            )
            .await?;
        }
        Ok(())
    }

    /// Move mouse to coordinates.
    pub async fn mouse_move(&self, x: f64, y: f64) -> Result<(), CdpError> {
        self.call(
            "Input.dispatchMouseEvent",
            Some(json!({
                "type": MouseEventType::MouseMoved,
                "x": x,
                "y": y,
            })),
        )
        .await?;
        Ok(())
    }

    /// Scroll by delta.
    pub async fn scroll(&self, x: f64, y: f64, delta_x: f64, delta_y: f64) -> Result<(), CdpError> {
        self.call(
            "Input.dispatchMouseEvent",
            Some(json!({
                "type": MouseEventType::MouseWheel,
                "x": x,
                "y": y,
                "deltaX": delta_x,
                "deltaY": delta_y,
            })),
        )
        .await?;
        Ok(())
    }

    // ========================================================================
    // Input - Keyboard
    // ========================================================================

    /// Type text.
    pub async fn type_text(&self, text: &str) -> Result<(), CdpError> {
        self.call("Input.insertText", Some(json!({"text": text})))
            .await?;
        debug!("Typed {} characters", text.len());
        Ok(())
    }

    /// Press a key.
    pub async fn press_key(&self, key: &str) -> Result<(), CdpError> {
        // Key down
        self.call(
            "Input.dispatchKeyEvent",
            Some(json!({
                "type": KeyEventType::KeyDown,
                "key": key,
            })),
        )
        .await?;

        // Key up
        self.call(
            "Input.dispatchKeyEvent",
            Some(json!({
                "type": KeyEventType::KeyUp,
                "key": key,
            })),
        )
        .await?;

        Ok(())
    }

    /// Press key combination (e.g., "Control+a").
    pub async fn press_key_combo(&self, combo: &str) -> Result<(), CdpError> {
        let parts: Vec<&str> = combo.split('+').collect();
        let modifiers = Self::get_modifiers(&parts[..parts.len() - 1]);
        let key = parts.last().unwrap_or(&"");

        // Key down with modifiers
        self.call(
            "Input.dispatchKeyEvent",
            Some(json!({
                "type": KeyEventType::KeyDown,
                "key": key,
                "modifiers": modifiers,
            })),
        )
        .await?;

        // Key up
        self.call(
            "Input.dispatchKeyEvent",
            Some(json!({
                "type": KeyEventType::KeyUp,
                "key": key,
                "modifiers": modifiers,
            })),
        )
        .await?;

        Ok(())
    }

    /// Get modifier flags from modifier names.
    fn get_modifiers(modifiers: &[&str]) -> i32 {
        let mut flags = 0;
        for m in modifiers {
            match m.to_lowercase().as_str() {
                "alt" => flags |= 1,
                "control" | "ctrl" => flags |= 2,
                "meta" | "command" | "cmd" => flags |= 4,
                "shift" => flags |= 8,
                _ => {}
            }
        }
        flags
    }

    // ========================================================================
    // DOM Operations
    // ========================================================================

    /// Get document root node.
    pub async fn get_document(&self) -> Result<DomNode, CdpError> {
        let result = self
            .call(
                "DOM.getDocument",
                Some(json!({"depth": -1, "pierce": true})),
            )
            .await?;

        let root: DomNode = serde_json::from_value(result["root"].clone())?;
        Ok(root)
    }

    /// Query selector.
    pub async fn query_selector(&self, selector: &str) -> Result<Option<i64>, CdpError> {
        let doc = self.get_document().await?;

        let result = self
            .call(
                "DOM.querySelector",
                Some(json!({
                    "nodeId": doc.node_id,
                    "selector": selector,
                })),
            )
            .await?;

        let node_id = result["nodeId"].as_i64().unwrap_or(0);
        if node_id == 0 {
            Ok(None)
        } else {
            Ok(Some(node_id))
        }
    }

    /// Query selector all.
    pub async fn query_selector_all(&self, selector: &str) -> Result<Vec<i64>, CdpError> {
        let doc = self.get_document().await?;

        let result = self
            .call(
                "DOM.querySelectorAll",
                Some(json!({
                    "nodeId": doc.node_id,
                    "selector": selector,
                })),
            )
            .await?;

        let node_ids: Vec<i64> = result["nodeIds"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();

        Ok(node_ids)
    }

    /// Get box model for node.
    pub async fn get_box_model(&self, node_id: i64) -> Result<Option<BoxModel>, CdpError> {
        let result = self
            .call("DOM.getBoxModel", Some(json!({"nodeId": node_id})))
            .await;

        match result {
            Ok(r) => {
                let model: BoxModel = serde_json::from_value(r["model"].clone())?;
                Ok(Some(model))
            }
            Err(CdpError::Protocol { code: -32000, .. }) => {
                // Node not visible or doesn't have layout
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Get computed style for node.
    pub async fn get_computed_style(&self, node_id: i64) -> Result<Vec<ComputedStyle>, CdpError> {
        let result = self
            .call(
                "CSS.getComputedStyleForNode",
                Some(json!({"nodeId": node_id})),
            )
            .await?;

        let styles: Vec<ComputedStyle> =
            serde_json::from_value(result["computedStyle"].clone()).unwrap_or_default();

        Ok(styles)
    }

    /// Get event listeners for node.
    pub async fn get_event_listeners(
        &self,
        object_id: &str,
    ) -> Result<Vec<EventListener>, CdpError> {
        let result = self
            .call(
                "DOMDebugger.getEventListeners",
                Some(json!({"objectId": object_id})),
            )
            .await?;

        let listeners: Vec<EventListener> =
            serde_json::from_value(result["listeners"].clone()).unwrap_or_default();

        Ok(listeners)
    }

    /// Resolve node to runtime object.
    pub async fn resolve_node(&self, node_id: i64) -> Result<RemoteObject, CdpError> {
        let result = self
            .call("DOM.resolveNode", Some(json!({"nodeId": node_id})))
            .await?;

        let obj: RemoteObject = serde_json::from_value(result["object"].clone())?;
        Ok(obj)
    }

    /// Focus element.
    pub async fn focus(&self, node_id: i64) -> Result<(), CdpError> {
        self.call("DOM.focus", Some(json!({"nodeId": node_id})))
            .await?;
        Ok(())
    }

    /// Set node value (for input elements).
    pub async fn set_node_value(&self, node_id: i64, value: &str) -> Result<(), CdpError> {
        // Focus the element first
        self.focus(node_id).await?;

        // Clear existing value and type new value
        self.press_key_combo("Control+a").await?;
        self.type_text(value).await?;

        Ok(())
    }

    /// Click on element by selector.
    pub async fn click_selector(&self, selector: &str) -> Result<(), CdpError> {
        let node_id = self
            .query_selector(selector)
            .await?
            .ok_or_else(|| CdpError::ElementNotFound(selector.to_string()))?;

        let box_model = self
            .get_box_model(node_id)
            .await?
            .ok_or_else(|| CdpError::ElementNotFound(format!("{} (not visible)", selector)))?;

        // Calculate center point from content quad
        let (x, y) = Self::quad_center(&box_model.content);
        self.click(x, y).await
    }

    /// Fill input by selector.
    pub async fn fill(&self, selector: &str, value: &str) -> Result<(), CdpError> {
        let node_id = self
            .query_selector(selector)
            .await?
            .ok_or_else(|| CdpError::ElementNotFound(selector.to_string()))?;

        self.set_node_value(node_id, value).await
    }

    /// Calculate center point of a quad.
    fn quad_center(quad: &[f64]) -> (f64, f64) {
        if quad.len() >= 8 {
            let x = (quad[0] + quad[2] + quad[4] + quad[6]) / 4.0;
            let y = (quad[1] + quad[3] + quad[5] + quad[7]) / 4.0;
            (x, y)
        } else {
            (0.0, 0.0)
        }
    }

    // ========================================================================
    // Accessibility
    // ========================================================================

    /// Get accessibility tree.
    pub async fn get_accessibility_tree(&self) -> Result<Vec<AXNode>, CdpError> {
        // Enable accessibility domain
        self.call("Accessibility.enable", None).await?;

        let result = self.call("Accessibility.getFullAXTree", None).await?;

        let nodes: Vec<AXNode> =
            serde_json::from_value(result["nodes"].clone()).unwrap_or_default();

        Ok(nodes)
    }

    // ========================================================================
    // Wait Operations
    // ========================================================================

    /// Wait for selector to appear.
    pub async fn wait_for_selector(
        &self,
        selector: &str,
        timeout_ms: Option<u32>,
    ) -> Result<i64, CdpError> {
        let timeout = std::time::Duration::from_millis(timeout_ms.unwrap_or(30000) as u64);
        let start = std::time::Instant::now();

        loop {
            if let Some(node_id) = self.query_selector(selector).await? {
                return Ok(node_id);
            }

            if start.elapsed() > timeout {
                return Err(CdpError::Timeout(format!(
                    "Waiting for selector '{}' timed out",
                    selector
                )));
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// Wait for navigation.
    pub async fn wait_for_navigation(&self, timeout_ms: Option<u32>) -> Result<(), CdpError> {
        let timeout = std::time::Duration::from_millis(timeout_ms.unwrap_or(30000) as u64);
        tokio::time::timeout(timeout, self.wait_for_load())
            .await
            .map_err(|_| CdpError::Timeout("Navigation timeout".to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quad_center() {
        let quad = vec![0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
        let (x, y) = PageSession::quad_center(&quad);
        assert_eq!(x, 50.0);
        assert_eq!(y, 50.0);
    }

    #[test]
    fn test_get_modifiers() {
        let modifiers = ["Control", "Shift"];
        let flags = PageSession::get_modifiers(&modifiers);
        assert_eq!(flags, 10); // 2 + 8
    }

    #[test]
    fn test_get_modifiers_mac() {
        let modifiers = ["Meta", "a"];
        // Only Meta should be counted, 'a' is not a modifier
        let flags = PageSession::get_modifiers(&modifiers[..1]);
        assert_eq!(flags, 4);
    }
}
