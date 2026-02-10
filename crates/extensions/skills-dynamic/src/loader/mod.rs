//! Skill loading module.
//!
//! Provides filesystem-based skill loading with multi-level priority and hot-reload support.
//!
//! ## Supported Formats
//!
//! The loader supports multiple skill formats through adapters:
//!
//! - **AutoHands** (native): Full feature support with `id`, `requires`, `variables`
//! - **Claude Code**: Simple `name` + `description` format
//! - **OpenClaw**: With `metadata.openclaw` and `_meta.json`
//! - **Microsoft Skills**: SDK-focused with language suffixes

pub mod adapter;
mod filesystem;
mod parser;
mod watcher;

pub use filesystem::FilesystemLoader;
pub use parser::parse_skill_markdown;
pub use watcher::SkillWatcher;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use autohands_protocols::error::SkillError;
use autohands_protocols::skill::{Skill, SkillDefinition, SkillLoader};

/// Source type for skill loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    /// Built-in skills (lowest priority).
    Bundled,
    /// Custom directory with explicit path.
    Directory(PathBuf),
    /// Managed skills directory (~/.autohands/skills/).
    Managed(PathBuf),
    /// Workspace skills (<cwd>/skills/).
    Workspace(PathBuf),
    /// Plugin-provided skills (highest priority).
    Plugin { plugin_id: String, path: PathBuf },
}

impl SkillSource {
    /// Get the priority of this source (higher = more preferred).
    pub fn priority(&self) -> i32 {
        match self {
            SkillSource::Bundled => 0,
            SkillSource::Directory(_) => 10,
            SkillSource::Managed(_) => 20,
            SkillSource::Workspace(_) => 30,
            SkillSource::Plugin { .. } => 40,
        }
    }

    /// Get the path for this source, if applicable.
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            SkillSource::Bundled => None,
            SkillSource::Directory(p) => Some(p),
            SkillSource::Managed(p) => Some(p),
            SkillSource::Workspace(p) => Some(p),
            SkillSource::Plugin { path, .. } => Some(path),
        }
    }
}

/// Dynamic skill loader with multi-level sources and hot-reload.
pub struct DynamicSkillLoader {
    /// Skill sources ordered by priority (low to high).
    sources: Vec<SkillSource>,
    /// Loaded skills indexed by ID.
    skills: Arc<RwLock<HashMap<String, Skill>>>,
    /// File system loader.
    fs_loader: FilesystemLoader,
    /// Optional file watcher for hot-reload.
    watcher: Option<Arc<RwLock<SkillWatcher>>>,
    /// Tool registry access for dependency checking.
    available_tools: Arc<RwLock<Vec<String>>>,
}

