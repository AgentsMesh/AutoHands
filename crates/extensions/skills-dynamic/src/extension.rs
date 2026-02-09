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
use autohands_protocols::tool::{Tool, ToolContext, ToolDefinition, ToolResult};
use autohands_protocols::types::Version;

use crate::loader::{DynamicSkillLoader, SkillSource};
use crate::registry::SkillRegistry;

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
    config: DynamicSkillsConfig,
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

        // Build loader with configured sources
        let mut loader = DynamicSkillLoader::new();

        // Add extra directories
        for dir in &self.config.extra_dirs {
            loader = loader.with_source(SkillSource::Directory(dir.clone()));
            debug!("Added extra skills directory: {}", dir.display());
        }

        // Add managed directory if configured
        if self.config.use_managed {
            if let Some(home) = dirs::home_dir() {
                let managed = home.join(".autohands").join("skills");
                if !managed.exists() {
                    // Create the directory
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

        // Add workspace directory if configured
        if self.config.use_workspace {
            let workspace = ctx.work_dir.join("skills");
            if workspace.exists() {
                loader = loader.with_source(SkillSource::Workspace(workspace.clone()));
                debug!("Added workspace skills directory: {}", workspace.display());
            }
        }

        // Get available tools for eligibility checking
        // Note: ToolRegistryAccess doesn't have a list method, so we skip this for now
        // The skill requirements will be checked when tools are actually needed
        loader.set_available_tools(Vec::new()).await;

        // Load all skills
        if let Err(e) = loader.load_all().await {
            warn!("Failed to load skills: {}", e);
        }

        // Register loaded skills to the registry
        {
            use autohands_protocols::skill::SkillLoader;
            let skills = loader.list().await.unwrap_or_default();
            for def in &skills {
                if let Ok(skill) = loader.load(&def.id).await {
                    self.registry.register(skill).await;
                }
            }
            info!("Registered {} dynamic skills", skills.len());

            // Update manifest with loaded skills
            self.manifest.provides.skills = skills.iter().map(|s| s.id.clone()).collect();
        }

        // Store loader
        let loader = Arc::new(RwLock::new(loader));
        self.loader = Some(loader.clone());

        // Enable hot-reload if configured
        if self.config.hot_reload {
            let mut loader_guard = loader.write().await;
            if let Err(e) = loader_guard.enable_hot_reload().await {
                warn!("Failed to enable hot-reload: {}", e);
            } else {
                info!("Hot-reload enabled for dynamic skills");
            }
        }

        // Register skill tools
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

impl DynamicSkillsExtension {
    /// Register skill-related tools.
    fn register_skill_tools(&self, ctx: &ExtensionContext) -> Result<(), ExtensionError> {
        // skill_list tool
        let registry = self.registry.clone();
        let list_tool = SkillListTool::new(registry);
        ctx.tool_registry
            .register_tool(Arc::new(list_tool))
            .map_err(|e| ExtensionError::InitializationFailed(format!("Failed to register skill_list: {}", e)))?;

        // skill_info tool
        let registry = self.registry.clone();
        let info_tool = SkillInfoTool::new(registry);
        ctx.tool_registry
            .register_tool(Arc::new(info_tool))
            .map_err(|e| ExtensionError::InitializationFailed(format!("Failed to register skill_info: {}", e)))?;

        // skill_read tool
        let registry = self.registry.clone();
        let read_tool = SkillReadTool::new(registry);
        ctx.tool_registry
            .register_tool(Arc::new(read_tool))
            .map_err(|e| ExtensionError::InitializationFailed(format!("Failed to register skill_read: {}", e)))?;

        // skill_content tool
        let registry = self.registry.clone();
        let content_tool = SkillContentTool::new(registry);
        ctx.tool_registry
            .register_tool(Arc::new(content_tool))
            .map_err(|e| ExtensionError::InitializationFailed(format!("Failed to register skill_content: {}", e)))?;

        // skill_reload tool (only if loader is available)
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

// ============================================================================
// Skill Tools
// ============================================================================

/// Tool to list all available skills.
struct SkillListTool {
    definition: ToolDefinition,
    registry: Arc<SkillRegistry>,
}

impl SkillListTool {
    fn new(registry: Arc<SkillRegistry>) -> Self {
        let definition = ToolDefinition::new(
            "skill_list",
            "skill_list",
            "List all available dynamic skills",
        )
        .with_parameters_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "tag": {
                    "type": "string",
                    "description": "Filter by tag"
                },
                "category": {
                    "type": "string",
                    "description": "Filter by category"
                }
            }
        }));

        Self { definition, registry }
    }
}

#[async_trait]
impl Tool for SkillListTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, autohands_protocols::error::ToolError> {
        let tag = params.get("tag").and_then(|v| v.as_str());
        let category = params.get("category").and_then(|v| v.as_str());

        let skills = if let Some(t) = tag {
            self.registry.find_by_tag(t).await
        } else if let Some(c) = category {
            self.registry.find_by_category(c).await
        } else {
            let defs = self.registry.list().await;
            // Convert definitions to minimal representation
            let list: Vec<serde_json::Value> = defs
                .iter()
                .map(|d| {
                    serde_json::json!({
                        "id": d.id,
                        "name": d.name,
                        "description": d.description,
                        "tags": d.tags,
                        "category": d.category,
                    })
                })
                .collect();

            return Ok(ToolResult::success(serde_json::to_string_pretty(&list).unwrap()));
        };

        let list: Vec<serde_json::Value> = skills
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.definition.id,
                    "name": s.definition.name,
                    "description": s.definition.description,
                    "tags": s.definition.tags,
                    "category": s.definition.category,
                })
            })
            .collect();

        Ok(ToolResult::success(serde_json::to_string_pretty(&list).unwrap()))
    }
}

