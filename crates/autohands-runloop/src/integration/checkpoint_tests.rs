    use super::*;

    #[tokio::test]
    async fn test_memory_checkpoint_manager() {
        let manager = MemoryCheckpointManager::new(5);

        let checkpoint = RunLoopCheckpoint {
            id: Uuid::new_v4(),
            mode: RunLoopMode::Default,
            pending_events: 10,
            metrics: CheckpointMetrics {
                iterations: 100,
                events_processed: 50,
                events_enqueued: 60,
                wakeups: 25,
                uptime_secs: 300,
            },
            timestamp: Utc::now(),
        };

        manager.save_runloop_checkpoint(&checkpoint).await.unwrap();
        assert_eq!(manager.checkpoint_count(), 1);

        let loaded = manager.load_latest_checkpoint().await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, checkpoint.id);
    }

    #[tokio::test]
    async fn test_checkpoint_manager_max_capacity() {
        let manager = MemoryCheckpointManager::new(3);

        for i in 0..5 {
            let checkpoint = RunLoopCheckpoint {
                id: Uuid::new_v4(),
                mode: RunLoopMode::Default,
                pending_events: i,
                metrics: CheckpointMetrics {
                    iterations: 0,
                    events_processed: 0,
                    events_enqueued: 0,
                    wakeups: 0,
                    uptime_secs: 0,
                },
                timestamp: Utc::now(),
            };
            manager.save_runloop_checkpoint(&checkpoint).await.unwrap();
        }

        // Should only have 3 checkpoints
        assert_eq!(manager.checkpoint_count(), 3);
    }

    #[tokio::test]
    async fn test_checkpoint_manager_delete() {
        let manager = MemoryCheckpointManager::new(5);

        let checkpoint = RunLoopCheckpoint {
            id: Uuid::new_v4(),
            mode: RunLoopMode::Default,
            pending_events: 0,
            metrics: CheckpointMetrics {
                iterations: 0,
                events_processed: 0,
                events_enqueued: 0,
                wakeups: 0,
                uptime_secs: 0,
            },
            timestamp: Utc::now(),
        };

        let id = checkpoint.id;
        manager.save_runloop_checkpoint(&checkpoint).await.unwrap();

        manager.delete_checkpoint(&id).await.unwrap();
        assert_eq!(manager.checkpoint_count(), 0);

        // Delete non-existent should fail
        let result = manager.delete_checkpoint(&Uuid::new_v4()).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_checkpoint_observer_should_checkpoint() {
        let manager = Arc::new(MemoryCheckpointManager::new(5));
        let observer = CheckpointObserver::new(manager).with_interval(0);

        assert!(observer.should_checkpoint());

        observer.mark_checkpointed();
        // With 0 interval, should still allow checkpointing
        assert!(observer.should_checkpoint());
    }
