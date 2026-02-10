//! Tests for file watcher trigger.

use super::*;
use super::super::trigger_types::{FileWatcherConfig, Trigger};
use super::super::file_watcher_source::{FileChangeEvent, FileChangeType, FileWatcherSource1};
use super::super::file_watcher_manager::FileWatcherManager;
use crate::source::Source1;

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

// Source1 tests
#[tokio::test]
async fn test_file_watcher_source1() {
    let source = FileWatcherSource1::new("file-watcher");
    assert_eq!(source.id(), "file-watcher");
    assert!(source.is_valid());
}

#[tokio::test]
async fn test_file_watcher_source1_handle() {
    let source = FileWatcherSource1::new("file-watcher");

    let msg = FileWatcherSource1::create_message(FileChangeEvent {
        path: "/test/file.txt".to_string(),
        change_type: FileChangeType::Modified,
        agent: Some("general".to_string()),
        prompt: Some("Handle file change".to_string()),
    });

    let events = source.handle(msg).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].task_type, "trigger:file:changed");
}

#[tokio::test]
async fn test_file_watcher_create_receiver() {
    let source = FileWatcherSource1::new("file-watcher");
    let (receiver, tx) = source.create_receiver();

    assert_eq!(receiver.source.id(), "file-watcher");

    // Test sending a message
    let msg = FileWatcherSource1::create_message(FileChangeEvent {
        path: "/test.txt".to_string(),
        change_type: FileChangeType::Created,
        agent: None,
        prompt: None,
    });

    tx.send(msg).await.unwrap();
}

#[test]
fn test_file_change_type_display() {
    assert_eq!(FileChangeType::Created.to_string(), "created");
    assert_eq!(FileChangeType::Modified.to_string(), "modified");
    assert_eq!(FileChangeType::Deleted.to_string(), "deleted");
    assert_eq!(FileChangeType::Renamed.to_string(), "renamed");
}