/// Tool to get detailed info about a skill.
struct SkillInfoTool {
    definition: ToolDefinition,
    registry: Arc<SkillRegistry>,
}

impl SkillInfoTool {
    fn new(registry: Arc<SkillRegistry>) -> Self {
        let definition = ToolDefinition::new(
            "skill_info",
            "skill_info",
            "Get detailed information about a specific skill",
        )
        .with_parameters_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "skill_id": {
                    "type": "string",
                    "description": "The skill ID to get info for"
                }
            },
            "required": ["skill_id"]
        }));

        Self { definition, registry }
    }
}

#[async_trait]
impl Tool for SkillInfoTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, autohands_protocols::error::ToolError> {
        let skill_id = params
            .get("skill_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                autohands_protocols::error::ToolError::InvalidParameters(
                    "skill_id is required".to_string(),
                )
            })?;

        match self.registry.get(skill_id).await {
            Some(skill) => {
                let info = serde_json::json!({
                    "id": skill.definition.id,
                    "name": skill.definition.name,
                    "description": skill.definition.description,
                    "category": skill.definition.category,
                    "tags": skill.definition.tags,
                    "priority": skill.definition.priority,
                    "required_tools": skill.definition.required_tools,
                    "variables": skill.definition.variables,
                    "enabled": skill.definition.enabled,
                    "content_preview": skill.content.chars().take(200).collect::<String>(),
                });
                Ok(ToolResult::success(serde_json::to_string_pretty(&info).unwrap()))
            }
            None => Ok(ToolResult::error(format!("Skill not found: {}", skill_id))),
        }
    }
}

/// Tool to read files from a skill's directory.
struct SkillReadTool {
    definition: ToolDefinition,
    registry: Arc<SkillRegistry>,
}

impl SkillReadTool {
    fn new(registry: Arc<SkillRegistry>) -> Self {
        let definition = ToolDefinition::new(
            "skill_read",
            "skill_read",
            "Read a file from a skill's directory",
        )
        .with_parameters_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "skill_id": {
                    "type": "string",
                    "description": "The skill ID"
                },
                "path": {
                    "type": "string",
                    "description": "Relative path within the skill directory"
                }
            },
            "required": ["skill_id", "path"]
        }));

        Self { definition, registry }
    }
}

