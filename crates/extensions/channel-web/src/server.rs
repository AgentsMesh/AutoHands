//! HTTP server and routing.

use std::sync::Arc;

use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State,
    },
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use rust_embed::RustEmbed;
use tracing::debug;

use crate::{WebChannelState, WebSocketConnection};

/// Embedded static assets.
#[derive(RustEmbed)]
#[folder = "src/static/"]
struct StaticAssets;

/// Create the Axum router for the web channel.
pub fn create_router(state: Arc<WebChannelState>) -> Router {
    Router::new()
        // Static file routes
        .route("/", get(serve_index))
        .route("/style.css", get(serve_css))
        .route("/app.js", get(serve_js))
        // WebSocket endpoint
        .route("/ws", get(ws_handler))
        // Health check
        .route("/health", get(health_check))
        // API info
        .route("/api/info", get(api_info))
        .with_state(state)
}

/// Serve the index HTML page.
async fn serve_index() -> impl IntoResponse {
    match StaticAssets::get("index.html") {
        Some(content) => Html(String::from_utf8_lossy(content.data.as_ref()).to_string()),
        None => Html(default_index_html().to_string()),
    }
}

/// Serve the CSS stylesheet.
async fn serve_css() -> impl IntoResponse {
    match StaticAssets::get("style.css") {
        Some(content) => (
            [(header::CONTENT_TYPE, "text/css")],
            String::from_utf8_lossy(content.data.as_ref()).to_string(),
        ),
        None => (
            [(header::CONTENT_TYPE, "text/css")],
            default_style_css().to_string(),
        ),
    }
}

/// Serve the JavaScript app.
async fn serve_js() -> impl IntoResponse {
    match StaticAssets::get("app.js") {
        Some(content) => (
            [(header::CONTENT_TYPE, "application/javascript")],
            String::from_utf8_lossy(content.data.as_ref()).to_string(),
        ),
        None => (
            [(header::CONTENT_TYPE, "application/javascript")],
            default_app_js().to_string(),
        ),
    }
}

/// WebSocket upgrade handler.
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WebChannelState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle a new WebSocket connection.
async fn handle_socket(socket: WebSocket, state: Arc<WebChannelState>) {
    let conn_id = uuid::Uuid::new_v4().to_string();
    debug!("New WebSocket connection: {}", conn_id);

    // Create and register the connection
    let conn = WebSocketConnection::spawn(conn_id.clone(), socket, state.clone());
    state.connections.insert(conn_id, conn);
}

/// Health check endpoint.
async fn health_check(State(state): State<Arc<WebChannelState>>) -> impl IntoResponse {
    let status = if state.started.load(std::sync::atomic::Ordering::SeqCst) {
        "ok"
    } else {
        "starting"
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::json!({
            "status": status,
            "channel_id": state.id,
            "connections": state.connections.len(),
        })
        .to_string(),
    )
}

/// API info endpoint.
async fn api_info(State(state): State<Arc<WebChannelState>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::json!({
            "name": "AutoHands Web Channel",
            "version": env!("CARGO_PKG_VERSION"),
            "channel_id": state.id,
            "endpoints": {
                "websocket": "/ws",
                "health": "/health",
                "info": "/api/info"
            }
        })
        .to_string(),
    )
}

// === Default embedded content (fallback if static files not found) ===

fn default_index_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AutoHands</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <div id="app">
        <header>
            <h1>AutoHands</h1>
            <span id="status" class="status disconnected">Disconnected</span>
        </header>
        <div id="messages"></div>
        <form id="input-form">
            <input type="text" id="input" placeholder="Type a message..." autocomplete="off">
            <button type="submit">Send</button>
        </form>
    </div>
    <script src="app.js"></script>
</body>
</html>"#
}

fn default_style_css() -> &'static str {
    r#"* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    background: #1a1a2e;
    color: #eee;
    height: 100vh;
    display: flex;
    justify-content: center;
    align-items: center;
}

#app {
    width: 100%;
    max-width: 800px;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: #16213e;
}

