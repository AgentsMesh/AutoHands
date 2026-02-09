//! Job definition and status.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Job status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is enabled and waiting for next run.
    Enabled,
    /// Job is currently running.
    Running,
    /// Job is disabled.
    Disabled,
    /// Job completed last run.
    Completed,
    /// Job failed last run.
    Failed,
}

impl Default for JobStatus {
    fn default() -> Self {
        JobStatus::Enabled
    }
}

/// Job definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDefinition {
    /// Unique job ID.
    pub id: String,
    /// Cron schedule expression.
    pub schedule: String,
    /// Agent to run the job.
    pub agent: String,
    /// Prompt to execute.
    pub prompt: String,
    /// Optional description.
    pub description: Option<String>,
    /// Whether job is enabled.
    pub enabled: bool,
}

impl JobDefinition {
    /// Create a new job definition.
    pub fn new(
        id: impl Into<String>,
        schedule: impl Into<String>,
        agent: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            schedule: schedule.into(),
            agent: agent.into(),
            prompt: prompt.into(),
            description: None,
            enabled: true,
        }
    }

    /// Add a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set enabled state.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Runtime job instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique instance ID.
    pub instance_id: Uuid,
    /// Job definition.
    pub definition: JobDefinition,
    /// Current status.
    pub status: JobStatus,
    /// Last run time.
    pub last_run: Option<DateTime<Utc>>,
    /// Next scheduled run time.
    pub next_run: Option<DateTime<Utc>>,
    /// Number of executions.
    pub run_count: u64,
    /// Last error message.
    pub last_error: Option<String>,
}

impl Job {
    /// Create a new job from a definition.
    pub fn new(definition: JobDefinition) -> Self {
        Self {
            instance_id: Uuid::new_v4(),
            definition,
            status: JobStatus::Enabled,
            last_run: None,
            next_run: None,
            run_count: 0,
            last_error: None,
        }
    }

    /// Mark the job as running.
    pub fn start_run(&mut self) {
        self.status = JobStatus::Running;
    }

    /// Mark the job as completed.
    pub fn complete_run(&mut self) {
        self.status = JobStatus::Completed;
        self.last_run = Some(Utc::now());
        self.run_count += 1;
        self.last_error = None;
    }

    /// Mark the job as failed.
    pub fn fail_run(&mut self, error: impl Into<String>) {
        self.status = JobStatus::Failed;
        self.last_run = Some(Utc::now());
        self.run_count += 1;
        self.last_error = Some(error.into());
    }

    /// Re-enable the job after completion/failure.
    pub fn re_enable(&mut self) {
        if self.definition.enabled {
            self.status = JobStatus::Enabled;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_definition_new() {
        let def = JobDefinition::new("test-job", "0 * * * *", "test-agent", "Test prompt");
        assert_eq!(def.id, "test-job");
        assert_eq!(def.schedule, "0 * * * *");
        assert_eq!(def.agent, "test-agent");
        assert!(def.enabled);
    }

    #[test]
    fn test_job_definition_with_description() {
        let def = JobDefinition::new("job", "* * * * *", "agent", "prompt")
            .with_description("My description");
        assert_eq!(def.description, Some("My description".to_string()));
    }

    #[test]
    fn test_job_new() {
        let def = JobDefinition::new("job", "* * * * *", "agent", "prompt");
        let job = Job::new(def);
        assert_eq!(job.status, JobStatus::Enabled);
        assert_eq!(job.run_count, 0);
        assert!(job.last_run.is_none());
    }

    #[test]
    fn test_job_lifecycle() {
        let def = JobDefinition::new("job", "* * * * *", "agent", "prompt");
        let mut job = Job::new(def);

        job.start_run();
        assert_eq!(job.status, JobStatus::Running);

        job.complete_run();
        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(job.run_count, 1);
        assert!(job.last_run.is_some());

        job.re_enable();
        assert_eq!(job.status, JobStatus::Enabled);
    }

    #[test]
    fn test_job_fail() {
        let def = JobDefinition::new("job", "* * * * *", "agent", "prompt");
        let mut job = Job::new(def);

        job.start_run();
        job.fail_run("Something went wrong");

        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.last_error, Some("Something went wrong".to_string()));
    }
}