#[async_trait]
impl Tool for SkillReadTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, autohands_protocols::error::ToolError> {
        let skill_id = params
            .get("skill_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                autohands_protocols::error::ToolError::InvalidParameters(
                    "skill_id is required".to_string(),
                )
            })?;

        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                autohands_protocols::error::ToolError::InvalidParameters(
                    "path is required".to_string(),
                )
            })?;

        let skill = self.registry.get(skill_id).await.ok_or_else(|| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Skill not found: {}",
                skill_id
            ))
        })?;

        // Get base directory from metadata
        let base_dir = skill
            .definition
            .metadata
            .get("base_dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                autohands_protocols::error::ToolError::ExecutionFailed(
                    "Skill has no base directory".to_string(),
                )
            })?;

        let full_path = PathBuf::from(base_dir).join(path);

        // Security check: ensure path is within skill directory
        let canonical_base = std::fs::canonicalize(base_dir).map_err(|e| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Failed to resolve base path: {}",
                e
            ))
        })?;
        let canonical_path = std::fs::canonicalize(&full_path).map_err(|e| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Failed to resolve path: {}",
                e
            ))
        })?;

        if !canonical_path.starts_with(&canonical_base) {
            return Err(autohands_protocols::error::ToolError::ExecutionFailed(
                "Path traversal detected".to_string(),
            ));
        }

        // Read the file
        let content = std::fs::read_to_string(&full_path).map_err(|e| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Failed to read file: {}",
                e
            ))
        })?;

        Ok(ToolResult::success(content))
    }
}

/// Tool to get the full content/prompt of a skill.
struct SkillContentTool {
    definition: ToolDefinition,
    registry: Arc<SkillRegistry>,
}

impl SkillContentTool {
    fn new(registry: Arc<SkillRegistry>) -> Self {
        let definition = ToolDefinition::new(
            "skill_content",
            "skill_content",
            "Get the full content/prompt of a skill for use as system prompt",
        )
        .with_parameters_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "skill_id": {
                    "type": "string",
                    "description": "The skill ID to get content for"
                },
                "variables": {
                    "type": "object",
                    "description": "Variables to render in the skill template"
                }
            },
            "required": ["skill_id"]
        }));

        Self { definition, registry }
    }
}

#[async_trait]
impl Tool for SkillContentTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, autohands_protocols::error::ToolError> {
        let skill_id = params
            .get("skill_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                autohands_protocols::error::ToolError::InvalidParameters(
                    "skill_id is required".to_string(),
                )
            })?;

        let skill = self.registry.get(skill_id).await.ok_or_else(|| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Skill not found: {}",
                skill_id
            ))
        })?;

        // Extract variables if provided
        let variables: std::collections::HashMap<String, String> = params
            .get("variables")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        // Render the skill content with variables
        let content = skill.render(&variables);

        Ok(ToolResult::success(content))
    }
}

/// Tool to reload all skills (useful after manual file changes).
struct SkillReloadTool {
    definition: ToolDefinition,
    loader: Arc<RwLock<DynamicSkillLoader>>,
    registry: Arc<SkillRegistry>,
}

impl SkillReloadTool {
    fn new(loader: Arc<RwLock<DynamicSkillLoader>>, registry: Arc<SkillRegistry>) -> Self {
        let definition = ToolDefinition::new(
            "skill_reload",
            "skill_reload",
            "Reload all skills from disk",
        )
        .with_parameters_schema(serde_json::json!({
            "type": "object",
            "properties": {}
        }));

        Self { definition, loader, registry }
    }
}

#[async_trait]
impl Tool for SkillReloadTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn execute(
        &self,
        _params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, autohands_protocols::error::ToolError> {
        use autohands_protocols::skill::SkillLoader;

        // Reload from loader
        let loader = self.loader.read().await;
        loader.reload().await.map_err(|e| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Failed to reload skills: {}",
                e
            ))
        })?;

        // Update registry
        let skills = loader.list().await.map_err(|e| {
            autohands_protocols::error::ToolError::ExecutionFailed(format!(
                "Failed to list skills: {}",
                e
            ))
        })?;

        // Clear and re-register
        self.registry.clear().await;
        for def in &skills {
            if let Ok(skill) = loader.load(&def.id).await {
                self.registry.register(skill).await;
            }
        }

        Ok(ToolResult::success(format!("Reloaded {} skills", skills.len())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_manifest() {
        let ext = DynamicSkillsExtension::new();
        assert_eq!(ext.manifest().id, "skills-dynamic");
    }

    #[test]
    fn test_config_default() {
        let config = DynamicSkillsConfig::default();
        assert!(config.hot_reload);
        assert!(config.use_managed);
        assert!(config.use_workspace);
    }

    #[test]
    fn test_with_extra_dir() {
        let ext = DynamicSkillsExtension::new()
            .with_extra_dir(PathBuf::from("/custom/skills"));
        assert_eq!(ext.config.extra_dirs.len(), 1);
    }

    #[test]
    fn test_with_hot_reload() {
        let ext = DynamicSkillsExtension::new().with_hot_reload(false);
        assert!(!ext.config.hot_reload);
    }
}
