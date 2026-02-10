//! Microkernel for managing extension lifecycle.

use std::path::PathBuf;
use std::sync::Arc;

use tracing::info;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, TaskSubmitter};

use crate::lifecycle::{KernelState, LifecycleHook, LifecycleManager, ShutdownSignal};
use crate::registry::{ExtensionRegistry, MemoryRegistry, ProviderRegistry, ToolRegistry};

/// The microkernel managing extension lifecycle.
pub struct Kernel {
    /// Optional task submitter (provided when running with RunLoop).
    task_submitter: Option<Arc<dyn TaskSubmitter>>,
    extension_registry: Arc<ExtensionRegistry>,
    tool_registry: Arc<ToolRegistry>,
    provider_registry: Arc<ProviderRegistry>,
    memory_registry: Arc<MemoryRegistry>,
    lifecycle: Arc<LifecycleManager>,
    work_dir: PathBuf,
}

impl Kernel {
    /// Create a new kernel.
    pub fn new(work_dir: PathBuf) -> Self {
        Self {
            task_submitter: None,
            extension_registry: Arc::new(ExtensionRegistry::new()),
            tool_registry: Arc::new(ToolRegistry::new()),
            provider_registry: Arc::new(ProviderRegistry::new()),
            memory_registry: Arc::new(MemoryRegistry::new()),
            lifecycle: Arc::new(LifecycleManager::default()),
            work_dir,
        }
    }

    /// Create a new kernel with task submitter (for RunLoop integration).
    pub fn with_task_submitter(work_dir: PathBuf, task_submitter: Arc<dyn TaskSubmitter>) -> Self {
        Self {
            task_submitter: Some(task_submitter),
            extension_registry: Arc::new(ExtensionRegistry::new()),
            tool_registry: Arc::new(ToolRegistry::new()),
            provider_registry: Arc::new(ProviderRegistry::new()),
            memory_registry: Arc::new(MemoryRegistry::new()),
            lifecycle: Arc::new(LifecycleManager::default()),
            work_dir,
        }
    }

    /// Start the kernel.
    pub async fn start(&self) -> Result<(), ExtensionError> {
        self.lifecycle.start().await
    }

    /// Stop the kernel and all extensions.
    pub async fn stop(&self) -> Result<(), ExtensionError> {
        info!("Stopping kernel...");

        // Shutdown all extensions
        let extensions = self.extension_registry.list();
        for manifest in extensions.iter().rev() {
            if let Err(e) = self.unload_extension(&manifest.id).await {
                tracing::warn!("Failed to unload {}: {}", manifest.id, e);
            }
        }

        self.lifecycle.stop().await
    }

    /// Get kernel state.
    pub fn state(&self) -> KernelState {
        self.lifecycle.state()
    }

    /// Check if kernel is running.
    pub fn is_running(&self) -> bool {
        self.lifecycle.is_running()
    }

    /// Get shutdown signal for graceful shutdown.
    pub fn shutdown_signal(&self) -> &ShutdownSignal {
        self.lifecycle.shutdown_signal()
    }

    /// Register a lifecycle hook.
    pub async fn register_lifecycle_hook(&self, hook: Arc<dyn LifecycleHook>) {
        self.lifecycle.register_hook(hook).await;
    }

    /// Load and initialize an extension.
    pub async fn load_extension(
        &self,
        mut extension: Box<dyn Extension>,
        config: serde_json::Value,
    ) -> Result<(), ExtensionError> {
        let manifest = extension.manifest();
        let id = manifest.id.clone();

        info!("Loading extension: {} v{}", manifest.name, manifest.version);

        // Check dependencies
        self.check_dependencies(manifest)?;

        // Create context
        let ctx = ExtensionContext::new(
            config,
            self.task_submitter.clone(),
            self.tool_registry.clone(),
            self.provider_registry.clone(),
            self.memory_registry.clone(),
            self.work_dir.clone(),
        );

        // Initialize
        extension.initialize(ctx).await?;

        // Register
        self.extension_registry.register(Arc::from(extension))?;

        info!("Extension loaded: {}", id);
        Ok(())
    }

    /// Unload an extension.
    pub async fn unload_extension(&self, id: &str) -> Result<(), ExtensionError> {
        info!("Unloading extension: {}", id);
        self.extension_registry.unregister(id)?;
        Ok(())
    }

    /// Check if all dependencies are satisfied.
    fn check_dependencies(&self, manifest: &ExtensionManifest) -> Result<(), ExtensionError> {
        for dep in &manifest.dependencies.required {
            if !self.extension_registry.contains(&dep.id) {
                return Err(ExtensionError::DependencyNotSatisfied {
                    extension: manifest.id.clone(),
                    dependency: dep.id.clone(),
                });
            }
        }
        Ok(())
    }

    /// Get the task submitter if available.
    pub fn task_submitter(&self) -> Option<&Arc<dyn TaskSubmitter>> {
        self.task_submitter.as_ref()
    }

    /// Get the tool registry.
    pub fn tool_registry(&self) -> &Arc<ToolRegistry> {
        &self.tool_registry
    }

    /// Get the provider registry.
    pub fn provider_registry(&self) -> &Arc<ProviderRegistry> {
        &self.provider_registry
    }

    /// Get the memory registry.
    pub fn memory_registry(&self) -> &Arc<MemoryRegistry> {
        &self.memory_registry
    }

    /// List all loaded extensions.
    pub fn list_extensions(&self) -> Vec<ExtensionManifest> {
        self.extension_registry.list()
    }
}

#[cfg(test)]
#[path = "kernel_tests.rs"]
mod tests;