header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    background: #0f3460;
    border-bottom: 1px solid #e94560;
}

header h1 {
    font-size: 1.5rem;
    color: #e94560;
}

.status {
    padding: 0.25rem 0.75rem;
    border-radius: 1rem;
    font-size: 0.8rem;
    font-weight: 500;
}

.status.connected {
    background: #10b981;
    color: #fff;
}

.status.disconnected {
    background: #ef4444;
    color: #fff;
}

#messages {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}

.message {
    max-width: 80%;
    padding: 0.75rem 1rem;
    border-radius: 1rem;
    line-height: 1.5;
    word-wrap: break-word;
}

.message.user {
    align-self: flex-end;
    background: #e94560;
    color: #fff;
    border-bottom-right-radius: 0.25rem;
}

.message.assistant {
    align-self: flex-start;
    background: #0f3460;
    color: #eee;
    border-bottom-left-radius: 0.25rem;
}

#input-form {
    display: flex;
    padding: 1rem;
    background: #0f3460;
    border-top: 1px solid #e94560;
    gap: 0.5rem;
}

#input {
    flex: 1;
    padding: 0.75rem 1rem;
    border: none;
    border-radius: 0.5rem;
    background: #16213e;
    color: #eee;
    font-size: 1rem;
    outline: none;
}

#input:focus {
    box-shadow: 0 0 0 2px #e94560;
}

#input::placeholder {
    color: #888;
}

button {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 0.5rem;
    background: #e94560;
    color: #fff;
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
}

button:hover {
    background: #d63050;
}

button:disabled {
    background: #666;
    cursor: not-allowed;
}"#
}

fn default_app_js() -> &'static str {
    r#"// AutoHands Web Channel Client
const messages = document.getElementById('messages');
const form = document.getElementById('input-form');
const input = document.getElementById('input');
const status = document.getElementById('status');

let ws = null;
let reconnectAttempts = 0;
const maxReconnectAttempts = 5;
const reconnectDelay = 2000;

function connect() {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(`${protocol}//${location.host}/ws`);

    ws.onopen = () => {
        console.log('WebSocket connected');
        status.textContent = 'Connected';
        status.className = 'status connected';
        reconnectAttempts = 0;
        input.disabled = false;
    };

    ws.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            if (data.type === 'message' && data.content) {
                addMessage(data.content, 'assistant');
            }
        } catch (e) {
            console.error('Failed to parse message:', e);
        }
    };

    ws.onclose = () => {
        console.log('WebSocket disconnected');
        status.textContent = 'Disconnected';
        status.className = 'status disconnected';
        input.disabled = true;

        if (reconnectAttempts < maxReconnectAttempts) {
            reconnectAttempts++;
            console.log(`Reconnecting in ${reconnectDelay}ms (attempt ${reconnectAttempts})`);
            setTimeout(connect, reconnectDelay);
        }
    };

    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
    };
}

function addMessage(content, role) {
    const div = document.createElement('div');
    div.className = `message ${role}`;
    div.textContent = content;
    messages.appendChild(div);
    messages.scrollTop = messages.scrollHeight;
}

form.onsubmit = (e) => {
    e.preventDefault();
    const text = input.value.trim();
    if (text && ws && ws.readyState === WebSocket.OPEN) {
        addMessage(text, 'user');
        ws.send(JSON.stringify({ content: text }));
        input.value = '';
    }
};

// Start connection
connect();"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_html_content() {
        let html = default_index_html();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("AutoHands"));
        assert!(html.contains("messages"));
    }

    #[test]
    fn test_default_css_content() {
        let css = default_style_css();
        assert!(css.contains("body"));
        assert!(css.contains(".message"));
    }

    #[test]
    fn test_default_js_content() {
        let js = default_app_js();
        assert!(js.contains("WebSocket"));
        assert!(js.contains("connect"));
    }

    #[test]
    fn test_create_router() {
        let state = Arc::new(WebChannelState::new("web"));
        let _router = create_router(state);
        // Router should be created without panicking
    }
}
