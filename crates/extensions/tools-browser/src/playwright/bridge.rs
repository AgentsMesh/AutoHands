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

use super::dom::EnhancedNodeTree;
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
    #[serde(default)]
    #[allow(dead_code)]
    code: Option<i32>,
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
    config: PlaywrightBridgeConfig,
    process: Mutex<Option<Child>>,
    stdin: Mutex<Option<tokio::process::ChildStdin>>,
    request_id: AtomicU64,
    pending_requests: Arc<RwLock<PendingRequests>>,
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
            bridge_script: Self::generate_bridge_script(),
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
    // Browser Methods
    // ============================================================================

    /// Launch a new browser.
    pub async fn launch_browser(
        &self,
        headless: bool,
        args: Vec<String>,
    ) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "launchBrowser",
                serde_json::json!({
                    "headless": headless,
                    "args": args
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::BrowserLaunchFailed("Invalid response".to_string()))
    }

    /// Connect to an existing browser via CDP.
    pub async fn connect_browser(&self, endpoint: &str) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "connectBrowser",
                serde_json::json!({
                    "endpoint": endpoint
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::BrowserConnectFailed("Invalid response".to_string()))
    }

    /// Close browser.
    pub async fn close_browser(&self, browser_id: &str) -> Result<(), PlaywrightError> {
        self.call(
            "closeBrowser",
            serde_json::json!({
                "browserId": browser_id
            }),
        )
        .await?;
        Ok(())
    }

    // ============================================================================
    // Page Methods
    // ============================================================================

    /// Create a new page.
    pub async fn new_page(&self, browser_id: &str) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "newPage",
                serde_json::json!({
                    "browserId": browser_id
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::ActionFailed("Invalid page ID response".to_string()))
    }

    /// Navigate to URL.
    pub async fn navigate(
        &self,
        page_id: &str,
        url: &str,
        wait_until: Option<&str>,
    ) -> Result<(), PlaywrightError> {
        self.call(
            "navigate",
            serde_json::json!({
                "pageId": page_id,
                "url": url,
                "waitUntil": wait_until.unwrap_or("domcontentloaded")
            }),
        )
        .await?;
        Ok(())
    }

    /// Click at coordinates.
    pub async fn click(&self, page_id: &str, x: f64, y: f64) -> Result<(), PlaywrightError> {
        self.call(
            "click",
            serde_json::json!({
                "pageId": page_id,
                "x": x,
                "y": y
            }),
        )
        .await?;
        Ok(())
    }

    /// Click on selector.
    pub async fn click_selector(
        &self,
        page_id: &str,
        selector: &str,
    ) -> Result<(), PlaywrightError> {
        self.call(
            "clickSelector",
            serde_json::json!({
                "pageId": page_id,
                "selector": selector
            }),
        )
        .await?;
        Ok(())
    }

    /// Type text.
    pub async fn type_text(
        &self,
        page_id: &str,
        text: &str,
        delay_ms: Option<u32>,
    ) -> Result<(), PlaywrightError> {
        self.call(
            "typeText",
            serde_json::json!({
                "pageId": page_id,
                "text": text,
                "delay": delay_ms.unwrap_or(0)
            }),
        )
        .await?;
        Ok(())
    }

    /// Fill input at selector.
    pub async fn fill(
        &self,
        page_id: &str,
        selector: &str,
        value: &str,
    ) -> Result<(), PlaywrightError> {
        self.call(
            "fill",
            serde_json::json!({
                "pageId": page_id,
                "selector": selector,
                "value": value
            }),
        )
        .await?;
        Ok(())
    }

    /// Press key.
    pub async fn press_key(&self, page_id: &str, key: &str) -> Result<(), PlaywrightError> {
        self.call(
            "pressKey",
            serde_json::json!({
                "pageId": page_id,
                "key": key
            }),
        )
        .await?;
        Ok(())
    }

    /// Take screenshot.
    pub async fn screenshot(
        &self,
        page_id: &str,
        options: Option<ScreenshotOptions>,
    ) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "screenshot",
                serde_json::json!({
                    "pageId": page_id,
                    "options": options
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::ScreenshotFailed("Invalid response".to_string()))
    }

    /// Get page content.
    pub async fn get_content(&self, page_id: &str) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "getContent",
                serde_json::json!({
                    "pageId": page_id
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::ActionFailed("Invalid content response".to_string()))
    }

    /// Get page URL.
    pub async fn get_url(&self, page_id: &str) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "getUrl",
                serde_json::json!({
                    "pageId": page_id
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::ActionFailed("Invalid URL response".to_string()))
    }

    /// Get page title.
    pub async fn get_title(&self, page_id: &str) -> Result<String, PlaywrightError> {
        let result = self
            .call(
                "getTitle",
                serde_json::json!({
                    "pageId": page_id
                }),
            )
            .await?;

        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PlaywrightError::ActionFailed("Invalid title response".to_string()))
    }

    /// Execute JavaScript.
    pub async fn evaluate(
        &self,
        page_id: &str,
        script: &str,
    ) -> Result<serde_json::Value, PlaywrightError> {
        self.call(
            "evaluate",
            serde_json::json!({
                "pageId": page_id,
                "script": script
            }),
        )
        .await
    }

    /// Wait for selector.
    pub async fn wait_for_selector(
        &self,
        page_id: &str,
        selector: &str,
        timeout_ms: Option<u32>,
    ) -> Result<(), PlaywrightError> {
        self.call(
            "waitForSelector",
            serde_json::json!({
                "pageId": page_id,
                "selector": selector,
                "timeout": timeout_ms.unwrap_or(30000)
            }),
        )
        .await?;
        Ok(())
    }

    /// Close page.
    pub async fn close_page(&self, page_id: &str) -> Result<(), PlaywrightError> {
        self.call(
            "closePage",
            serde_json::json!({
                "pageId": page_id
            }),
        )
        .await?;
        Ok(())
    }

    /// Go back.
    pub async fn go_back(&self, page_id: &str) -> Result<(), PlaywrightError> {
        self.call(
            "goBack",
            serde_json::json!({
                "pageId": page_id
            }),
        )
        .await?;
        Ok(())
    }

    /// Go forward.
    pub async fn go_forward(&self, page_id: &str) -> Result<(), PlaywrightError> {
        self.call(
            "goForward",
            serde_json::json!({
                "pageId": page_id
            }),
        )
        .await?;
        Ok(())
    }

    /// Reload page.
    pub async fn reload(&self, page_id: &str) -> Result<(), PlaywrightError> {
        self.call(
            "reload",
            serde_json::json!({
                "pageId": page_id
            }),
        )
        .await?;
        Ok(())
    }

    /// Scroll page.
    pub async fn scroll(
        &self,
        page_id: &str,
        x: f64,
        y: f64,
    ) -> Result<(), PlaywrightError> {
        self.call(
            "scroll",
            serde_json::json!({
                "pageId": page_id,
                "x": x,
                "y": y
            }),
        )
        .await?;
        Ok(())
    }

    // ============================================================================
    // DOM Analysis Methods (Browser-Use Style)
    // ============================================================================

    /// Get enhanced DOM tree with clickability analysis.
    pub async fn get_dom_tree(&self, page_id: &str) -> Result<EnhancedNodeTree, PlaywrightError> {
        let result = self
            .call(
                "getDomTree",
                serde_json::json!({
                    "pageId": page_id
                }),
            )
            .await?;

        serde_json::from_value(result)
            .map_err(|e| PlaywrightError::DomProcessingError(format!("Failed to parse DOM tree: {}", e)))
    }

    /// Get element at coordinates.
    pub async fn element_at(
        &self,
        page_id: &str,
        x: f64,
        y: f64,
    ) -> Result<Option<serde_json::Value>, PlaywrightError> {
        let result = self
            .call(
                "elementAt",
                serde_json::json!({
                    "pageId": page_id,
                    "x": x,
                    "y": y
                }),
            )
            .await?;

        if result.is_null() {
            Ok(None)
        } else {
            Ok(Some(result))
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

    /// Generate the bridge JavaScript code.
    fn generate_bridge_script() -> String {
        r#"
const { chromium } = require('playwright');
const readline = require('readline');

// State
const browsers = new Map();
const pages = new Map();
let idCounter = 1;

// JSON-RPC handler
async function handleRequest(request) {
    const { id, method, params } = request;

    try {
        let result;
        switch (method) {
            case 'ping':
                result = 'pong';
                break;

            case 'shutdown':
                // Close all browsers
                for (const browser of browsers.values()) {
                    await browser.close().catch(() => {});
                }
                process.exit(0);
                break;

            case 'launchBrowser':
                result = await launchBrowser(params);
                break;

            case 'connectBrowser':
                result = await connectBrowser(params);
                break;

            case 'closeBrowser':
                await closeBrowser(params);
                result = null;
                break;

            case 'newPage':
                result = await newPage(params);
                break;

            case 'navigate':
                await navigate(params);
                result = null;
                break;

            case 'click':
                await click(params);
                result = null;
                break;

            case 'clickSelector':
                await clickSelector(params);
                result = null;
                break;

            case 'typeText':
                await typeText(params);
                result = null;
                break;

            case 'fill':
                await fill(params);
                result = null;
                break;

            case 'pressKey':
                await pressKey(params);
                result = null;
                break;

            case 'screenshot':
                result = await screenshot(params);
                break;

            case 'getContent':
                result = await getContent(params);
                break;

            case 'getUrl':
                result = await getUrl(params);
                break;

            case 'getTitle':
                result = await getTitle(params);
                break;

            case 'evaluate':
                result = await evaluate(params);
                break;

            case 'waitForSelector':
                await waitForSelector(params);
                result = null;
                break;

            case 'closePage':
                await closePage(params);
                result = null;
                break;

            case 'goBack':
                await goBack(params);
                result = null;
                break;

            case 'goForward':
                await goForward(params);
                result = null;
                break;

            case 'reload':
                await reload(params);
                result = null;
                break;

            case 'scroll':
                await scroll(params);
                result = null;
                break;

            case 'getDomTree':
                result = await getDomTree(params);
                break;

            case 'elementAt':
                result = await elementAt(params);
                break;

            default:
                throw new Error(`Unknown method: ${method}`);
        }

        return { id, result };
    } catch (error) {
        return {
            id,
            error: {
                message: error.message,
                code: -1
            }
        };
    }
}

// Browser methods
async function launchBrowser({ headless = true, args = [] }) {
    const browser = await chromium.launch({
        headless,
        args: [
            '--disable-blink-features=AutomationControlled',
            '--disable-gpu',
            '--no-sandbox',
            ...args
        ]
    });

    const browserId = `browser_${idCounter++}`;
    browsers.set(browserId, browser);
    return browserId;
}

async function connectBrowser({ endpoint }) {
    const browser = await chromium.connectOverCDP(endpoint);
    const browserId = `browser_${idCounter++}`;
    browsers.set(browserId, browser);
    return browserId;
}

async function closeBrowser({ browserId }) {
    const browser = browsers.get(browserId);
    if (browser) {
        await browser.close();
        browsers.delete(browserId);
    }
}

// Page methods
async function newPage({ browserId }) {
    const browser = browsers.get(browserId);
    if (!browser) throw new Error(`Browser not found: ${browserId}`);

    const context = await browser.newContext({
        viewport: { width: 1280, height: 720 }
    });
    const page = await context.newPage();

    const pageId = `page_${idCounter++}`;
    pages.set(pageId, { page, context, browserId });
    return pageId;
}

async function navigate({ pageId, url, waitUntil = 'domcontentloaded' }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.goto(url, { waitUntil });
}

async function click({ pageId, x, y }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.mouse.click(x, y);
}

async function clickSelector({ pageId, selector }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.click(selector);
}

async function typeText({ pageId, text, delay = 0 }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.keyboard.type(text, { delay });
}

async function fill({ pageId, selector, value }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.fill(selector, value);
}

async function pressKey({ pageId, key }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.keyboard.press(key);
}

async function screenshot({ pageId, options = {} }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    const buffer = await pageData.page.screenshot({
        type: options.type || 'png',
        fullPage: options.fullPage || false,
        quality: options.quality,
        clip: options.clip
    });

    return buffer.toString('base64');
}

async function getContent({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    return await pageData.page.content();
}

async function getUrl({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    return pageData.page.url();
}

async function getTitle({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    return await pageData.page.title();
}

async function evaluate({ pageId, script }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    return await pageData.page.evaluate(script);
}

async function waitForSelector({ pageId, selector, timeout = 30000 }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.waitForSelector(selector, { timeout });
}

async function closePage({ pageId }) {
    const pageData = pages.get(pageId);
    if (pageData) {
        await pageData.page.close();
        await pageData.context.close();
        pages.delete(pageId);
    }
}

async function goBack({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.goBack();
}

async function goForward({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.goForward();
}

async function reload({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.reload();
}

async function scroll({ pageId, x, y }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.evaluate(({ x, y }) => {
        window.scrollBy(x, y);
    }, { x, y });
}

// DOM Analysis (Browser-Use Style)
async function getDomTree({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    const page = pageData.page;

    // Get viewport info
    const viewport = page.viewportSize() || { width: 1280, height: 720 };
    const scrollInfo = await page.evaluate(() => ({
        x: window.scrollX,
        y: window.scrollY,
        devicePixelRatio: window.devicePixelRatio
    }));

    // Get all interactive elements with their properties
    const nodes = await page.evaluate(() => {
        const results = [];
        let nodeId = 0;

        // Interactive selectors
        const interactiveSelectors = [
            'a', 'button', 'input', 'select', 'textarea', 'option',
            '[role="button"]', '[role="link"]', '[role="checkbox"]',
            '[role="radio"]', '[role="menuitem"]', '[role="tab"]',
            '[onclick]', '[tabindex]', '[contenteditable="true"]'
        ];

        // 10-layer clickable detection
        function calculateClickability(el) {
            let score = 0;
            const reasons = [];

            const tag = el.tagName.toLowerCase();
            const style = window.getComputedStyle(el);
            const role = el.getAttribute('role');

            // Layer 1: Native interactive tags
            const interactiveTags = ['a', 'button', 'input', 'select', 'textarea', 'option', 'label'];
            if (interactiveTags.includes(tag)) {
                score += 0.3;
                reasons.push(`native_tag:${tag}`);
            }

            // Layer 2: ARIA roles
            const clickableRoles = ['button', 'link', 'checkbox', 'radio', 'menuitem', 'tab', 'option', 'switch'];
            if (role && clickableRoles.includes(role)) {
                score += 0.2;
                reasons.push(`aria_role:${role}`);
            }

            // Layer 3: Cursor pointer
            if (style.cursor === 'pointer') {
                score += 0.15;
                reasons.push('cursor_pointer');
            }

            // Layer 4: Has href
            if (el.hasAttribute('href')) {
                score += 0.2;
                reasons.push('has_href');
            }

            // Layer 5: Event handlers
            const eventAttrs = ['onclick', 'onmousedown', 'onmouseup', 'ontouchstart'];
            for (const attr of eventAttrs) {
                if (el.hasAttribute(attr)) {
                    score += 0.15;
                    reasons.push(`event:${attr}`);
                    break;
                }
            }

            // Layer 6: Tabindex
            const tabindex = el.getAttribute('tabindex');
            if (tabindex !== null && parseInt(tabindex) >= 0) {
                score += 0.1;
                reasons.push('tabindex');
            }

            // Layer 7: Input type
            if (tag === 'input') {
                const type = el.getAttribute('type') || 'text';
                const clickableTypes = ['button', 'submit', 'reset', 'checkbox', 'radio', 'file'];
                if (clickableTypes.includes(type)) {
                    score += 0.15;
                    reasons.push(`input_type:${type}`);
                }
            }

            // Layer 8: Contenteditable
            if (el.isContentEditable) {
                score += 0.2;
                reasons.push('contenteditable');
            }

            return { score: Math.min(score, 1.0), reasons };
        }

        // Get element info
        function getElementInfo(el) {
            const rect = el.getBoundingClientRect();
            const style = window.getComputedStyle(el);

            // Check visibility
            const isVisible = rect.width > 0 &&
                rect.height > 0 &&
                style.visibility !== 'hidden' &&
                style.display !== 'none' &&
                style.opacity !== '0';

            // Check if in viewport
            const isInViewport = rect.top < window.innerHeight &&
                rect.bottom > 0 &&
                rect.left < window.innerWidth &&
                rect.right > 0;

            const { score, reasons } = calculateClickability(el);

            // Get text content (direct text only)
            let textContent = '';
            for (const node of el.childNodes) {
                if (node.nodeType === Node.TEXT_NODE) {
                    textContent += node.textContent;
                }
            }
            textContent = textContent.trim().substring(0, 200);

            // Get attributes
            const attrs = {};
            for (const attr of el.attributes) {
                if (attr.name.startsWith('data-')) {
                    if (!attrs.data) attrs.data = {};
                    attrs.data[attr.name.substring(5)] = attr.value;
                } else {
                    const attrMap = {
                        'id': 'id', 'class': 'class', 'href': 'href', 'src': 'src',
                        'alt': 'alt', 'title': 'title', 'placeholder': 'placeholder',
                        'value': 'value', 'type': 'type', 'name': 'name', 'role': 'role',
                        'aria-label': 'aria_label', 'aria-expanded': 'aria_expanded',
                        'aria-selected': 'aria_selected'
                    };
                    if (attrMap[attr.name]) {
                        attrs[attrMap[attr.name]] = attr.value;
                    }
                }
            }

            // Generate unique selector
            let cssSelector = el.tagName.toLowerCase();
            if (el.id) {
                cssSelector = `#${el.id}`;
            } else if (el.className) {
                cssSelector += '.' + el.className.split(' ').filter(c => c).join('.');
            }

            // Generate XPath
            function getXPath(element) {
                if (element.id) return `//*[@id="${element.id}"]`;

                const parts = [];
                let current = element;
                while (current && current.nodeType === Node.ELEMENT_NODE) {
                    let index = 0;
                    let sibling = current.previousSibling;
                    while (sibling) {
                        if (sibling.nodeType === Node.ELEMENT_NODE &&
                            sibling.tagName === current.tagName) {
                            index++;
                        }
                        sibling = sibling.previousSibling;
                    }
                    const tag = current.tagName.toLowerCase();
                    parts.unshift(index > 0 ? `${tag}[${index + 1}]` : tag);
                    current = current.parentElement;
                }
                return '/' + parts.join('/');
            }

            return {
                id: `node_${nodeId++}`,
                backend_node_id: nodeId,
                tag_name: el.tagName.toLowerCase(),
                attributes: attrs,
                text_content: textContent,
                bounding_box: {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height
                },
                is_visible: isVisible,
                is_in_viewport: isInViewport,
                clickability_score: score,
                clickability_reasons: reasons,
                paint_order: 0, // Would need layerTree for accurate z-index
                is_interactive: score > 0.3,
                is_focusable: el.tabIndex >= 0,
                parent_id: null,
                children: [],
                xpath: getXPath(el),
                css_selector: cssSelector,
                computed_styles: {
                    cursor: style.cursor,
                    display: style.display,
                    visibility: style.visibility
                }
            };
        }

        // Collect all potentially interactive elements
        const elements = document.querySelectorAll(interactiveSelectors.join(','));
        for (const el of elements) {
            try {
                const info = getElementInfo(el);
                if (info.is_visible) {
                    results.push(info);
                }
            } catch (e) {
                // Skip problematic elements
            }
        }

        return results;
    });

    // Build node map
    const nodeMap = {};
    for (const node of nodes) {
        nodeMap[node.id] = node;
    }

    return {
        roots: nodes.filter(n => !n.parent_id).map(n => n.id),
        nodes: nodeMap,
        viewport: {
            width: viewport.width,
            height: viewport.height,
            device_pixel_ratio: scrollInfo.devicePixelRatio || 1,
            scroll_x: scrollInfo.x || 0,
            scroll_y: scrollInfo.y || 0
        },
        timestamp: Date.now(),
        url: page.url(),
        title: await page.title()
    };
}

async function elementAt({ pageId, x, y }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    return await pageData.page.evaluate(({ x, y }) => {
        const el = document.elementFromPoint(x, y);
        if (!el) return null;

        return {
            tagName: el.tagName.toLowerCase(),
            id: el.id || null,
            className: el.className || null,
            textContent: el.textContent?.substring(0, 100) || null
        };
    }, { x, y });
}

// Main loop
const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: false
});

rl.on('line', async (line) => {
    try {
        const request = JSON.parse(line);
        const response = await handleRequest(request);
        console.log(JSON.stringify(response));
    } catch (error) {
        console.log(JSON.stringify({
            id: 0,
            error: { message: error.message, code: -1 }
        }));
    }
});

// Handle uncaught errors
process.on('uncaughtException', (error) => {
    console.error('Uncaught exception:', error);
});

process.on('unhandledRejection', (error) => {
    console.error('Unhandled rejection:', error);
});

console.error('Playwright bridge started');
"#.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = PlaywrightBridgeConfig::default();
        assert!(config.node_path.is_none());
        assert_eq!(config.response_timeout_ms, 30000);
    }

    #[test]
    fn test_screenshot_options_serialize() {
        let options = ScreenshotOptions {
            full_page: Some(true),
            clip: None,
            quality: Some(80),
            format: Some("jpeg".to_string()),
        };

        let json = serde_json::to_string(&options).unwrap();
        assert!(json.contains("fullPage"));
        assert!(json.contains("true"));
        assert!(json.contains("quality"));
        assert!(json.contains("80"));
    }

    #[test]
    fn test_clip_region_serialize() {
        let clip = ClipRegion {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };

        let json = serde_json::to_string(&clip).unwrap();
        assert!(json.contains("10"));
        assert!(json.contains("100"));
    }
}
