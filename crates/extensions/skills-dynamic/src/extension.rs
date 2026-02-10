//! Dynamic skills extension for AutoHands.
//!
//! Provides runtime skill loading with hot-reload support.

use std::any::Any;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;

use crate::loader::{DynamicSkillLoader, SkillSource};
use crate::registry::SkillRegistry;
use crate::skill_tools::{
    SkillContentTool, SkillInfoTool, SkillListTool, SkillReadTool, SkillReloadTool,
};

/// Dynamic skills extension configuration.
#[derive(Debug, Clone)]
pub struct DynamicSkillsConfig {
    /// Additional skill directories.
    pub extra_dirs: Vec<PathBuf>,
    /// Enable hot-reload.
    pub hot_reload: bool,
    /// Use managed skills directory (~/.autohands/skills/).
    pub use_managed: bool,
    /// Use workspace skills directory (<cwd>/skills/).
    pub use_workspace: bool,
}

impl Default for DynamicSkillsConfig {
    fn default() -> Self {
        Self {
            extra_dirs: Vec::new(),
            hot_reload: true,
            use_managed: true,
            use_workspace: true,
        }
    }
}

/// Dynamic skills extension.
pub struct DynamicSkillsExtension {
    manifest: ExtensionManifest,
    pub(crate) config: DynamicSkillsConfig,
    loader: Option<Arc<RwLock<DynamicSkillLoader>>>,
    registry: Arc<SkillRegistry>,
}

impl DynamicSkillsExtension {
    /// Create a new dynamic skills extension with default configuration.
    pub fn new() -> Self {
        Self::with_config(DynamicSkillsConfig::default())
    }

    /// Create a new dynamic skills extension with custom configuration.
    pub fn with_config(config: DynamicSkillsConfig) -> Self {
        let mut manifest = ExtensionManifest::new(
            "skills-dynamic",
            "Dynamic Skills",
            Version::new(0, 1, 0),
        );
        manifest.description = "Dynamic skill loading with hot-reload support".to_string();
        manifest.provides = Provides {
            skills: Vec::new(), // Will be populated after loading
            tools: vec![
                "skill_list".to_string(),
                "skill_info".to_string(),
                "skill_read".to_string(),
                "skill_content".to_string(),
                "skill_reload".to_string(),
            ],
            ..Default::default()
        };

        Self {
            manifest,
            config,
            loader: None,
            registry: Arc::new(SkillRegistry::new()),
        }
    }

    /// Add an extra skill directory.
    pub fn with_extra_dir(mut self, dir: PathBuf) -> Self {
        self.config.extra_dirs.push(dir);
        self
    }

    /// Enable or disable hot-reload.
    pub fn with_hot_reload(mut self, enabled: bool) -> Self {
        self.config.hot_reload = enabled;
        self
    }

    /// Get the skill registry.
    pub fn registry(&self) -> Arc<SkillRegistry> {
        self.registry.clone()
    }

    /// Get the skill loader.
    pub fn loader(&self) -> Option<Arc<RwLock<DynamicSkillLoader>>> {
        self.loader.clone()
    }

    /// Register skill-related tools.
    fn register_skill_tools(&self, ctx: &ExtensionContext) -> Result<(), ExtensionError> {
        let registry = self.registry.clone();
        let list_tool = SkillListTool::new(registry);
        ctx.tool_registry
            .register_tool(Arc::new(list_tool))
            .map_err(|e| ExtensionError::InitializationFailed(format!("Failed to register skill_list: {}", e)))?;

        let registry = self.registry.clone();
        let info_tool = SkillInfoTool::new(registry);
        ctx.tool_registry
            .register_tool(Arc::new(info_tool))
            .map_err(|e| ExtensionError::InitializationFailed(format!("Failed to register skill_info: {}", e)))?;

        let registry = self.registry.clone();
        let read_tool = SkillReadTool::new(registry);
        ctx.tool_registry
            .register_tool(Arc::new(read_tool))
            .map_err(|e| ExtensionError::InitializationFailed(format!("Failed to register skill_read: {}", e)))?;

        let registry = self.registry.clone();
        let content_tool = SkillContentTool::new(registry);
        ctx.tool_registry
            .register_tool(Arc::new(content_tool))
            .map_err(|e| ExtensionError::InitializationFailed(format!("Failed to register skill_content: {}", e)))?;

        if let Some(loader) = &self.loader {
            let reload_tool = SkillReloadTool::new(loader.clone(), self.registry.clone());
            ctx.tool_registry
                .register_tool(Arc::new(reload_tool))
                .map_err(|e| ExtensionError::InitializationFailed(format!("Failed to register skill_reload: {}", e)))?;
        }

        debug!("Registered skill tools");
        Ok(())
    }
}

impl Default for DynamicSkillsExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for DynamicSkillsExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        info!("Initializing dynamic skills extension");

        let mut loader = DynamicSkillLoader::new();

        for dir in &self.config.extra_dirs {
            loader = loader.with_source(SkillSource::Directory(dir.clone()));
            debug!("Added extra skills directory: {}", dir.display());
        }

        if self.config.use_managed {
            if let Some(home) = dirs::home_dir() {
                let managed = home.join(".autohands").join("skills");
                if !managed.exists() {
                    if let Err(e) = std::fs::create_dir_all(&managed) {
                        warn!("Failed to create managed skills directory: {}", e);
                    } else {
                        info!("Created managed skills directory: {}", managed.display());
                    }
                }
                loader = loader.with_source(SkillSource::Managed(managed.clone()));
                debug!("Added managed skills directory: {}", managed.display());
            }
        }

        if self.config.use_workspace {
            let workspace = ctx.work_dir.join("skills");
            if workspace.exists() {
                loader = loader.with_source(SkillSource::Workspace(workspace.clone()));
                debug!("Added workspace skills directory: {}", workspace.display());
            }
        }

        loader.set_available_tools(Vec::new()).await;

        if let Err(e) = loader.load_all().await {
            warn!("Failed to load skills: {}", e);
        }

        {
            use autohands_protocols::skill::SkillLoader;
            let skills = loader.list().await.unwrap_or_default();
            for def in &skills {
                if let Ok(skill) = loader.load(&def.id).await {
                    self.registry.register(skill).await;
                }
            }
            info!("Registered {} dynamic skills", skills.len());
            self.manifest.provides.skills = skills.iter().map(|s| s.id.clone()).collect();
        }

        let loader = Arc::new(RwLock::new(loader));
        self.loader = Some(loader.clone());

        if self.config.hot_reload {
            let mut loader_guard = loader.write().await;
            if let Err(e) = loader_guard.enable_hot_reload().await {
                warn!("Failed to enable hot-reload: {}", e);
            } else {
                info!("Hot-reload enabled for dynamic skills");
            }
        }

        self.register_skill_tools(&ctx)?;

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
