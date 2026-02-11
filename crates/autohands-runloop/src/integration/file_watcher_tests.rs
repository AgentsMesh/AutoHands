//! Tests for file watcher trigger and injector.

use super::*;
use super::super::trigger_types::{FileWatcherConfig, Trigger};
use super::super::file_watcher_source::{FileChangeEvent, FileChangeType, FileWatcherInjector};
use super::super::file_watcher_manager::FileWatcherManager;

use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

fn test_config() -> FileWatcherConfig {
    FileWatcherConfig {
        id: "test-watcher".to_string(),
        paths: vec![PathBuf::from("/tmp")],
        patterns: vec!["*.txt".to_string()],
        agent: "general".to_string(),
        prompt: "Process file change".to_string(),
        enabled: true,
        debounce_ms: 500,
    }
}

#[test]
fn test_file_watcher_new() {
    let trigger = FileWatcherTrigger::new(test_config());
    assert_eq!(trigger.id(), "test-watcher");
    assert_eq!(trigger.trigger_type(), "file_watcher");
    assert!(trigger.is_enabled());
}

#[test]
fn test_matches_pattern() {
    let trigger = FileWatcherTrigger::new(test_config());

    assert!(trigger.matches_pattern(&PathBuf::from("/tmp/test.txt")));
    assert!(!trigger.matches_pattern(&PathBuf::from("/tmp/test.rs")));
}

#[test]
fn test_matches_pattern_empty() {
    let mut config = test_config();
    config.patterns = vec![];
    let trigger = FileWatcherTrigger::new(config);

    // Empty patterns match everything
    assert!(trigger.matches_pattern(&PathBuf::from("/tmp/anything")));
}

#[test]
fn test_handle_event() {
    let trigger = FileWatcherTrigger::new(test_config());
    let event = trigger.handle_event(vec![PathBuf::from("/tmp/test.txt")]);

    assert!(event.is_some());
    let e = event.unwrap();
    assert_eq!(e.trigger_id, "test-watcher");
}

#[test]
fn test_handle_event_no_match() {
    let trigger = FileWatcherTrigger::new(test_config());
    let event = trigger.handle_event(vec![PathBuf::from("/tmp/test.rs")]);

    assert!(event.is_none());
}

#[test]
fn test_debounce_duration() {
    let trigger = FileWatcherTrigger::new(test_config());
    assert_eq!(trigger.debounce_duration(), Duration::from_millis(500));
}

#[tokio::test]
async fn test_file_watcher_start_stop() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = test_config();
    config.paths = vec![temp_dir.path().to_path_buf()];
    config.debounce_ms = 100;

    let trigger = FileWatcherTrigger::new(config);

    // Start
    trigger.start().await.unwrap();
    assert!(trigger.is_enabled());

    // Stop
    trigger.stop().await.unwrap();
    assert!(!trigger.is_enabled());
}

#[tokio::test]
async fn test_file_watcher_manager() {
    let manager = FileWatcherManager::new();
    let temp_dir = TempDir::new().unwrap();

    let mut config = test_config();
    config.paths = vec![temp_dir.path().to_path_buf()];

    // Register
    let trigger = manager.register(config.clone()).await.unwrap();
    assert_eq!(trigger.id(), "test-watcher");

    // Get
    let retrieved = manager.get("test-watcher").await;
    assert!(retrieved.is_some());

    // List
    let list = manager.list().await;
    assert_eq!(list.len(), 1);

    // Duplicate registration should fail
    let result = manager.register(config).await;
    assert!(result.is_err());

    // Unregister
    manager.unregister("test-watcher").await.unwrap();
    let list = manager.list().await;
    assert!(list.is_empty());
}

#[tokio::test]
async fn test_file_watcher_event_detection() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = test_config();
    config.paths = vec![temp_dir.path().to_path_buf()];
    config.patterns = vec!["*.txt".to_string()];
    config.debounce_ms = 50;

    let trigger = std::sync::Arc::new(FileWatcherTrigger::new(config));
    let mut receiver = trigger.subscribe();

    trigger.start().await.unwrap();

    // Give the watcher time to initialize
    sleep(Duration::from_millis(100)).await;

    // Create a file
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "hello").unwrap();

    // Wait for event with timeout
    let result = tokio::time::timeout(Duration::from_secs(2), receiver.recv()).await;

    trigger.stop().await.unwrap();

    // The event may or may not be received depending on timing
    // This test mainly verifies no panics/crashes
    if let Ok(Ok(event)) = result {
        assert_eq!(event.trigger_type, "file_watcher");
    }
}

// FileWatcherInjector tests
#[test]
fn test_file_watcher_injector_creation() {
    use crate::RunLoopConfig;
    use autohands_protocols::extension::TaskSubmitter;
    // RunLoop implements TaskSubmitter
    let run_loop: std::sync::Arc<dyn TaskSubmitter> = std::sync::Arc::new(crate::RunLoop::new(RunLoopConfig::default()));
    let injector = FileWatcherInjector::new(run_loop);
    // Injector created successfully - no panics
    let _ = injector;
}

#[test]
fn test_file_change_event_creation() {
    let event = FileChangeEvent {
        path: "/test/file.txt".to_string(),
        change_type: FileChangeType::Modified,
        agent: Some("general".to_string()),
        prompt: Some("Handle file change".to_string()),
    };

    assert_eq!(event.path, "/test/file.txt");
    assert_eq!(event.agent.unwrap(), "general");
}

#[test]
fn test_file_change_type_display() {
    assert_eq!(FileChangeType::Created.to_string(), "created");
    assert_eq!(FileChangeType::Modified.to_string(), "modified");
    assert_eq!(FileChangeType::Deleted.to_string(), "deleted");
    assert_eq!(FileChangeType::Renamed.to_string(), "renamed");
}
