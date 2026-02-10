use super::*;
use crate::correlation::CorrelationGuard;
use crate::spawner_types::{SpawnerInner, TaskInfo, TaskState};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn test_spawner_creation() {
    let spawner = RunLoopSpawner::new();

    let metrics = spawner.metrics();
    assert_eq!(metrics.total_spawned, 0);
    assert_eq!(metrics.active_tasks, 0);
}

#[tokio::test]
async fn test_spawn_task() {
    let spawner = RunLoopSpawner::new();

    let handle = spawner
        .spawn("test-task", async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            42
        })
        .await;

    let result = handle.await.unwrap();
    assert_eq!(result, 42);

    let metrics = spawner.metrics();
    assert_eq!(metrics.total_spawned, 1);
    assert_eq!(metrics.total_completed, 1);
}

#[tokio::test]
async fn test_correlation_context() {
    let spawner = RunLoopSpawner::new();

    assert!(spawner.correlation_context().await.is_none());

    spawner
        .set_correlation_context(Some("test-correlation".to_string()))
        .await;
    assert_eq!(
        spawner.correlation_context().await,
        Some("test-correlation".to_string())
    );

    spawner.set_correlation_context(None).await;
    assert!(spawner.correlation_context().await.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_correlation_guard_restores_context() {
    let spawner = RunLoopSpawner::new();

    // Set initial context
    spawner
        .set_correlation_context(Some("original".to_string()))
        .await;

    {
        // Create guard with new context
        let _guard = CorrelationGuard::new(&spawner, "scoped".to_string()).await;

        // Inside scope, context should be "scoped"
        assert_eq!(
            spawner.correlation_context().await,
            Some("scoped".to_string())
        );
    }
    // Guard is dropped here

    // Small delay to allow async restore if needed
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Context should be restored to "original"
    assert_eq!(
        spawner.correlation_context().await,
        Some("original".to_string())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_correlation_guard_manual_restore() {
    let spawner = RunLoopSpawner::new();

    spawner
        .set_correlation_context(Some("original".to_string()))
        .await;

    let guard = CorrelationGuard::new(&spawner, "scoped".to_string()).await;

    assert_eq!(
        spawner.correlation_context().await,
        Some("scoped".to_string())
    );

    // Manual restore
    guard.restore().await;

    // Context should be immediately restored
    assert_eq!(
        spawner.correlation_context().await,
        Some("original".to_string())
    );
}

#[tokio::test]
async fn test_spawn_with_correlation() {
    let spawner = RunLoopSpawner::new();

    spawner
        .set_correlation_context(Some("parent-correlation".to_string()))
        .await;

    let handle = spawner
        .spawn("correlated-task", async { "done" })
        .await;

    handle.await.unwrap();

    // Verify the task was created with correlation
    let metrics = spawner.metrics();
    assert_eq!(metrics.total_completed, 1);
}

#[tokio::test]
async fn test_spawn_blocking() {
    let spawner = RunLoopSpawner::new();

    let handle = spawner
        .spawn_blocking("blocking-task", || {
            std::thread::sleep(Duration::from_millis(10));
            123
        })
        .await;

    let result = handle.await.unwrap();
    assert_eq!(result, 123);
}

#[tokio::test]
async fn test_task_abort() {
    let spawner = RunLoopSpawner::new();

    let handle = spawner
        .spawn("long-task", async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            "never"
        })
        .await;

    handle.abort();

    // Task should be marked as cancelled eventually
    tokio::time::sleep(Duration::from_millis(50)).await;

    let metrics = spawner.metrics();
    assert_eq!(metrics.total_cancelled, 1);
}

#[tokio::test]
async fn test_active_tasks() {
    let spawner = RunLoopSpawner::new();

    let handle1 = spawner
        .spawn("task-1", async {
            tokio::time::sleep(Duration::from_millis(100)).await;
        })
        .await;

    let handle2 = spawner
        .spawn("task-2", async {
            tokio::time::sleep(Duration::from_millis(100)).await;
        })
        .await;

    // Give tasks time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    let active = spawner.active_tasks();
    assert_eq!(active.len(), 2);

    // Wait for completion
    handle1.await.unwrap();
    handle2.await.unwrap();

    let active = spawner.active_tasks();
    assert_eq!(active.len(), 0);
}

#[tokio::test]
async fn test_cancel_all() {
    let spawner = RunLoopSpawner::new();

    let _handle1 = spawner
        .spawn("task-1", async {
            tokio::time::sleep(Duration::from_secs(10)).await;
        })
        .await;

    let _handle2 = spawner
        .spawn("task-2", async {
            tokio::time::sleep(Duration::from_secs(10)).await;
        })
        .await;

    spawner.cancel_all();

    let metrics = spawner.metrics();
    assert_eq!(metrics.total_cancelled, 2);
}

#[tokio::test]
async fn test_task_info() {
    let spawner = RunLoopSpawner::new();

    spawner
        .set_correlation_context(Some("test-corr".to_string()))
        .await;

    let handle = spawner
        .spawn("info-task", async {
            tokio::time::sleep(Duration::from_millis(50)).await;
        })
        .await;

    let active = spawner.active_tasks();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].name, "info-task");
    assert_eq!(active[0].correlation_id, Some("test-corr".to_string()));
    assert_eq!(active[0].state, TaskState::Running);
    assert!(!active[0].cancellable); // Regular spawn is not cancellable

    handle.await.unwrap();
}

#[tokio::test]
async fn test_spawn_cancellable() {
    let spawner = RunLoopSpawner::new();

    let handle = spawner
        .spawn_cancellable("cancellable-task", |token| async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        return "cancelled";
                    }
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {
                        // Keep working
                    }
                }
            }
        })
        .await;

    // Verify task is marked as cancellable
    let active = spawner.active_tasks();
    assert_eq!(active.len(), 1);
    assert!(active[0].cancellable);
    assert_eq!(spawner.inner().cancellable_count(), 1);

    // Cancel the task
    let cancelled = spawner.cancel_task(handle.id);
    assert!(cancelled);

    // Wait for task to finish
    let result = handle.await.unwrap();
    assert_eq!(result, "cancelled");

    // Verify metrics
    let metrics = spawner.metrics();
    assert_eq!(metrics.total_cancelled, 1);
}

