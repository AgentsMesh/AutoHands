use super::*;

#[test]
fn test_agent_config_new() {
    let config = AgentConfig::new("test-agent", "Test Agent", "gpt-4");
    assert_eq!(config.id, "test-agent");
    assert_eq!(config.name, "Test Agent");
    assert_eq!(config.default_model, "gpt-4");
    assert!(config.description.is_empty());
    assert!(config.system_prompt.is_none());
    assert_eq!(config.max_turns, 50);
    assert_eq!(config.timeout_seconds, 300);
}

#[test]
fn test_agent_config_with_system_prompt() {
    let config = AgentConfig::new("test", "Test", "gpt-4")
        .with_system_prompt("You are a helpful assistant.");
    assert_eq!(config.system_prompt, Some("You are a helpful assistant.".to_string()));
}

#[test]
fn test_agent_config_with_tools() {
    let config = AgentConfig::new("test", "Test", "gpt-4")
        .with_tools(vec!["tool1".to_string(), "tool2".to_string()]);
    assert_eq!(config.tools.len(), 2);
    assert!(config.tools.contains(&"tool1".to_string()));
}

#[test]
fn test_agent_config_serialization() {
    let config = AgentConfig::new("test", "Test", "gpt-4");
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("test"));
    assert!(json.contains("gpt-4"));
}

#[test]
fn test_agent_config_deserialization() {
    let json = r#"{"id":"test","name":"Test","description":"","default_model":"gpt-4","max_turns":50,"timeout_seconds":300,"tools":[],"skills":[],"metadata":{}}"#;
    let config: AgentConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.id, "test");
    assert_eq!(config.default_model, "gpt-4");
}

#[test]
fn test_agent_config_clone() {
    let config = AgentConfig::new("test", "Test", "gpt-4")
        .with_system_prompt("System prompt")
        .with_tools(vec!["tool1".to_string()]);
    let cloned = config.clone();
    assert_eq!(cloned.id, config.id);
    assert_eq!(cloned.system_prompt, config.system_prompt);
    assert_eq!(cloned.tools, config.tools);
}

#[test]
fn test_agent_config_debug() {
    let config = AgentConfig::new("test", "Test", "gpt-4");
    let debug = format!("{:?}", config);
    assert!(debug.contains("AgentConfig"));
    assert!(debug.contains("test"));
}

#[test]
fn test_agent_context_new() {
    let ctx = AgentContext::new("session-123");
    assert_eq!(ctx.session_id, "session-123");
    assert!(ctx.history.is_empty());
    assert!(ctx.data.is_empty());
}

#[test]
fn test_agent_context_with_history() {
    let history = vec![Message::user("Hello"), Message::assistant("Hi there!")];
    let ctx = AgentContext::new("session-123").with_history(history.clone());
    assert_eq!(ctx.history.len(), 2);
}

#[test]
fn test_agent_context_clone() {
    let ctx = AgentContext::new("session-123")
        .with_history(vec![Message::user("Hello")]);
    let cloned = ctx.clone();
    assert_eq!(cloned.session_id, ctx.session_id);
    assert_eq!(cloned.history.len(), ctx.history.len());
}

#[test]
fn test_agent_context_abort_signal() {
    let ctx = AgentContext::new("session-123");
    assert!(!ctx.abort_signal.is_aborted());
    ctx.abort_signal.abort();
    assert!(ctx.abort_signal.is_aborted());
}

#[test]
fn test_agent_response_serialization() {
    let response = AgentResponse {
        message: Message::assistant("Hello!"),
        is_complete: true,
        tool_calls: Vec::new(),
        metadata: HashMap::new(),
        usage: None,
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Hello!"));
    assert!(json.contains("is_complete"));
}

#[test]
fn test_agent_response_deserialization() {
    let json = r#"{"message":{"role":"assistant","content":[{"type":"text","text":"Hello"}]},"is_complete":true,"tool_calls":[],"metadata":{}}"#;
    let response: AgentResponse = serde_json::from_str(json).unwrap();
    assert!(response.is_complete);
}

#[test]
fn test_agent_response_clone() {
    let response = AgentResponse {
        message: Message::assistant("Hello!"),
        is_complete: false,
        tool_calls: Vec::new(),
        metadata: HashMap::new(),
        usage: None,
    };
    let cloned = response.clone();
    assert_eq!(cloned.is_complete, response.is_complete);
}

#[test]
fn test_agent_response_debug() {
    let response = AgentResponse {
        message: Message::assistant("Hello!"),
        is_complete: true,
        tool_calls: Vec::new(),
        metadata: HashMap::new(),
        usage: None,
    };
    let debug = format!("{:?}", response);
    assert!(debug.contains("AgentResponse"));
}

#[test]
fn test_default_max_turns() {
    assert_eq!(default_max_turns(), 50);
}

#[test]
fn test_default_timeout() {
    assert_eq!(default_timeout(), 300);
}
