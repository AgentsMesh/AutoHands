use super::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::mode::{RunLoopPhase, RunLoopRunResult};
use crate::task::Task;

#[tokio::test]
async fn test_runloop_new() {
    let run_loop = RunLoop::default();
    assert_eq!(run_loop.state(), RunLoopState::Created);
}

#[tokio::test]
async fn test_runloop_modes() {
    let run_loop = RunLoop::default();

    assert!(run_loop.modes.contains_key(&RunLoopMode::Default));
    assert!(run_loop.modes.contains_key(&RunLoopMode::AgentProcessing));
    assert!(run_loop.modes.contains_key(&RunLoopMode::Background));
}

#[tokio::test]
async fn test_runloop_inject_task() {
    let run_loop = RunLoop::default();
    let task = Task::new("test:task", serde_json::json!({"key": "value"}));

    run_loop.inject_task(task).await.unwrap();
    assert_eq!(run_loop.pending_task_count().await, 1);
}

#[tokio::test]
async fn test_runloop_stop() {
    let run_loop = Arc::new(RunLoop::default());
    let run_loop_clone = run_loop.clone();

    // Stop before run completes
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        run_loop_clone.stop();
    });

    let result = run_loop
        .run_in_mode(RunLoopMode::Default, Duration::from_secs(10))
        .await;
    assert!(matches!(result, Ok(RunLoopRunResult::Stopped)));
}

#[tokio::test]
async fn test_runloop_timeout() {
    let run_loop = RunLoop::default();

    let result = run_loop
        .run_in_mode(RunLoopMode::Default, Duration::from_millis(50))
        .await;
    assert!(matches!(result, Ok(RunLoopRunResult::TimedOut)));
}

#[tokio::test]
async fn test_runloop_observer() {
    use crate::observer::RunLoopObserver;
    use async_trait::async_trait;

    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();

    struct TestObserver {
        counter: Arc<AtomicU32>,
    }

    #[async_trait]
    impl RunLoopObserver for TestObserver {
        fn activities(&self) -> u32 {
            RunLoopPhase::Entry as u32 | RunLoopPhase::Exit as u32
        }

        async fn on_phase(&self, _phase: RunLoopPhase, _run_loop: &RunLoop) {
            self.counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    let run_loop = Arc::new(RunLoop::default());
    run_loop
        .add_observer("test", Arc::new(TestObserver { counter: counter_clone }))
        .await;

    let run_loop_clone = run_loop.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        run_loop_clone.stop();
    });

    run_loop
        .run_in_mode(RunLoopMode::Default, Duration::from_secs(1))
        .await
        .unwrap();

    // Should have been called for Entry and Exit
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_runloop_wakeup() {
    let run_loop = Arc::new(RunLoop::default());
    let wakeup_tx = run_loop.wakeup_sender();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        wakeup_tx
            .send(WakeupSignal::Explicit {
                reason: "test".to_string(),
            })
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        wakeup_tx.send(WakeupSignal::Stop).await.unwrap();
    });

    let result = run_loop
        .run_in_mode(RunLoopMode::Default, Duration::from_secs(10))
        .await;
    assert!(matches!(result, Ok(RunLoopRunResult::Stopped)));
}
