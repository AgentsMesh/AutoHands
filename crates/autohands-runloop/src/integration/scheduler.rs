//! Scheduler integration with RunLoop.
//!
//! Provides a Source0 adapter for the existing Scheduler component.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tracing::debug;

use crate::error::RunLoopResult;
use crate::task::{Task, TaskPriority, TaskSource};
use crate::mode::RunLoopMode;
use crate::source::Source0;

/// Scheduler control trait.
///
/// Implement this trait to integrate a scheduler with RunLoop.
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

/// Scheduler Source0 adapter.
///
/// Wraps a scheduler as a Source0 for the RunLoop.
/// On each tick, it checks for due jobs and produces events.
pub struct SchedulerSource0<S>
where
    S: SchedulerTick,
{
    id: String,
    scheduler: Arc<S>,
    signaled: AtomicBool,
    cancelled: AtomicBool,
    modes: Vec<RunLoopMode>,
}

impl<S> SchedulerSource0<S>
where
    S: SchedulerTick + 'static,
{
    /// Create a new Scheduler Source0.
    pub fn new(id: impl Into<String>, scheduler: Arc<S>) -> Self {
        Self {
            id: id.into(),
            scheduler,
            signaled: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            modes: vec![RunLoopMode::Default],
        }
    }

    /// Set the modes this source is associated with.
    pub fn with_modes(mut self, modes: Vec<RunLoopMode>) -> Self {
        self.modes = modes;
        self
    }

    /// Signal the source to indicate it should be checked.
    ///
    /// Call this periodically (e.g., every second) or when you know
    /// there might be due jobs.
    pub fn signal_tick(&self) {
        self.signal();
    }
}

#[async_trait]
impl<S> Source0 for SchedulerSource0<S>
where
    S: SchedulerTick + 'static,
{
    fn id(&self) -> &str {
        &self.id
    }

    fn is_signaled(&self) -> bool {
        self.signaled.load(Ordering::SeqCst)
    }

    fn signal(&self) {
        self.signaled.store(true, Ordering::SeqCst);
    }

    fn clear_signal(&self) {
        self.signaled.store(false, Ordering::SeqCst);
    }

    async fn perform(&self) -> RunLoopResult<Vec<Task>> {
        self.clear_signal();

        if !self.scheduler.is_running() {
            return Ok(Vec::new());
        }

        // Get due jobs
        let due_jobs = self.scheduler.tick().await;

        if due_jobs.is_empty() {
            return Ok(Vec::new());
        }

        debug!("Scheduler tick: {} jobs due", due_jobs.len());

        // Convert jobs to events
        let events: Vec<Task> = due_jobs
            .iter()
            .map(|job| {
                let info = self.scheduler.job_info(job);
                Task::new(
                    "scheduler:job:due",
                    json!({
                        "job_id": info.job_id,
                        "agent": info.agent,
                        "prompt": info.prompt,
                    }),
                )
                .with_source(TaskSource::Scheduler)
                .with_priority(TaskPriority::Normal)
            })
            .collect();

        Ok(events)
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    fn modes(&self) -> &[RunLoopMode] {
        &self.modes
    }

    fn is_valid(&self) -> bool {
        !self.cancelled.load(Ordering::SeqCst)
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
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_source0_new() {
        let scheduler = Arc::new(MockScheduler::new());
        let source = SchedulerSource0::new("scheduler", scheduler);

        assert_eq!(source.id(), "scheduler");
        assert!(!source.is_signaled());
        assert!(source.is_valid());
    }

    #[tokio::test]
    async fn test_scheduler_source0_signal() {
        let scheduler = Arc::new(MockScheduler::new());
        let source = SchedulerSource0::new("scheduler", scheduler);

        assert!(!source.is_signaled());
        source.signal_tick();
        assert!(source.is_signaled());
    }

    #[tokio::test]
    async fn test_scheduler_source0_perform() {
        let scheduler = Arc::new(MockScheduler::new());
        scheduler.add_job(MockJob {
            id: "job-1".to_string(),
            agent: "general".to_string(),
            prompt: "test task".to_string(),
        });

        let source = SchedulerSource0::new("scheduler", scheduler);
        source.signal();

        let events = source.perform().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].task_type, "scheduler:job:due");
    }

    #[tokio::test]
    async fn test_scheduler_source0_cancel() {
        let scheduler = Arc::new(MockScheduler::new());
        let source = SchedulerSource0::new("scheduler", scheduler);

        assert!(source.is_valid());
        source.cancel();
        assert!(!source.is_valid());
    }
}
