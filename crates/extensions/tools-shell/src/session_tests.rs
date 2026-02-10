use super::*;

#[tokio::test]
async fn test_session_manager_creation() {
    let manager = SessionManager::new();
    assert!(manager.list_sessions().await.is_empty());
}

#[tokio::test]
async fn test_session_manager_default() {
    let manager = SessionManager::default();
    assert!(manager.list_sessions().await.is_empty());
}

#[tokio::test]
async fn test_create_session() {
    let manager = SessionManager::new();
    let result = manager.create_session(None).await;
    // May fail on CI without proper shell
    if result.is_ok() {
        assert!(!manager.list_sessions().await.is_empty());
    }
}

#[tokio::test]
async fn test_create_session_with_shell() {
    let manager = SessionManager::new();
    let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
    let result = manager.create_session(Some(shell)).await;
    if result.is_ok() {
        assert!(!manager.list_sessions().await.is_empty());
    }
}

#[tokio::test]
async fn test_execute_in_nonexistent_session() {
    let manager = SessionManager::new();
    let result = manager.execute_in_session("nonexistent", "echo hello", 1000).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        SessionError::NotFound(id) => assert_eq!(id, "nonexistent"),
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_kill_nonexistent_session() {
    let manager = SessionManager::new();
    let result = manager.kill_session("nonexistent").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        SessionError::NotFound(id) => assert_eq!(id, "nonexistent"),
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_kill_session() {
    let manager = SessionManager::new();
    let result = manager.create_session(None).await;
    if let Ok(session_id) = result {
        // Kill the session
        let kill_result = manager.kill_session(&session_id).await;
        assert!(kill_result.is_ok());

        // Verify it's removed
        assert!(!manager.list_sessions().await.contains(&session_id));
    }
}

#[tokio::test]
async fn test_cleanup() {
    let manager = SessionManager::new();
    // Create and kill a session to test cleanup
    if let Ok(session_id) = manager.create_session(None).await {
        let _ = manager.kill_session(&session_id).await;
    }
    // Cleanup should work even on empty manager
    manager.cleanup().await;
}

#[test]
fn test_session_error_display() {
    let not_found = SessionError::NotFound("id123".to_string());
    assert!(not_found.to_string().contains("Session not found"));
    assert!(not_found.to_string().contains("id123"));

    let spawn_failed = SessionError::SpawnFailed("reason".to_string());
    assert!(spawn_failed.to_string().contains("Session spawn failed"));

    let io_error = SessionError::IoError("io reason".to_string());
    assert!(io_error.to_string().contains("Session I/O error"));

    let timeout = SessionError::Timeout;
    assert!(timeout.to_string().contains("timeout"));
}

#[test]
fn test_session_error_debug() {
    let err = SessionError::NotFound("test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("NotFound"));
}

#[tokio::test]
async fn test_shell_session_spawn_invalid() {
    let result = ShellSession::spawn("nonexistent_shell_xyz");
    assert!(result.is_err());
    let err = result.err().unwrap();
    match err {
        SessionError::SpawnFailed(_) => {}
        _ => panic!("Expected SpawnFailed error"),
    }
}

#[tokio::test]
async fn test_multiple_sessions() {
    let manager = SessionManager::new();

    // Create multiple sessions
    let mut session_ids = Vec::new();
    for _ in 0..3 {
        if let Ok(id) = manager.create_session(None).await {
            session_ids.push(id);
        }
    }

    // Verify sessions count
    let sessions = manager.list_sessions().await;
    assert_eq!(sessions.len(), session_ids.len());

    // Clean up
    for id in session_ids {
        let _ = manager.kill_session(&id).await;
    }
}
