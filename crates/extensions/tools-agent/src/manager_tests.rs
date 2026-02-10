use super::*;

#[test]
fn test_spawned_agent_status_serialize() {
    assert_eq!(
        serde_json::to_string(&SpawnedAgentStatus::Running).unwrap(),
        "\"running\""
    );
    assert_eq!(
        serde_json::to_string(&SpawnedAgentStatus::Completed).unwrap(),
        "\"completed\""
    );
}

#[test]
fn test_spawned_agent_status_deserialize() {
    let status: SpawnedAgentStatus = serde_json::from_str("\"starting\"").unwrap();
    assert_eq!(status, SpawnedAgentStatus::Starting);
}

#[test]
fn test_agent_manager_creation() {
    let manager = AgentManager::new(5);
    assert!(manager.list().is_empty());
}

#[test]
fn test_agent_manager_default() {
    let manager = AgentManager::default();
    assert_eq!(manager.max_concurrent, 10);
}

#[test]
fn test_get_status_not_found() {
    let manager = AgentManager::new(5);
    assert!(manager.get_status("nonexistent").is_none());
}

#[test]
fn test_terminate_not_found() {
    let manager = AgentManager::new(5);
    let result = manager.terminate("nonexistent");
    assert!(matches!(result, Err(AgentManagerError::NotFound(_))));
}

#[test]
fn test_agent_manager_error_display() {
    let err = AgentManagerError::NotFound("agent-1".to_string());
    assert_eq!(err.to_string(), "Agent not found: agent-1");

    let err = AgentManagerError::SpawnFailed("reason".to_string());
    assert_eq!(err.to_string(), "Spawn failed: reason");
}

#[test]
fn test_spawned_agent_serialize() {
    let agent = SpawnedAgent {
        id: "spawn-1".to_string(),
        agent_id: "general".to_string(),
        session_id: "session-1".to_string(),
        parent_id: Some("parent-1".to_string()),
        status: SpawnedAgentStatus::Running,
        task: "test task".to_string(),
        spawned_at: Utc::now(),
        completed_at: None,
        last_message: None,
        error: None,
        tools: vec!["read_file".to_string()],
        metadata: HashMap::new(),
    };

    let json = serde_json::to_string(&agent).unwrap();
    assert!(json.contains("spawn-1"));
    assert!(json.contains("running"));
}
