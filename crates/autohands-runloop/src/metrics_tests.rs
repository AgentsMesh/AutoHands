    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = RunLoopMetrics::new();
        assert_eq!(metrics.iterations.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_iteration() {
        let metrics = RunLoopMetrics::new();
        metrics.record_iteration();
        metrics.record_iteration();
        assert_eq!(metrics.iterations.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_snapshot() {
        let metrics = RunLoopMetrics::new();
        metrics.record_iteration();
        metrics.record_events_processed(5);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.iterations, 1);
        assert_eq!(snapshot.events_processed, 5);
    }

    #[test]
    fn test_events_per_second() {
        let snapshot = MetricsSnapshot {
            timestamp: Utc::now(),
            uptime_secs: 10,
            iterations: 100,
            events_processed: 500,
            events_enqueued: 600,
            source0_performs: 50,
            source1_messages: 25,
            observer_notifications: 100,
            wait_time_us: 5000000,
            process_time_us: 2000000,
            wakeups: 100,
            pending_events: 10,
            active_tasks: 5,
        };

        assert_eq!(snapshot.events_per_second(), 50.0);
        assert_eq!(snapshot.avg_wait_time_ms(), 50.0);
        assert_eq!(snapshot.avg_process_time_ms(), 20.0);
    }

    #[test]
    fn test_zero_division() {
        let snapshot = MetricsSnapshot {
            timestamp: Utc::now(),
            uptime_secs: 0,
            iterations: 0,
            events_processed: 0,
            events_enqueued: 0,
            source0_performs: 0,
            source1_messages: 0,
            observer_notifications: 0,
            wait_time_us: 0,
            process_time_us: 0,
            wakeups: 0,
            pending_events: 0,
            active_tasks: 0,
        };

        assert_eq!(snapshot.events_per_second(), 0.0);
        assert_eq!(snapshot.avg_wait_time_ms(), 0.0);
        assert_eq!(snapshot.avg_process_time_ms(), 0.0);
    }
