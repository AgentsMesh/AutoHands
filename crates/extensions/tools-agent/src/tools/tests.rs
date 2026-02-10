use super::*;
use crate::manager::SpawnedAgentStatus;

#[test]
fn test_spawn_params_deserialize() {
    let json = r#"{"agent_id": "general", "task": "research AI"}"#;
    let params: AgentSpawnParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.agent_id, "general");
    assert_eq!(params.task, "research AI");
    assert!(params.tools.is_empty());
}

#[test]
fn test_spawn_params_with_tools() {
    let json = r#"{"agent_id": "general", "task": "test", "tools": ["read_file", "write_file"]}"#;
    let params: AgentSpawnParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.tools.len(), 2);
}

#[test]
fn test_status_params_deserialize() {
    let json = r#"{"spawn_id": "spawn-123"}"#;
    let params: AgentStatusParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.spawn_id, "spawn-123");
    assert!(params.include_result); // default
}

#[test]
fn test_message_params_deserialize() {
    let json = r#"{"spawn_id": "spawn-123", "message": "update progress"}"#;
    let params: AgentMessageParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.spawn_id, "spawn-123");
    assert_eq!(params.message, "update progress");
}

#[test]
fn test_terminate_params_deserialize() {
    let json = r#"{"spawn_id": "spawn-123", "reason": "no longer needed"}"#;
    let params: AgentTerminateParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.spawn_id, "spawn-123");
    assert_eq!(params.reason, Some("no longer needed".to_string()));
}

#[test]
fn test_list_params_deserialize() {
    let json = r#"{"status": "running"}"#;
    let params: AgentListParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.status, Some(SpawnedAgentStatus::Running));
}

#[test]
fn test_spawn_result_serialize() {
    let result = AgentSpawnResult {
        spawn_id: "spawn-123".to_string(),
        agent_id: "general".to_string(),
        session_id: "session-456".to_string(),
        status: SpawnedAgentStatus::Starting,
        message: "spawned".to_string(),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("spawn-123"));
    assert!(json.contains("starting"));
}

#[test]
fn test_list_result_serialize() {
    let result = AgentListResult {
        count: 0,
        agents: vec![],
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"count\":0"));
}
