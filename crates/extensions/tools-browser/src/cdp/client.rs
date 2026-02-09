//! CDP WebSocket client.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, trace, warn};

use super::error::CdpError;
use super::protocol::{BrowserVersion, CdpRequest, CdpResponse, PageInfo, TargetInfo};
use super::session::PageSession;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub(crate) type WsSink = SplitSink<WsStream, Message>;
type WsSource = SplitStream<WsStream>;

/// Pending request waiting for response.
pub(crate) struct PendingRequest {
    pub tx: oneshot::Sender<Result<Value, CdpError>>,
}

/// CDP client for browser automation.
///
/// Connects to Chrome via WebSocket and provides methods for browser control.
pub struct CdpClient {
    /// HTTP endpoint for page discovery.
    http_endpoint: String,
    /// Browser WebSocket URL.
    browser_ws_url: String,
    /// WebSocket sender.
    ws_tx: Arc<tokio::sync::Mutex<WsSink>>,
    /// Request ID counter.
    request_id: Arc<AtomicU64>,
    /// Pending requests waiting for responses.
    pending: Arc<Mutex<HashMap<u64, PendingRequest>>>,
    /// Event handlers by session ID.
    #[allow(clippy::type_complexity)]
    event_handlers: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<CdpResponse>>>>,
    /// Background task handle.
    _recv_task: tokio::task::JoinHandle<()>,
}

impl CdpClient {
    /// Connect to Chrome at the given endpoint.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Chrome debugging endpoint (e.g., "http://localhost:9222")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let client = CdpClient::connect("http://localhost:9222").await?;
    /// ```
    pub async fn connect(endpoint: &str) -> Result<Self, CdpError> {
        let http_endpoint = endpoint.trim_end_matches('/').to_string();

        // Get browser version info to find WebSocket URL
        let version_url = format!("{}/json/version", http_endpoint);
        debug!("Fetching browser version from {}", version_url);

        let version: BrowserVersion = reqwest::get(&version_url)
            .await
            .map_err(|e| CdpError::ChromeNotAvailable(format!("{}: {}", endpoint, e)))?
            .json()
            .await
            .map_err(|e| CdpError::ChromeNotAvailable(format!("{}: {}", endpoint, e)))?;

        debug!("Connected to browser: {}", version.browser);

        let browser_ws_url = version.web_socket_debugger_url;

        // Connect WebSocket
        let (ws_stream, _) = tokio_tungstenite::connect_async(&browser_ws_url)
            .await
            .map_err(|e| CdpError::ConnectionFailed(format!("WebSocket: {}", e)))?;

        let (ws_sink, ws_source) = ws_stream.split();
        let ws_tx = Arc::new(tokio::sync::Mutex::new(ws_sink));
        let pending: Arc<Mutex<HashMap<u64, PendingRequest>>> = Arc::new(Mutex::new(HashMap::new()));
        let event_handlers: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<CdpResponse>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Start receive task
        let recv_task = {
            let pending = pending.clone();
            let event_handlers = event_handlers.clone();
            tokio::spawn(async move {
                Self::receive_loop(ws_source, pending, event_handlers).await;
            })
        };

        debug!("CDP client connected to {}", browser_ws_url);

        Ok(Self {
            http_endpoint,
            browser_ws_url,
            ws_tx,
            request_id: Arc::new(AtomicU64::new(1)),
            pending,
            event_handlers,
            _recv_task: recv_task,
        })
    }

