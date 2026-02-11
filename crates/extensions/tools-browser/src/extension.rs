//! Browser tools extension.

use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::provider::LLMProvider;
use autohands_protocols::types::Version;

use crate::ai_tools::{AiClickTool, AiExtractTool, AiFillTool, VisionProvider};
use crate::manager::{BrowserManager, BrowserManagerConfig};
use crate::tools::*;

/// Configuration for AI-powered browser tools.
#[derive(Clone)]
pub struct AiToolsConfig {
    /// Vision-capable LLM provider.
    pub provider: Arc<dyn LLMProvider>,
    /// Model name to use for vision tasks.
    pub model: String,
}

/// Browser tools extension.
///
/// Provides browser automation tools via Chrome DevTools Protocol (CDP).
/// Chrome is automatically launched when tools are first used, using a
/// persistent profile at `~/.autohands/browser-profile` to preserve logins.
pub struct BrowserToolsExtension {
    manifest: ExtensionManifest,
    config: BrowserManagerConfig,
    ai_config: Option<AiToolsConfig>,
    manager: Option<Arc<BrowserManager>>,
}

impl BrowserToolsExtension {
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "tools-browser",
            "Browser Tools",
            Version::new(0, 4, 0),
        );
        manifest.description =
            "Browser automation via CDP - auto-launches Chrome with persistent profile".to_string();
        manifest.provides = Provides {
            tools: vec![
                "browser_open".to_string(),
                "browser_close".to_string(),
                "browser_list_pages".to_string(),
                "browser_navigate".to_string(),
                "browser_click".to_string(),
                "browser_type".to_string(),
                "browser_screenshot".to_string(),
                "browser_get_content".to_string(),
                "browser_get_url".to_string(),
                "browser_execute_js".to_string(),
                "browser_wait_for".to_string(),
                "browser_scroll".to_string(),
                "browser_press_key".to_string(),
                "browser_back".to_string(),
                "browser_forward".to_string(),
                "browser_refresh".to_string(),
                // DOM analysis tool (Browser-Use style)
                "browser_get_dom".to_string(),
                // AI-powered tools (optional, require vision provider)
                "browser_ai_click".to_string(),
                "browser_ai_fill".to_string(),
                "browser_ai_extract".to_string(),
            ],
            ..Default::default()
        };

        Self {
            manifest,
            config: BrowserManagerConfig::default(),
            ai_config: None,
            manager: None,
        }
    }

    /// Set viewport size.
    pub fn viewport(mut self, width: u32, height: u32) -> Self {
        self.config.viewport_width = width;
        self.config.viewport_height = height;
        self
    }

    /// Set the Chrome debugging port.
    /// Default: 9222
    pub fn debug_port(mut self, port: u16) -> Self {
        self.config.debug_port = port;
        self
    }

    /// Set custom profile directory for persistent login state.
    /// Default: ~/.autohands/browser-profile
    pub fn profile_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.profile_dir = Some(path.into());
        self
    }

    /// Enable headless mode.
    pub fn headless(mut self, headless: bool) -> Self {
        self.config.headless = headless;
        self
    }

    /// Configure AI-powered browser tools with a vision-capable LLM provider.
    ///
    /// This enables `browser_ai_click`, `browser_ai_fill`, and `browser_ai_extract`
    /// tools which use vision models to identify page elements by natural language
    /// description.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let ext = BrowserToolsExtension::new()
    ///     .with_vision_provider(openai_provider.clone(), "gpt-4o");
    /// ```
    pub fn with_vision_provider(
        mut self,
        provider: Arc<dyn LLMProvider>,
        model: impl Into<String>,
    ) -> Self {
        self.ai_config = Some(AiToolsConfig {
            provider,
            model: model.into(),
        });
        self
    }

    /// Get the browser manager.
    pub fn manager(&self) -> Option<Arc<BrowserManager>> {
        self.manager.clone()
    }
}

impl Default for BrowserToolsExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for BrowserToolsExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        // Create browser manager but DO NOT connect yet.
        // Chrome will be lazily launched on first tool use.
        let manager = Arc::new(BrowserManager::new(self.config.clone()));

        // Register tools - browser will connect when browser_open is first used
        ctx.tool_registry
            .register_tool(Arc::new(OpenPageTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(ClosePageTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(ListPagesTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(NavigateTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(ClickTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(TypeTextTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(ScreenshotTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(GetContentTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(GetUrlTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(ExecuteJsTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(WaitForTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(ScrollTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(PressKeyTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(BackTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(ForwardTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(RefreshTool::new(manager.clone())))?;

        // Register DOM analysis tool (Browser-Use style)
        ctx.tool_registry
            .register_tool(Arc::new(GetDomTool::new(manager.clone())))?;

        // Register AI-powered tools if vision provider is configured
        if let Some(ref ai_config) = self.ai_config {
            let vision =
                Arc::new(VisionProvider::new(ai_config.provider.clone(), &ai_config.model));

            ctx.tool_registry
                .register_tool(Arc::new(AiClickTool::new(manager.clone(), vision.clone())))?;
            ctx.tool_registry
                .register_tool(Arc::new(AiFillTool::new(manager.clone(), vision.clone())))?;
            ctx.tool_registry
                .register_tool(Arc::new(AiExtractTool::new(manager.clone(), vision.clone())))?;

            tracing::info!(
                "AI browser tools enabled with model: {}",
                ai_config.model
            );
        }

        self.manager = Some(manager);

        tracing::info!(
            "Browser tools extension initialized (profile: {})",
            self.config.get_profile_dir().display()
        );
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), ExtensionError> {
        if let Some(ref manager) = self.manager {
            manager
                .shutdown_chrome()
                .await
                .map_err(|e| ExtensionError::ShutdownFailed(e.to_string()))?;
        }
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
#[path = "extension_tests.rs"]
mod tests;
