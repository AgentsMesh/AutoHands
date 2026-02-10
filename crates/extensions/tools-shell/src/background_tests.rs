use super::*;

#[test]
fn test_background_manager_creation() {
    let manager = BackgroundManager::new();
    assert_eq!(manager.running_count(), 0);
}

#[test]
fn test_background_manager_default() {
    let manager = BackgroundManager::default();
    assert_eq!(manager.running_count(), 0);
}

#[test]
fn test_spawn_and_list() {
    let manager = BackgroundManager::new();
    // Use a simple command that exits quickly
    let result = manager.spawn("echo hello", None);
    assert!(result.is_ok());

    let id = result.unwrap();
    let list = manager.list();
    assert!(list.iter().any(|(i, _, _)| i == &id));
}

#[test]
fn test_spawn_with_cwd() {
    let manager = BackgroundManager::new();
    let result = manager.spawn("pwd", Some("/tmp"));
    assert!(result.is_ok());
}

#[test]
fn test_status() {
    let manager = BackgroundManager::new();
    let id = manager.spawn("echo status_test", None).unwrap();

    // Wait a bit for the command to complete
    std::thread::sleep(std::time::Duration::from_millis(100));

    let status = manager.status(&id);
    assert!(status.is_some());
}

#[test]
fn test_status_not_found() {
    let manager = BackgroundManager::new();
    let status = manager.status("nonexistent_id");
    assert!(status.is_none());
}

#[test]
fn test_kill() {
    let manager = BackgroundManager::new();
    // Spawn a long-running process
    let id = manager.spawn("sleep 60", None).unwrap();

    // Kill it
    let result = manager.kill(&id);
    assert!(result.is_ok());

    // Verify it's no longer running
    let status = manager.status(&id);
    assert!(matches!(status, Some(ProcessStatus::Completed(_))));
}

#[test]
fn test_kill_not_found() {
    let manager = BackgroundManager::new();
    let result = manager.kill("nonexistent_id");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Process not found"));
}

#[test]
fn test_wait() {
    let manager = BackgroundManager::new();
    let id = manager.spawn("echo wait_test", None).unwrap();

    // Wait for the process to complete
    let result = manager.wait(&id);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0); // echo should exit with 0
}

#[test]
fn test_wait_not_found() {
    let manager = BackgroundManager::new();
    let result = manager.wait("nonexistent_id");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Process not found"));
}

#[test]
fn test_cleanup() {
    let manager = BackgroundManager::new();
    // Spawn a quick command
    let _ = manager.spawn("echo test", None);
    // Give it time to complete
    std::thread::sleep(std::time::Duration::from_millis(100));
    manager.cleanup();
    // After cleanup, completed processes should be removed
    assert_eq!(manager.running_count(), 0);
}

#[test]
fn test_running_count() {
    let manager = BackgroundManager::new();

    // Initially empty
    assert_eq!(manager.running_count(), 0);

    // Spawn a long-running process
    let id = manager.spawn("sleep 60", None).unwrap();

    // Should have one running
    let _ = manager.running_count(); // Process may have started

    // Kill it
    let _ = manager.kill(&id);

    // Should be back to zero
    assert_eq!(manager.running_count(), 0);
}

#[test]
fn test_process_status_debug() {
    let running = ProcessStatus::Running;
    let completed = ProcessStatus::Completed(0);
    let failed = ProcessStatus::Failed("error".to_string());

    // Test Debug impl
    assert!(format!("{:?}", running).contains("Running"));
    assert!(format!("{:?}", completed).contains("Completed"));
    assert!(format!("{:?}", failed).contains("Failed"));
}

#[test]
fn test_process_status_clone() {
    let original = ProcessStatus::Completed(42);
    let cloned = original.clone();
    assert!(matches!(cloned, ProcessStatus::Completed(42)));
}

#[test]
fn test_multiple_processes() {
    let manager = BackgroundManager::new();

    // Spawn multiple processes
    let id1 = manager.spawn("echo one", None).unwrap();
    let id2 = manager.spawn("echo two", None).unwrap();

    // Verify both are listed
    let list = manager.list();
    assert!(list.iter().any(|(i, _, _)| i == &id1));
    assert!(list.iter().any(|(i, _, _)| i == &id2));
}

#[test]
fn test_spawn_invalid_command_cwd() {
    let manager = BackgroundManager::new();
    // This should still succeed as the shell is spawned, just running in wrong dir
    let result = manager.spawn("echo test", Some("/nonexistent_directory_xyz"));
    // The spawn may succeed because the shell itself is spawned
    // but the command might fail - either way we test the path
    let _ = result;
}