    /// WebSocket receive loop.
    async fn receive_loop(
        mut ws_source: WsSource,
        pending: Arc<Mutex<HashMap<u64, PendingRequest>>>,
        event_handlers: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<CdpResponse>>>>,
    ) {
        while let Some(msg) = ws_source.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    trace!("CDP recv: {}", text);
                    match serde_json::from_str::<CdpResponse>(&text) {
                        Ok(resp) => {
                            // Check if it's a response to a request
                            if let Some(id) = resp.id {
                                let pending_req = pending.lock().remove(&id);
                                if let Some(req) = pending_req {
                                    let result = if let Some(error) = resp.error {
                                        Err(CdpError::Protocol {
                                            code: error.code,
                                            message: error.message,
                                        })
                                    } else {
                                        Ok(resp.result.unwrap_or(Value::Null))
                                    };
                                    let _ = req.tx.send(result);
                                }
                            } else if resp.method.is_some() {
                                // It's an event
                                let session_id = resp.session_id.clone().unwrap_or_default();
                                let handlers = event_handlers.read().await;
                                if let Some(tx) = handlers.get(&session_id) {
                                    let _ = tx.send(resp);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse CDP message: {}", e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    debug!("WebSocket closed");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    }

    /// Send a CDP command and wait for response.
    pub async fn call(
        &self,
        method: &str,
        params: Option<Value>,
        session_id: Option<&str>,
    ) -> Result<Value, CdpError> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);

        let request = CdpRequest {
            id,
            method: method.to_string(),
            params,
            session_id: session_id.map(|s| s.to_string()),
        };

        let json = serde_json::to_string(&request)?;
        trace!("CDP send: {}", json);

        // Create response channel
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(id, PendingRequest { tx });

        // Send request
        {
            let mut ws = self.ws_tx.lock().await;
            ws.send(Message::Text(json.into())).await?;
        }

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(CdpError::SessionClosed),
            Err(_) => {
                self.pending.lock().remove(&id);
                Err(CdpError::Timeout(format!("Request {} timed out", method)))
            }
        }
    }

    /// Get browser WebSocket URL.
    pub fn browser_ws_url(&self) -> &str {
        &self.browser_ws_url
    }

    // ========================================================================
    // Target Management
    // ========================================================================

    /// List all pages.
    pub async fn list_pages(&self) -> Result<Vec<PageInfo>, CdpError> {
        let url = format!("{}/json/list", self.http_endpoint);
        let pages: Vec<PageInfo> = reqwest::get(&url).await?.json().await?;
        Ok(pages)
    }

    /// Create a new page/tab.
    pub async fn new_page(&self, url: Option<&str>) -> Result<PageSession, CdpError> {
        // Chrome requires PUT method for /json/new
        let create_url = if let Some(u) = url {
            format!("{}/json/new?{}", self.http_endpoint, u)
        } else {
            format!("{}/json/new", self.http_endpoint)
        };

        let client = reqwest::Client::new();
        let page_info: PageInfo = client.put(&create_url).send().await?.json().await?;
        debug!("Created new page: {} - {}", page_info.id, page_info.url);

        // Attach to target
        let result = self
            .call(
                "Target.attachToTarget",
                Some(json!({
                    "targetId": page_info.id,
                    "flatten": true
                })),
                None,
            )
            .await?;

        let session_id = result["sessionId"]
            .as_str()
            .ok_or_else(|| CdpError::InvalidResponse("Missing sessionId".to_string()))?
            .to_string();

        // Create event channel for this session
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        self.event_handlers
            .write()
            .await
            .insert(session_id.clone(), event_tx);

        // Enable required domains
        let session = PageSession::new(
            page_info.id.clone(),
            session_id,
            self.ws_tx.clone(),
            self.pending.clone(),
            self.request_id.clone(),
            event_rx,
        );

        session.enable_domains().await?;

        Ok(session)
    }

    /// Attach to an existing page.
    pub async fn attach_page(&self, target_id: &str) -> Result<PageSession, CdpError> {
        let result = self
            .call(
                "Target.attachToTarget",
                Some(json!({
                    "targetId": target_id,
                    "flatten": true
                })),
                None,
            )
            .await?;

        let session_id = result["sessionId"]
            .as_str()
            .ok_or_else(|| CdpError::InvalidResponse("Missing sessionId".to_string()))?
            .to_string();

        // Create event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        self.event_handlers
            .write()
            .await
            .insert(session_id.clone(), event_tx);

        let session = PageSession::new(
            target_id.to_string(),
            session_id,
            self.ws_tx.clone(),
            self.pending.clone(),
            self.request_id.clone(),
            event_rx,
        );

        session.enable_domains().await?;

        Ok(session)
    }

    /// Get all targets.
    pub async fn get_targets(&self) -> Result<Vec<TargetInfo>, CdpError> {
        let result = self.call("Target.getTargets", None, None).await?;
        let targets: Vec<TargetInfo> = serde_json::from_value(result["targetInfos"].clone())?;
        Ok(targets)
    }

    /// Close a page/target.
    pub async fn close_page(&self, target_id: &str) -> Result<(), CdpError> {
        self.call(
            "Target.closeTarget",
            Some(json!({"targetId": target_id})),
            None,
        )
        .await?;
        Ok(())
    }
}

impl Drop for CdpClient {
    fn drop(&mut self) {
        self._recv_task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_increment() {
        let id = AtomicU64::new(1);
        assert_eq!(id.fetch_add(1, Ordering::SeqCst), 1);
        assert_eq!(id.fetch_add(1, Ordering::SeqCst), 2);
        assert_eq!(id.load(Ordering::SeqCst), 3);
    }
}