impl DynamicSkillLoader {
    /// Create a new dynamic skill loader with default sources.
    pub fn new() -> Self {
        let mut sources = Vec::new();

        // Add managed directory (~/.autohands/skills/) if it exists
        if let Some(home) = dirs::home_dir() {
            let managed = home.join(".autohands").join("skills");
            if managed.exists() {
                sources.push(SkillSource::Managed(managed));
            }
        }

        Self {
            sources,
            skills: Arc::new(RwLock::new(HashMap::new())),
            fs_loader: FilesystemLoader::new(),
            watcher: None,
            available_tools: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a skill source.
    pub fn with_source(mut self, source: SkillSource) -> Self {
        self.sources.push(source);
        // Sort by priority (low to high, so higher priority sources override)
        self.sources.sort_by_key(|s| s.priority());
        self
    }

    /// Set the workspace directory for skill loading.
    pub fn with_workspace(mut self, workspace: PathBuf) -> Self {
        let skills_dir = workspace.join("skills");
        self.sources.push(SkillSource::Workspace(skills_dir));
        self.sources.sort_by_key(|s| s.priority());
        self
    }

    /// Set available tools for dependency checking.
    pub async fn set_available_tools(&self, tools: Vec<String>) {
        let mut available = self.available_tools.write().await;
        *available = tools;
    }

    /// Enable hot-reload with file watching.
    pub async fn enable_hot_reload(&mut self) -> Result<(), SkillError> {
        let skills = self.skills.clone();
        let fs_loader = self.fs_loader.clone();
        let available_tools = self.available_tools.clone();

        let mut watcher = SkillWatcher::new(skills, fs_loader, available_tools);

        // Watch all filesystem-based sources
        for source in &self.sources {
            if let Some(path) = source.path() {
                if path.exists() {
                    watcher.watch(path.clone())?;
                    info!("Watching for skill changes: {}", path.display());
                }
            }
        }

        // Start the watcher
        watcher.start()?;

        self.watcher = Some(Arc::new(RwLock::new(watcher)));
        Ok(())
    }

    /// Disable hot-reload.
    pub async fn disable_hot_reload(&mut self) {
        if let Some(watcher) = self.watcher.take() {
            let mut watcher_guard = watcher.write().await;
            watcher_guard.stop();
            info!("Hot-reload disabled");
        }
    }

    /// Load all skills from configured sources.
    pub async fn load_all(&self) -> Result<(), SkillError> {
        let mut all_skills = HashMap::new();

        // Load from each source (lower priority first, so higher priority overwrites)
        for source in &self.sources {
            match source {
                SkillSource::Bundled => {
                    // Bundled skills are handled by skills-bundled extension
                    debug!("Skipping bundled skills (handled separately)");
                }
                SkillSource::Directory(path)
                | SkillSource::Managed(path)
                | SkillSource::Workspace(path)
                | SkillSource::Plugin { path, .. } => {
                    if path.exists() {
                        let skills = self.fs_loader.load_from_directory(path).await?;
                        for skill in skills {
                            if self.check_eligibility(&skill).await {
                                debug!("Loaded skill: {} from {:?}", skill.definition.id, source);
                                all_skills.insert(skill.definition.id.clone(), skill);
                            } else {
                                warn!(
                                    "Skill {} not eligible (missing dependencies)",
                                    skill.definition.id
                                );
                            }
                        }
                    } else {
                        debug!("Skill source path does not exist: {}", path.display());
                    }
                }
            }
        }

        let mut skills = self.skills.write().await;
        *skills = all_skills;

        info!("Loaded {} dynamic skills", skills.len());
        Ok(())
    }

    /// Check if a skill's dependencies are satisfied.
    async fn check_eligibility(&self, skill: &Skill) -> bool {
        let metadata = &skill.definition.metadata;

        // Check required binaries (any_bins)
        if let Some(any_bins) = metadata.get("any_bins") {
            if let Some(bins) = any_bins.as_array() {
                let bins: Vec<&str> = bins.iter().filter_map(|v| v.as_str()).collect();
                if !bins.is_empty() && !bins.iter().any(|b| which::which(b).is_ok()) {
                    debug!(
                        "Skill {} missing any of required binaries: {:?}",
                        skill.definition.id, bins
                    );
                    return false;
                }
            }
        }

        // Check required binaries (all_bins)
        if let Some(all_bins) = metadata.get("all_bins") {
            if let Some(bins) = all_bins.as_array() {
                let bins: Vec<&str> = bins.iter().filter_map(|v| v.as_str()).collect();
                if !bins.iter().all(|b| which::which(b).is_ok()) {
                    debug!(
                        "Skill {} missing all required binaries: {:?}",
                        skill.definition.id, bins
                    );
                    return false;
                }
            }
        }

        // Check required tools (only if we have a known list of available tools)
        if !skill.definition.required_tools.is_empty() {
            let available = self.available_tools.read().await;
            // Skip check if no tools are registered (CLI mode)
            if !available.is_empty() {
                for tool in &skill.definition.required_tools {
                    if !available.contains(tool) {
                        debug!(
                            "Skill {} missing required tool: {}",
                            skill.definition.id, tool
                        );
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Get the list of skill sources.
    pub fn sources(&self) -> &[SkillSource] {
        &self.sources
    }
}

impl Default for DynamicSkillLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillLoader for DynamicSkillLoader {
    async fn load(&self, skill_id: &str) -> Result<Skill, SkillError> {
        let skills = self.skills.read().await;
        skills
            .get(skill_id)
            .cloned()
            .ok_or_else(|| SkillError::NotFound(skill_id.to_string()))
    }

    async fn list(&self) -> Result<Vec<SkillDefinition>, SkillError> {
        let skills = self.skills.read().await;
        Ok(skills.values().map(|s| s.definition.clone()).collect())
    }

    async fn reload(&self) -> Result<(), SkillError> {
        self.load_all().await
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
