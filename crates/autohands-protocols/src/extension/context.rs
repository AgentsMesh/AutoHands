//! Extension context for initialization.

use std::sync::Arc;

use super::{MemoryRegistryAccess, ProviderRegistryAccess, TaskSubmitter, ToolRegistryAccess};

/// Context passed to extensions during initialization.
#[derive(Clone)]
pub struct ExtensionContext {
    /// Configuration for this extension.
    pub config: serde_json::Value,

    /// Task submitter for publishing tasks to RunLoop.
    pub task_submitter: Option<Arc<dyn TaskSubmitter>>,

    /// Registry for registering tools.
    pub tool_registry: Arc<dyn ToolRegistryAccess>,

    /// Registry for registering providers.
    pub provider_registry: Arc<dyn ProviderRegistryAccess>,

    /// Registry for registering memory backends.
    pub memory_registry: Arc<dyn MemoryRegistryAccess>,

    /// Working directory.
    pub work_dir: std::path::PathBuf,
}

impl ExtensionContext {
    /// Create a new extension context.
    pub fn new(
        config: serde_json::Value,
        task_submitter: Option<Arc<dyn TaskSubmitter>>,
        tool_registry: Arc<dyn ToolRegistryAccess>,
        provider_registry: Arc<dyn ProviderRegistryAccess>,
        memory_registry: Arc<dyn MemoryRegistryAccess>,
        work_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            config,
            task_submitter,
            tool_registry,
            provider_registry,
            memory_registry,
            work_dir,
        }
    }

    /// Get a configuration value.
    pub fn get_config<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.config
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}
