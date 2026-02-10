use super::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

#[test]
fn test_observer_handle() {
    struct TestObserver;

    #[async_trait]
    impl RunLoopObserver for TestObserver {
        fn activities(&self) -> u32 {
            RunLoopPhase::Entry as u32
        }

        async fn on_phase(&self, _phase: RunLoopPhase, _run_loop: &RunLoop) {}
    }

    let handle = ObserverHandle::new("test", Arc::new(TestObserver));

    assert!(handle.should_trigger(RunLoopPhase::Entry));
    assert!(!handle.should_trigger(RunLoopPhase::Exit));
    assert!(!handle.should_remove());

    handle.mark_fired();
    assert!(!handle.should_remove()); // Still repeats by default
}

#[test]
fn test_non_repeating_observer() {
    struct NonRepeatingObserver;

    #[async_trait]
    impl RunLoopObserver for NonRepeatingObserver {
        fn activities(&self) -> u32 {
            RunLoopPhase::Entry as u32
        }

        fn repeats(&self) -> bool {
            false
        }

        async fn on_phase(&self, _phase: RunLoopPhase, _run_loop: &RunLoop) {}
    }

    let handle = ObserverHandle::new("test", Arc::new(NonRepeatingObserver));

    assert!(handle.should_trigger(RunLoopPhase::Entry));
    handle.mark_fired();
    assert!(!handle.should_trigger(RunLoopPhase::Entry));
    assert!(handle.should_remove());
}

#[test]
fn test_metrics_observer() {
    let observer = MetricsObserver::new();

    assert!(RunLoopPhase::BeforeWaiting.matches(observer.activities()));
    assert!(RunLoopPhase::AfterWaiting.matches(observer.activities()));
    assert!(!RunLoopPhase::Entry.matches(observer.activities()));
}

#[test]
fn test_resource_cleanup_observer() {
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();

    let observer = ResourceCleanupObserver::new(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
    });

    assert!(RunLoopPhase::BeforeWaiting.matches(observer.activities()));
    assert!(RunLoopPhase::Exit.matches(observer.activities()));
}

#[test]
fn test_logging_observer() {
    let observer = LoggingObserver::new("test");

    assert_eq!(observer.activities(), RunLoopPhase::ALL);
    assert_eq!(observer.priority(), -1000);
}

#[test]
fn test_one_shot_observer() {
    let observer = OneShotObserver::new(RunLoopPhase::Entry, |_| {});

    assert!(!observer.repeats());
    assert_eq!(observer.activities(), RunLoopPhase::Entry as u32);
}

#[test]
fn test_spawner_observer_creation() {
    use crate::spawner::SpawnerInner;

    let inner = Arc::new(SpawnerInner::new());
    let observer = SpawnerObserver::new(inner);

    assert!(RunLoopPhase::BeforeWaiting.matches(observer.activities()));
    assert!(RunLoopPhase::Exit.matches(observer.activities()));
    assert_eq!(observer.priority(), 50);
}

#[test]
fn test_spawner_observer_with_timeout() {
    use crate::spawner::SpawnerInner;

    let inner = Arc::new(SpawnerInner::new());
    let observer = SpawnerObserver::new(inner)
        .with_task_timeout(Duration::from_secs(60))
        .with_cancel_on_exit(false);

    assert!(observer.task_timeout.is_some());
    assert!(!observer.cancel_on_exit);
}
