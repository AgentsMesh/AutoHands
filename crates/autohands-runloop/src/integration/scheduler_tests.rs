    use super::*;
    use crate::RunLoopConfig;

    #[test]
    fn test_scheduler_injector_new() {
        let scheduler = Arc::new(MockScheduler::new());
        // RunLoop implements TaskSubmitter
        let task_submitter: Arc<dyn TaskSubmitter> = Arc::new(crate::RunLoop::new(RunLoopConfig::default()));
        let injector = SchedulerInjector::new(scheduler, task_submitter);

        assert!(!injector.is_running());
    }

    #[test]
    fn test_scheduler_injector_stop() {
        let scheduler = Arc::new(MockScheduler::new());
        let task_submitter: Arc<dyn TaskSubmitter> = Arc::new(crate::RunLoop::new(RunLoopConfig::default()));
        let injector = SchedulerInjector::new(scheduler, task_submitter);

        // Initially not running
        assert!(!injector.is_running());

        // After stop, still not running
        injector.stop();
        assert!(!injector.is_running());
    }

    #[test]
    fn test_job_info() {
        let info = JobInfo {
            job_id: "job-1".to_string(),
            agent: "general".to_string(),
            prompt: "test task".to_string(),
        };

        assert_eq!(info.job_id, "job-1");
        assert_eq!(info.agent, "general");
        assert_eq!(info.prompt, "test task");
    }

    #[test]
    fn test_mock_scheduler() {
        let scheduler = MockScheduler::new();
        assert!(scheduler.is_running());

        scheduler.add_job(MockJob {
            id: "job-1".to_string(),
            agent: "general".to_string(),
            prompt: "test".to_string(),
        });
    }

    #[tokio::test]
    async fn test_mock_scheduler_tick() {
        let scheduler = MockScheduler::new();
        scheduler.add_job(MockJob {
            id: "job-1".to_string(),
            agent: "general".to_string(),
            prompt: "test".to_string(),
        });

        let jobs = scheduler.tick().await;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "job-1");

        // Second tick should be empty (jobs consumed)
        let jobs = scheduler.tick().await;
        assert!(jobs.is_empty());
    }

    #[tokio::test]
    async fn test_mock_scheduler_job_info() {
        let scheduler = MockScheduler::new();
        let job = MockJob {
            id: "job-1".to_string(),
            agent: "general".to_string(),
            prompt: "test task".to_string(),
        };

        let info = scheduler.job_info(&job);
        assert_eq!(info.job_id, "job-1");
        assert_eq!(info.agent, "general");
        assert_eq!(info.prompt, "test task");
    }
