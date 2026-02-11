use super::*;
use std::collections::HashMap;

use async_trait::async_trait;

use autohands_core::registry::{ProviderRegistry, ToolRegistry};
use autohands_protocols::agent::{Agent, AgentConfig, AgentContext, AgentResponse};
use autohands_protocols::error::AgentError;
use autohands_protocols::tool::AbortSignal;
use autohands_protocols::types::Message;

use crate::agent_loop::AgentLoopConfig;

struct MockAgent {
    config: AgentConfig,
}

impl MockAgent {
    fn new(id: &str) -> Self {
        Self {
            config: AgentConfig::new(id, "Mock Agent", "mock-model"),
        }
    }
}

#[async_trait]
impl Agent for MockAgent {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn process(
        &self,
        message: Message,
        _ctx: AgentContext,
    ) -> Result<AgentResponse, AgentError> {
        Ok(AgentResponse {
            message: Message::assistant(&format!("Echo: {}", message.content.text())),
            is_complete: true,
            tool_calls: Vec::new(),
            metadata: HashMap::new(),
            usage: None,
        })
    }
}

#[test]
fn test_runtime_config_default() {
    let config = AgentRuntimeConfig::default();
    assert_eq!(config.max_concurrent, 10);
}

#[test]
fn test_runtime_creation() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let config = AgentRuntimeConfig::default();

    let runtime = AgentRuntime::new(provider_registry, tool_registry, config);
    assert!(runtime.list_agents().is_empty());
}

#[test]
fn test_register_agent() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

    let agent = Arc::new(MockAgent::new("test-agent"));
    runtime.register_agent(agent);

    assert_eq!(runtime.list_agents().len(), 1);
    assert!(runtime.get_agent("test-agent").is_some());
}

#[test]
fn test_unregister_agent() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

    let agent = Arc::new(MockAgent::new("test-agent"));
    runtime.register_agent(agent);
    runtime.unregister_agent("test-agent");

    assert!(runtime.list_agents().is_empty());
}

#[tokio::test]
async fn test_execute_agent() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

    let agent = Arc::new(MockAgent::new("test-agent"));
    runtime.register_agent(agent);

    let message = Message::user("Hello");
    let result = runtime.execute("test-agent", "session-1", message).await;

    assert!(result.is_ok());
    let messages = result.unwrap();
    assert!(!messages.is_empty());
}

#[tokio::test]
async fn test_execute_nonexistent_agent() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

    let message = Message::user("Hello");
    let result = runtime.execute("nonexistent", "session-1", message).await;

    assert!(result.is_err());
}

#[test]
fn test_running_count() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

    assert_eq!(runtime.running_count(), 0);
    assert!(!runtime.is_running("session-1"));
}

#[test]
fn test_abort_nonexistent() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

    assert!(!runtime.abort("nonexistent"));
}

#[test]
fn test_runtime_config_debug() {
    let config = AgentRuntimeConfig::default();
    let debug = format!("{:?}", config);
    assert!(debug.contains("AgentRuntimeConfig"));
}

#[test]
fn test_runtime_config_clone() {
    let config = AgentRuntimeConfig::default();
    let cloned = config.clone();
    assert_eq!(cloned.max_concurrent, config.max_concurrent);
}

#[test]
fn test_runtime_config_custom() {
    let config = AgentRuntimeConfig {
        max_concurrent: 5,
        default_loop_config: AgentLoopConfig {
            checkpoint_enabled: false,
            max_tool_output_chars: 50_000,
        },
    };
    assert_eq!(config.max_concurrent, 5);
    assert!(!config.default_loop_config.checkpoint_enabled);
}

#[test]
fn test_get_nonexistent_agent() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

    assert!(runtime.get_agent("nonexistent").is_none());
}

#[test]
fn test_session_manager_access() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

    let sm = runtime.session_manager();
    // Should be able to access session manager and it starts empty
    assert!(sm.get("nonexistent").is_none());
}

#[test]
fn test_list_agents_multiple() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

    runtime.register_agent(Arc::new(MockAgent::new("agent1")));
    runtime.register_agent(Arc::new(MockAgent::new("agent2")));
    runtime.register_agent(Arc::new(MockAgent::new("agent3")));

    assert_eq!(runtime.list_agents().len(), 3);
}

#[test]
fn test_agent_handle_fields() {
    let abort_signal = Arc::new(AbortSignal::new());
    let handle = AgentHandle {
        session_id: "test-session".to_string(),
        abort_signal: abort_signal.clone(),
    };
    assert_eq!(handle.session_id, "test-session");
    assert!(!handle.abort_signal.is_aborted());
}

#[test]
fn test_history_manager_access() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

    let hm = runtime.history_manager();
    // Should be able to access history manager and it starts empty
    assert!(hm.get("nonexistent").is_empty());
}

#[test]
fn test_clear_history() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

    // Manually add some history
    runtime.history_manager().push("session-1", Message::user("Hello"));
    runtime.history_manager().push("session-1", Message::assistant("Hi"));
    assert_eq!(runtime.history_manager().get("session-1").len(), 2);

    // Clear and verify
    runtime.clear_history("session-1");
    assert!(runtime.history_manager().get("session-1").is_empty());
}

#[tokio::test]
async fn test_execute_records_history() {
    let provider_registry = Arc::new(ProviderRegistry::new());
    let tool_registry = Arc::new(ToolRegistry::new());
    let runtime = AgentRuntime::new(provider_registry, tool_registry, Default::default());

    let agent = Arc::new(MockAgent::new("test-agent"));
    runtime.register_agent(agent);

    // First message
    let message = Message::user("Hello");
    let result = runtime.execute("test-agent", "session-1", message).await;
    assert!(result.is_ok());

    // History should contain the user message and agent response
    let history = runtime.history_manager().get("session-1");
    assert!(history.len() >= 2); // At least user message + agent response
}