#[tokio::test]
async fn test_cancel_all_with_cancellable_tasks() {
    let spawner = RunLoopSpawner::new();

    // Spawn a mix of cancellable and non-cancellable tasks
    let _handle1 = spawner
        .spawn("regular-task", async {
            tokio::time::sleep(Duration::from_secs(10)).await;
        })
        .await;

    let _handle2 = spawner
        .spawn_cancellable("cancellable-task", |token| async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        return "cancelled";
                    }
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {}
                }
            }
        })
        .await;

    // Give tasks time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Verify we have one cancellable task
    assert_eq!(spawner.inner().cancellable_count(), 1);

    // Cancel all tasks
    let cancelled_with_tokens = spawner.cancel_all();
    assert_eq!(cancelled_with_tokens, 1); // Only one had a token

    // All tasks should be marked as cancelled
    let metrics = spawner.metrics();
    assert_eq!(metrics.total_cancelled, 2);
}

#[tokio::test]
async fn test_cancel_task_by_id() {
    let spawner = RunLoopSpawner::new();

    let handle = spawner
        .spawn_cancellable("task-to-cancel", |token| async move {
            token.cancelled().await;
            "done"
        })
        .await;

    let task_id = handle.id;

    // Cancel by ID
    assert!(spawner.cancel_task(task_id));

    // Try to cancel again (should fail, already removed)
    assert!(!spawner.cancel_task(task_id));

    // Wait for completion
    let _ = handle.await;
}

#[tokio::test]
async fn test_spawner_inner_cancel_all() {
    let inner = Arc::new(SpawnerInner::new());

    // Register some tasks with tokens
    let token1 = CancellationToken::new();
    let token2 = CancellationToken::new();

    let info1 = TaskInfo {
        id: uuid::Uuid::new_v4(),
        name: "task1".to_string(),
        correlation_id: None,
        parent_correlation_id: None,
        state: TaskState::Running,
        spawned_at: chrono::Utc::now(),
        cancellable: true,
    };

    let info2 = TaskInfo {
        id: uuid::Uuid::new_v4(),
        name: "task2".to_string(),
        correlation_id: None,
        parent_correlation_id: None,
        state: TaskState::Running,
        spawned_at: chrono::Utc::now(),
        cancellable: true,
    };

    inner.register_task(info1, Some(token1.clone()));
    inner.register_task(info2, Some(token2.clone()));

    assert_eq!(inner.cancellable_count(), 2);

    // Cancel all
    let cancelled = inner.cancel_all();
    assert_eq!(cancelled, 2);

    // Tokens should be triggered
    assert!(token1.is_cancelled());
    assert!(token2.is_cancelled());

    // Tasks should be removed
    assert_eq!(inner.tasks.len(), 0);
    assert_eq!(inner.cancellable_count(), 0);
}
