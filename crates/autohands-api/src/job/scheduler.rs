//! Job scheduler that checks due jobs periodically and submits them via RunLoop.

use std::str::FromStr;
use std::sync::Arc;

use chrono::Utc;
use cron::Schedule;
use tokio::time::{self, Duration};
use tracing::{debug, error, info, warn};

use super::definition::JobStatus;
use super::store::JobStore;
use crate::runloop_bridge::RunLoopState;

/// Job scheduler that periodically checks for due jobs and submits them.
pub struct JobScheduler {
    job_store: Arc<dyn JobStore>,
    runloop: Arc<RunLoopState>,
    check_interval: Duration,
}

impl JobScheduler {
    /// Create a new job scheduler.
    pub fn new(
        job_store: Arc<dyn JobStore>,
        runloop: Arc<RunLoopState>,
    ) -> Self {
        Self {
            job_store,
            runloop,
            check_interval: Duration::from_secs(60),
        }
    }

    /// Set the check interval.
    pub fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    /// Start the scheduler loop. Runs until the provided cancellation token fires.
    pub async fn run(self: Arc<Self>, cancel: tokio::sync::watch::Receiver<bool>) {
        info!(
            "Job scheduler started (check interval: {:?})",
            self.check_interval
        );

        let mut interval = time::interval(self.check_interval);
        let mut cancel = cancel;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.check_due_jobs().await {
                        error!("Job scheduler check failed: {}", e);
                    }
                }
                _ = cancel.changed() => {
                    info!("Job scheduler shutting down");
                    break;
                }
            }
        }
    }

    /// Check for due jobs and submit them.
    async fn check_due_jobs(&self) -> Result<(), crate::error::InterfaceError> {
        let jobs = self.job_store.load_all().await?;
        let now = Utc::now();

        for mut job in jobs {
            if job.status != JobStatus::Enabled || !job.definition.enabled {
                continue;
            }

            // Parse cron schedule to determine if the job is due
            let schedule = match Schedule::from_str(&job.definition.schedule) {
                Ok(s) => s,
                Err(e) => {
                    warn!(
                        "Invalid cron expression for job '{}': {} - {}",
                        job.definition.id, job.definition.schedule, e
                    );
                    continue;
                }
            };

            // Find the next scheduled time after last_run (or epoch if never run)
            let after = job.last_run.unwrap_or_else(|| {
                chrono::DateTime::from_timestamp(0, 0).unwrap_or_else(|| Utc::now())
            });

            let next = schedule.after(&after).next();
            if let Some(next_time) = next {
                if next_time <= now {
                    debug!("Job '{}' is due, submitting", job.definition.id);
                    self.submit_job(&mut job).await;
                }
            }
        }

        Ok(())
    }

    /// Submit a job for execution via RunLoop.
    async fn submit_job(&self, job: &mut super::definition::Job) {
        job.start_run();

        let payload = serde_json::json!({
            "prompt": job.definition.prompt,
            "agent_id": job.definition.agent,
            "job_id": job.definition.id,
            "source": "scheduler",
        });

        match self
            .runloop
            .submit_task("agent:execute", payload, None)
            .await
        {
            Ok(()) => {
                info!("Job '{}' submitted to RunLoop", job.definition.id);
                job.complete_run();
                job.re_enable();
            }
            Err(e) => {
                error!("Failed to submit job '{}': {}", job.definition.id, e);
                job.fail_run(e.to_string());
                job.re_enable();
            }
        }

        // Persist updated status
        if let Err(e) = self.job_store.update_status(job).await {
            error!(
                "Failed to update job status for '{}': {}",
                job.definition.id, e
            );
        }
    }
}

#[cfg(test)]
#[path = "scheduler_tests.rs"]
mod tests;
