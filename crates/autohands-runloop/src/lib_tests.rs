    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic_runloop() {
        let config = RunLoopConfig::default();
        let run_loop = Arc::new(RunLoop::new(config));

        assert_eq!(run_loop.state(), RunLoopState::Created);
    }

    #[tokio::test]
    async fn test_runloop_with_timeout() {
        let config = RunLoopConfig::default();
        let run_loop = Arc::new(RunLoop::new(config));

        let result = run_loop
            .run_in_mode(RunLoopMode::Default, Duration::from_millis(50))
            .await;

        assert!(matches!(result, Ok(RunLoopRunResult::TimedOut)));
    }

    #[tokio::test]
    async fn test_runloop_inject_task() {
        let run_loop = Arc::new(RunLoop::default());

        let task = Task::new("test:task", serde_json::json!({"key": "value"}))
            .with_priority(TaskPriority::High)
            .with_source(TaskSource::User);

        run_loop.inject_task(task).await.unwrap();
        assert_eq!(run_loop.pending_task_count().await, 1);
    }

    #[tokio::test]
    async fn test_runloop_with_observer() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        struct TestObserver {
            counter: Arc<AtomicU32>,
        }

        #[async_trait::async_trait]
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
