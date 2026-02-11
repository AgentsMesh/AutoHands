//! Tests for job scheduler.

use super::*;
use crate::job::definition::JobDefinition;
use crate::job::store::MemoryJobStore;
use autohands_runloop::{RunLoop, RunLoopConfig};

fn create_test_scheduler() -> Arc<JobScheduler> {
    let store: Arc<dyn JobStore> = Arc::new(MemoryJobStore::new());
    let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
    let runloop = Arc::new(RunLoopState::from_runloop(run_loop));
    Arc::new(
        JobScheduler::new(store, runloop)
            .with_check_interval(Duration::from_millis(100)),
    )
}

#[tokio::test]
async fn test_scheduler_creation() {
    let scheduler = create_test_scheduler();
    assert_eq!(scheduler.check_interval, Duration::from_millis(100));
}

#[tokio::test]
async fn test_check_due_jobs_empty() {
    let scheduler = create_test_scheduler();
    // Should succeed with no jobs
    scheduler.check_due_jobs().await.unwrap();
}

#[tokio::test]
async fn test_check_due_jobs_with_disabled_job() {
    let store = Arc::new(MemoryJobStore::new());
    let def = JobDefinition::new("test-job", "* * * * *", "test-agent", "Do task")
        .with_enabled(false);
    let job = super::super::definition::Job::new(def);
    store.save(&job).await.unwrap();

    let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));
    let runloop = Arc::new(RunLoopState::from_runloop(run_loop));
    let job_store: Arc<dyn JobStore> = store;
    let scheduler = Arc::new(JobScheduler::new(job_store, runloop));

    // Disabled job should not be submitted
    scheduler.check_due_jobs().await.unwrap();
}

#[tokio::test]
async fn test_scheduler_shutdown() {
    let scheduler = create_test_scheduler();
    let (tx, rx) = tokio::sync::watch::channel(false);

    let scheduler_clone = scheduler.clone();
    let handle = tokio::spawn(async move {
        scheduler_clone.run(rx).await;
    });

    // Let it run briefly
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Signal shutdown
    tx.send(true).unwrap();

    // Should complete within a reasonable time
    tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("Scheduler should shut down promptly")
        .expect("Scheduler task should not panic");
}
