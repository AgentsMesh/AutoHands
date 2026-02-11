//! Scheduler integration via TaskSubmitter.
//!
//! Provides `SchedulerInjector` which polls a scheduler for due jobs
//! and injects them as tasks via `TaskSubmitter`. Decoupled from
//! RunLoop internals.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tracing::{debug, info, warn};

use autohands_protocols::extension::TaskSubmitter;

/// Scheduler control trait.
///
/// Implement this trait to integrate a scheduler with the task system.
#[async_trait]
pub trait SchedulerTick: Send + Sync {
    /// Represents a due job.
    type Job: Send + Sync;

    /// Run one scheduling cycle, returning due jobs.
    async fn tick(&self) -> Vec<Self::Job>;

    /// Check if the scheduler is running.
    fn is_running(&self) -> bool;

    /// Get job information for event creation.
    fn job_info(&self, job: &Self::Job) -> JobInfo;
}

/// Job information for event creation.
#[derive(Debug, Clone)]
pub struct JobInfo {
    pub job_id: String,
    pub agent: String,
    pub prompt: String,
}

/// Scheduler injector.
///
/// Polls a scheduler for due jobs and injects them as tasks via
/// `TaskSubmitter`. Decoupled from RunLoop internals.
pub struct SchedulerInjector<S>
where
    S: SchedulerTick,
{
    scheduler: Arc<S>,
    task_submitter: Arc<dyn TaskSubmitter>,
    running: AtomicBool,
}

impl<S> SchedulerInjector<S>
where
    S: SchedulerTick + 'static,
{
    /// Create a new scheduler injector.
    pub fn new(scheduler: Arc<S>, task_submitter: Arc<dyn TaskSubmitter>) -> Self {
        Self {
            scheduler,
            task_submitter,
            running: AtomicBool::new(false),
        }
    }

    /// Start the scheduler polling loop in a background task.
    ///
    /// Polls the scheduler every `interval` and injects due jobs as tasks.
    pub fn start(self: &Arc<Self>, interval: Duration) {
        self.running.store(true, Ordering::SeqCst);
        let this = self.clone();

        tokio::spawn(async move {
            info!("SchedulerInjector started (interval={}ms)", interval.as_millis());

            while this.running.load(Ordering::SeqCst) {
                if this.scheduler.is_running() {
                    let due_jobs = this.scheduler.tick().await;

                    if !due_jobs.is_empty() {
                        debug!("Scheduler tick: {} jobs due", due_jobs.len());
                    }

                    for job in &due_jobs {
                        let info = this.scheduler.job_info(job);
                        if let Err(e) = this.task_submitter
                            .submit_task(
                                "scheduler:job:due",
                                json!({
                                    "job_id": info.job_id,
                                    "agent": info.agent,
                                    "prompt": info.prompt,
                                }),
                                None,
                            )
                            .await
                        {
                            warn!("Failed to inject scheduler task: {}", e);
                        }
                    }
                }

                tokio::time::sleep(interval).await;
            }

            info!("SchedulerInjector stopped");
        });
    }

    /// Stop the scheduler polling loop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if the injector is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Simple mock scheduler for testing.
#[cfg(test)]
pub struct MockScheduler {
    running: AtomicBool,
    jobs: parking_lot::RwLock<Vec<MockJob>>,
}

#[cfg(test)]
#[derive(Clone)]
pub struct MockJob {
    pub id: String,
    pub agent: String,
    pub prompt: String,
}

#[cfg(test)]
impl MockScheduler {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(true),
            jobs: parking_lot::RwLock::new(Vec::new()),
        }
    }

    pub fn add_job(&self, job: MockJob) {
        self.jobs.write().push(job);
    }
}

#[cfg(test)]
#[async_trait]
impl SchedulerTick for MockScheduler {
    type Job = MockJob;

    async fn tick(&self) -> Vec<Self::Job> {
        let mut jobs = self.jobs.write();
        std::mem::take(&mut *jobs)
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn job_info(&self, job: &Self::Job) -> JobInfo {
        JobInfo {
            job_id: job.id.clone(),
            agent: job.agent.clone(),
            prompt: job.prompt.clone(),
        }
    }
}

#[cfg(test)]
#[path = "scheduler_tests.rs"]
mod tests;
