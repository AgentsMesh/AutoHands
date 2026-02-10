//! Agent tools extension.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;

use autohands_protocols::error::ExtensionError;
use autohands_protocols::extension::{Extension, ExtensionContext, ExtensionManifest, Provides};
use autohands_protocols::types::Version;
use autohands_runtime::AgentRuntime;

use crate::manager::AgentManager;
use crate::tools::*;

/// Configuration for the agent tools extension.
#[derive(Debug, Clone)]
pub struct AgentToolsConfig {
    /// Maximum concurrent sub-agents.
    pub max_concurrent: usize,
}

impl Default for AgentToolsConfig {
    fn default() -> Self {
        Self { max_concurrent: 10 }
    }
}

/// Agent management tools extension.
pub struct AgentToolsExtension {
    manifest: ExtensionManifest,
    config: AgentToolsConfig,
    manager: Option<Arc<AgentManager>>,
}

impl AgentToolsExtension {
    /// Create a new agent tools extension.
    pub fn new() -> Self {
        let mut manifest = ExtensionManifest::new(
            "tools-agent",
            "Agent Tools",
            Version::new(0, 1, 0),
        );
        manifest.description = "Sub-agent spawning and management tools".to_string();
        manifest.provides = Provides {
            tools: vec![
                "agent_spawn".to_string(),
                "agent_status".to_string(),
                "agent_message".to_string(),
                "agent_terminate".to_string(),
                "agent_list".to_string(),
            ],
            ..Default::default()
        };

        Self {
            manifest,
            config: AgentToolsConfig::default(),
            manager: None,
        }
    }

    /// Set maximum concurrent sub-agents.
    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.config.max_concurrent = max;
        self
    }

    /// Set the agent runtime for spawning agents.
    ///
    /// This must be called before the extension is initialized if you want
    /// agents to actually execute.
    pub fn with_runtime(self, _runtime: Arc<AgentRuntime>) -> Self {
        // Store runtime temporarily - will be applied in initialize
        // For now, this is a design note: the runtime should be set via
        // ExtensionContext or after initialization
        self
    }

    /// Get the agent manager.
    pub fn manager(&self) -> Option<Arc<AgentManager>> {
        self.manager.clone()
    }

    /// Set the runtime on the manager after initialization.
    pub fn set_runtime(&self, runtime: Arc<AgentRuntime>) {
        if let Some(ref manager) = self.manager {
            manager.set_runtime(runtime);
        }
    }
}

impl Default for AgentToolsExtension {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extension for AgentToolsExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: ExtensionContext) -> Result<(), ExtensionError> {
        // Create agent manager
        let manager = Arc::new(AgentManager::new(self.config.max_concurrent));

        // Register tools
        ctx.tool_registry
            .register_tool(Arc::new(AgentSpawnTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(AgentStatusTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(AgentMessageTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(AgentTerminateTool::new(manager.clone())))?;
        ctx.tool_registry
            .register_tool(Arc::new(AgentListTool::new(manager.clone())))?;

        self.manager = Some(manager);

        tracing::info!(
            "Agent tools initialized (max_concurrent={})",
            self.config.max_concurrent
        );
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), ExtensionError> {
        if let Some(ref manager) = self.manager {
            // Terminate all running agents
            for agent in manager.list() {
                if matches!(
                    agent.status,
                    crate::manager::SpawnedAgentStatus::Starting
                        | crate::manager::SpawnedAgentStatus::Running
                        | crate::manager::SpawnedAgentStatus::Idle
                ) {
                    let _ = manager.terminate(&agent.id);
                }
            }
        }
        tracing::info!("Agent tools shutdown");
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
mod tests {
    use super::*;

    #[test]
    fn test_extension_manifest() {
        let ext = AgentToolsExtension::new();
        assert_eq!(ext.manifest().id, "tools-agent");
        assert!(ext
            .manifest()
            .provides
            .tools
            .contains(&"agent_spawn".to_string()));
    }

    #[test]
    fn test_config_default() {
        let config = AgentToolsConfig::default();
        assert_eq!(config.max_concurrent, 10);
    }

    #[test]
    fn test_max_concurrent_builder() {
        let ext = AgentToolsExtension::new().max_concurrent(5);
        assert_eq!(ext.config.max_concurrent, 5);
    }

    #[test]
    fn test_extension_default() {
        let ext = AgentToolsExtension::default();
        assert_eq!(ext.manifest().id, "tools-agent");
    }

    #[test]
    fn test_manager_initially_none() {
        let ext = AgentToolsExtension::new();
        assert!(ext.manager().is_none());
    }

    #[test]
    fn test_as_any() {
        let ext = AgentToolsExtension::new();
        let any_ref = ext.as_any();
        assert!(any_ref.downcast_ref::<AgentToolsExtension>().is_some());
    }

    #[test]
    fn test_manifest_tools_count() {
        let ext = AgentToolsExtension::new();
        assert_eq!(ext.manifest().provides.tools.len(), 5);
    }

    #[test]
    fn test_manifest_description() {
        let ext = AgentToolsExtension::new();
        assert!(ext
            .manifest()
            .description
            .contains("Sub-agent"));
    }
}
