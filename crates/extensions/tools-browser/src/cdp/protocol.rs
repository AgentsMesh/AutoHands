//! CDP protocol types and message definitions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// CDP request message.
#[derive(Debug, Serialize)]
pub struct CdpRequest {
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

/// CDP response message.
#[derive(Debug, Deserialize)]
pub struct CdpResponse {
    pub id: Option<u64>,
    pub result: Option<Value>,
    pub error: Option<CdpErrorResponse>,
    pub method: Option<String>,
    pub params: Option<Value>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

/// CDP error in response.
#[derive(Debug, Deserialize)]
pub struct CdpErrorResponse {
    pub code: i64,
    pub message: String,
    pub data: Option<String>,
}

/// Target info from CDP.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    pub target_id: String,
    #[serde(rename = "type")]
    pub target_type: String,
    pub title: String,
    pub url: String,
    pub attached: Option<bool>,
    pub browser_context_id: Option<String>,
}

/// Page info from /json endpoint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub page_type: String,
    pub title: String,
    pub url: String,
    pub web_socket_debugger_url: Option<String>,
    pub dev_tools_frontend_url: Option<String>,
}

/// Browser version info.
///
/// Note: Chrome returns PascalCase field names for this endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct BrowserVersion {
    #[serde(rename = "Browser")]
    pub browser: String,
    #[serde(rename = "Protocol-Version")]
    pub protocol_version: String,
    #[serde(rename = "User-Agent")]
    pub user_agent: String,
    #[serde(rename = "V8-Version")]
    pub v8_version: Option<String>,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub web_socket_debugger_url: String,
}

// ============================================================================
// DOM Types
// ============================================================================

/// DOM node from CDP.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomNode {
    pub node_id: i64,
    pub backend_node_id: i64,
    pub node_type: i64,
    pub node_name: String,
    pub local_name: Option<String>,
    pub node_value: Option<String>,
    pub child_node_count: Option<i64>,
    pub children: Option<Vec<DomNode>>,
    pub attributes: Option<Vec<String>>,
    pub frame_id: Option<String>,
    pub content_document: Option<Box<DomNode>>,
    pub shadow_roots: Option<Vec<DomNode>>,
    pub pseudo_elements: Option<Vec<DomNode>>,
}

/// Box model from CDP.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxModel {
    pub content: Vec<f64>,
    pub padding: Vec<f64>,
    pub border: Vec<f64>,
    pub margin: Vec<f64>,
    pub width: i64,
    pub height: i64,
}

/// Computed style from CDP.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputedStyle {
    pub name: String,
    pub value: String,
}

// ============================================================================
// Runtime Types
// ============================================================================

/// Remote object from Runtime domain.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObject {
    #[serde(rename = "type")]
    pub object_type: String,
    pub subtype: Option<String>,
    pub class_name: Option<String>,
    pub value: Option<Value>,
    pub unserializable_value: Option<String>,
    pub description: Option<String>,
    pub object_id: Option<String>,
}

/// Exception details from Runtime.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionDetails {
    pub exception_id: i64,
    pub text: String,
    pub line_number: i64,
    pub column_number: i64,
    pub script_id: Option<String>,
    pub url: Option<String>,
    pub exception: Option<RemoteObject>,
}

// ============================================================================
// Input Types
// ============================================================================

/// Mouse button.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    None,
    Left,
    Middle,
    Right,
    Back,
    Forward,
}

/// Mouse event type.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MouseEventType {
    MousePressed,
    MouseReleased,
    MouseMoved,
    MouseWheel,
}

/// Key event type.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyEventType {
    KeyDown,
    KeyUp,
    RawKeyDown,
    Char,
}

// ============================================================================
// Accessibility Types
// ============================================================================

/// AX node from Accessibility domain.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXNode {
    pub node_id: String,
    pub ignored: bool,
    pub role: Option<AXValue>,
    pub name: Option<AXValue>,
    pub description: Option<AXValue>,
    pub value: Option<AXValue>,
    pub properties: Option<Vec<AXProperty>>,
    pub child_ids: Option<Vec<String>>,
    pub backend_dom_node_id: Option<i64>,
}

/// AX value.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXValue {
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: Option<Value>,
}

/// AX property.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXProperty {
    pub name: String,
    pub value: AXValue,
}

// ============================================================================
// Event Listener Types
// ============================================================================

/// Event listener info from DOMDebugger.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventListener {
    #[serde(rename = "type")]
    pub event_type: String,
    pub use_capture: bool,
    pub passive: bool,
    pub once: bool,
    pub script_id: String,
    pub line_number: i64,
    pub column_number: i64,
    pub handler: Option<RemoteObject>,
    pub original_handler: Option<RemoteObject>,
}

// ============================================================================
// Screenshot Types
// ============================================================================

/// Screenshot format.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenshotFormat {
    Jpeg,
    Png,
    Webp,
}

/// Viewport for screenshot clip.
#[derive(Debug, Clone, Serialize)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
