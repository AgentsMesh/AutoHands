//! Tests for task types, queue, and chain tracker.

use super::*;
use crate::task_chain::TaskChainTracker;
use crate::task_queue::TaskQueue;
use autohands_protocols::channel::ReplyAddress;

#[test]
fn test_task_new() {
    let task = Task::new("test:task", serde_json::json!({"key": "value"}));
    assert_eq!(task.task_type, "test:task");
    assert_eq!(task.priority, TaskPriority::Normal);
    assert!(task.is_ready());
}

#[test]
fn test_task_builder() {
    let task = Task::new("test", serde_json::Value::Null)
        .with_priority(TaskPriority::High)
        .with_source(TaskSource::Agent)
        .with_correlation_id("chain-1")
        .with_max_retries(5);

    assert_eq!(task.priority, TaskPriority::High);
    assert_eq!(task.source, TaskSource::Agent);
    assert_eq!(task.correlation_id, Some("chain-1".to_string()));
    assert_eq!(task.max_retries, 5);
}

#[test]
fn test_task_delayed() {
    let future = chrono::Utc::now() + chrono::Duration::hours(1);
    let task = Task::new("test", serde_json::Value::Null).with_scheduled_at(future);

    assert!(!task.is_ready());
}

#[test]
fn test_priority_ordering() {
    assert!(TaskPriority::System > TaskPriority::Critical);
    assert!(TaskPriority::Critical > TaskPriority::High);
    assert!(TaskPriority::High > TaskPriority::Normal);
    assert!(TaskPriority::Normal > TaskPriority::Low);
}

#[tokio::test]
async fn test_task_queue_basic() {
    let config = crate::config::TaskQueueConfig::default();
    let queue = TaskQueue::new(config, 100);

    let task = Task::new("test", serde_json::Value::Null);
    queue.enqueue(task.clone()).await.unwrap();

    assert_eq!(queue.len().await, 1);

    let dequeued = queue.dequeue().await;
    assert!(dequeued.is_some());
    assert_eq!(dequeued.unwrap().task_type, "test");
    assert_eq!(queue.len().await, 0);
}

#[tokio::test]
async fn test_task_queue_priority() {
    let config = crate::config::TaskQueueConfig::default();
    let queue = TaskQueue::new(config, 100);

    let low = Task::new("low", serde_json::Value::Null).with_priority(TaskPriority::Low);
    let high =
        Task::new("high", serde_json::Value::Null).with_priority(TaskPriority::High);
    let normal = Task::new("normal", serde_json::Value::Null);

    queue.enqueue(low).await.unwrap();
    queue.enqueue(normal).await.unwrap();
    queue.enqueue(high).await.unwrap();

    assert_eq!(queue.dequeue().await.unwrap().task_type, "high");
    assert_eq!(queue.dequeue().await.unwrap().task_type, "normal");
    assert_eq!(queue.dequeue().await.unwrap().task_type, "low");
}

#[tokio::test]
async fn test_task_queue_delayed() {
    let config = crate::config::TaskQueueConfig::default();
    let queue = TaskQueue::new(config, 100);

    let future = chrono::Utc::now() + chrono::Duration::hours(1);
    let task =
        Task::new("delayed", serde_json::Value::Null).with_scheduled_at(future);

    queue.enqueue(task).await.unwrap();

    // Should be in delayed queue
    assert_eq!(queue.immediate_len().await, 0);
    assert_eq!(queue.delayed_len().await, 1);

    // Should not be dequeued
    assert!(queue.dequeue().await.is_none());
}

#[test]
fn test_chain_tracker() {
    let tracker = TaskChainTracker::new(3);

    // First 3 should succeed
    assert!(tracker.try_produce("chain-1").is_ok());
    assert!(tracker.try_produce("chain-1").is_ok());
    assert!(tracker.try_produce("chain-1").is_ok());

    // 4th should fail
    assert!(tracker.try_produce("chain-1").is_err());

    // Different chain should work
    assert!(tracker.try_produce("chain-2").is_ok());
}

#[test]
fn test_chain_tracker_reset() {
    let tracker = TaskChainTracker::new(2);

    tracker.try_produce("chain-1").unwrap();
    tracker.try_produce("chain-1").unwrap();
    assert!(tracker.try_produce("chain-1").is_err());

    tracker.reset_chain("chain-1");
    assert!(tracker.try_produce("chain-1").is_ok());
}

#[test]
fn test_task_retry() {
    let mut task = Task::new("test", serde_json::Value::Null).with_max_retries(2);

    assert!(task.can_retry());
    task.increment_retry();
    assert!(task.can_retry());
    task.increment_retry();
    assert!(!task.can_retry());
}

#[test]
fn test_ensure_correlation_id() {
    let mut task = Task::new("test", serde_json::Value::Null);
    assert!(task.correlation_id.is_none());

    let id1 = task.ensure_correlation_id();
    let id2 = task.ensure_correlation_id();
    assert_eq!(id1, id2);
}

#[test]
fn test_task_with_reply_to() {
    let reply_to = ReplyAddress::new("web", "conn-123");
    let task = Task::new("agent:execute", serde_json::json!({"prompt": "hello"}))
        .with_reply_to(reply_to.clone());

    assert!(task.reply_to.is_some());
    let task_reply_to = task.reply_to.unwrap();
    assert_eq!(task_reply_to.channel_id, "web");
    assert_eq!(task_reply_to.target, "conn-123");
}

#[test]
fn test_task_reply_to_serialization() {
    let reply_to = ReplyAddress::with_thread("telegram", "chat-456", "thread-789");
    let task = Task::new("test", serde_json::Value::Null).with_reply_to(reply_to);

    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains("telegram"));
    assert!(json.contains("chat-456"));
    assert!(json.contains("thread-789"));
}

#[test]
fn test_task_without_reply_to_serialization() {
    let task = Task::new("test", serde_json::Value::Null);

    let json = serde_json::to_string(&task).unwrap();
    // reply_to should be skipped when None
    assert!(!json.contains("reply_to"));
}

#[test]
fn test_task_reply_to_deserialization() {
    let json = r#"{
        "id": "00000000-0000-0000-0000-000000000000",
        "task_type": "test",
        "payload": null,
        "priority": "Normal",
        "source": "User",
        "created_at": "2024-01-01T00:00:00Z",
        "metadata": {},
        "retry_count": 0,
        "max_retries": 3,
        "reply_to": {
            "channel_id": "web",
            "target": "conn-123"
        }
    }"#;

    let task: Task = serde_json::from_str(json).unwrap();
    assert!(task.reply_to.is_some());
    let reply_to = task.reply_to.unwrap();
    assert_eq!(reply_to.channel_id, "web");
    assert_eq!(reply_to.target, "conn-123");
}
