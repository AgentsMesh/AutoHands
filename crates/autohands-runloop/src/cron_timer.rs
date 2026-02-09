//! CronTimer - Cron expression based timer built on top of RunLoop events.
//!
//! Similar to Timer, CronTimer generates events with scheduled_at timestamps.
//! The key difference is that CronTimer uses cron expressions for scheduling
//! instead of fixed intervals.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use chrono::Utc;
use cron::Schedule;
use serde_json::json;
use tracing::debug;
use uuid::Uuid;

use crate::task::{Task, TaskPriority, TaskSource};
use crate::RunLoop;

/// CronTimer - Cron expression based timer.
///
/// CronTimer generates events according to cron schedule expressions.
/// Each time a cron time point is reached, the event is scheduled
/// and flows through the RunLoop's event queue.
pub struct CronTimer {
    /// Timer ID.
    id: String,

    /// Cron schedule.
    schedule: Schedule,

    /// Cron expression string (for display).
    cron_expr: String,

    /// Event factory function.
    event_factory: Arc<dyn Fn() -> Task + Send + Sync>,

    /// RunLoop reference.
    run_loop: Arc<RunLoop>,

    /// Whether the timer is valid (not cancelled).
    valid: AtomicBool,

    /// Fire count.
    fire_count: AtomicU64,
}

impl CronTimer {
    /// Create a new CronTimer.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique timer identifier
    /// * `cron_expr` - Cron expression (e.g., "0 */5 * * * *" for every 5 minutes)
    /// * `event_factory` - Function that creates the event to fire
    /// * `run_loop` - RunLoop reference for event injection
    ///
    /// # Cron Expression Format
    ///
    /// The cron expression follows the standard 6-field format:
    /// `second minute hour day_of_month month day_of_week`
    ///
    /// Examples:
    /// - `"0 0 * * * *"` - Every hour at minute 0
    /// - `"0 */5 * * * *"` - Every 5 minutes
    /// - `"0 0 9 * * MON-FRI"` - 9 AM on weekdays
    /// - `"0 30 4 1 * *"` - 4:30 AM on the 1st of each month
    ///
    /// # Errors
    ///
    /// Returns an error if the cron expression is invalid.
    pub fn new<F>(
        id: impl Into<String>,
        cron_expr: &str,
        event_factory: F,
        run_loop: Arc<RunLoop>,
    ) -> Result<Arc<Self>, cron::error::Error>
    where
        F: Fn() -> Task + Send + Sync + 'static,
    {
        let schedule: Schedule = cron_expr.parse()?;
        let timer = Arc::new(Self {
            id: id.into(),
            schedule,
            cron_expr: cron_expr.to_string(),
            event_factory: Arc::new(event_factory),
            run_loop,
            valid: AtomicBool::new(true),
            fire_count: AtomicU64::new(0),
        });

        // Schedule the first fire
        timer.schedule_next();
        Ok(timer)
    }

    /// Get the timer ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the cron expression.
    pub fn cron_expr(&self) -> &str {
        &self.cron_expr
    }

    /// Check if the timer is valid (not cancelled).
    pub fn is_valid(&self) -> bool {
        self.valid.load(Ordering::SeqCst)
    }

    /// Get the fire count.
    pub fn fire_count(&self) -> u64 {
        self.fire_count.load(Ordering::Relaxed)
    }

    /// Cancel the timer.
    ///
    /// After cancellation, no more events will be scheduled.
    pub fn cancel(&self) {
        self.valid.store(false, Ordering::SeqCst);
        debug!("CronTimer {} cancelled", self.id);
    }

    /// Manually trigger rescheduling.
    ///
    /// This is typically called after an event is processed to schedule
    /// the next occurrence.
    pub fn reschedule(&self) {
        self.schedule_next();
    }

    /// Get the next scheduled time.
    pub fn next_fire_time(&self) -> Option<chrono::DateTime<Utc>> {
        if !self.is_valid() {
            return None;
        }
        self.schedule.upcoming(Utc).next()
    }

    /// Schedule the next fire.
    fn schedule_next(&self) {
        if !self.is_valid() {
            return;
        }

        if let Some(next) = self.schedule.upcoming(Utc).next() {
            let mut event = (self.event_factory)();
            event.scheduled_at = Some(next);
            event
                .metadata
                .insert("cron_timer_id".to_string(), json!(self.id));
            event
                .metadata
                .insert("cron_timer_expr".to_string(), json!(self.cron_expr));

            debug!(
                "CronTimer {} scheduled for {}",
                self.id,
                next.to_rfc3339()
            );

            // Inject into RunLoop
            let run_loop = self.run_loop.clone();
            tokio::spawn(async move {
                if let Err(e) = run_loop.inject_task(event).await {
                    tracing::warn!("Failed to inject cron timer task: {}", e);
                }
            });

            self.fire_count.fetch_add(1, Ordering::Relaxed);
        } else {
            debug!("CronTimer {} has no upcoming schedule", self.id);
        }
    }
}

/// CronTimer builder for convenient creation.
pub struct CronTimerBuilder {
    id: Option<String>,
    cron_expr: String,
    priority: TaskPriority,
    task_type: String,
    payload: serde_json::Value,
}

impl CronTimerBuilder {
    /// Create a new CronTimer builder.
    ///
    /// # Arguments
    ///
    /// * `cron_expr` - Cron expression for scheduling
    pub fn new(cron_expr: impl Into<String>) -> Self {
        Self {
            id: None,
            cron_expr: cron_expr.into(),
            priority: TaskPriority::Normal,
            task_type: "cron:fired".to_string(),
            payload: serde_json::Value::Null,
        }
    }

    /// Set the timer ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the task priority.
    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the task type.
    pub fn task_type(mut self, task_type: impl Into<String>) -> Self {
        self.task_type = task_type.into();
        self
    }

    /// Set the task payload.
    pub fn payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    /// Build the CronTimer.
    ///
    /// # Errors
    ///
    /// Returns an error if the cron expression is invalid.
    pub fn build(self, run_loop: Arc<RunLoop>) -> Result<Arc<CronTimer>, cron::error::Error> {
        let id = self.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let task_type = self.task_type;
        let payload = self.payload;
        let priority = self.priority;

        let task_factory = move || {
            Task::new(task_type.clone(), payload.clone())
                .with_priority(priority)
                .with_source(TaskSource::Scheduler)
        };

        CronTimer::new(id, &self.cron_expr, task_factory, run_loop)
    }
}

/// Common cron schedule presets.
pub mod schedules {
    use super::*;

    /// Every minute.
    pub const EVERY_MINUTE: &str = "0 * * * * *";

    /// Every 5 minutes.
    pub const EVERY_5_MINUTES: &str = "0 */5 * * * *";

    /// Every 15 minutes.
    pub const EVERY_15_MINUTES: &str = "0 */15 * * * *";

    /// Every 30 minutes.
    pub const EVERY_30_MINUTES: &str = "0 */30 * * * *";

    /// Every hour.
    pub const EVERY_HOUR: &str = "0 0 * * * *";

    /// Every day at midnight.
    pub const DAILY_MIDNIGHT: &str = "0 0 0 * * *";

    /// Every day at noon.
    pub const DAILY_NOON: &str = "0 0 12 * * *";

    /// Every Monday at 9 AM.
    pub const WEEKLY_MONDAY_9AM: &str = "0 0 9 * * MON";

    /// First day of each month at midnight.
    pub const MONTHLY_FIRST: &str = "0 0 0 1 * *";

    /// Create a CronTimer that fires every N seconds.
    pub fn every_seconds(
        id: impl Into<String>,
        seconds: u32,
        run_loop: Arc<RunLoop>,
    ) -> Result<Arc<CronTimer>, cron::error::Error> {
        let cron_expr = format!("*/{} * * * * *", seconds);
        CronTimerBuilder::new(cron_expr)
            .id(id)
            .task_type("cron:tick")
            .build(run_loop)
    }

    /// Create a CronTimer that fires every N minutes.
    pub fn every_minutes(
        id: impl Into<String>,
        minutes: u32,
        run_loop: Arc<RunLoop>,
    ) -> Result<Arc<CronTimer>, cron::error::Error> {
        let cron_expr = format!("0 */{} * * * *", minutes);
        CronTimerBuilder::new(cron_expr)
            .id(id)
            .task_type("cron:tick")
            .build(run_loop)
    }

    /// Create a CronTimer that fires every N hours.
    pub fn every_hours(
        id: impl Into<String>,
        hours: u32,
        run_loop: Arc<RunLoop>,
    ) -> Result<Arc<CronTimer>, cron::error::Error> {
        let cron_expr = format!("0 0 */{} * * *", hours);
        CronTimerBuilder::new(cron_expr)
            .id(id)
            .task_type("cron:tick")
            .build(run_loop)
    }

    /// Create a daily CronTimer at specific hour and minute.
    pub fn daily_at(
        id: impl Into<String>,
        hour: u32,
        minute: u32,
        run_loop: Arc<RunLoop>,
    ) -> Result<Arc<CronTimer>, cron::error::Error> {
        let cron_expr = format!("0 {} {} * * *", minute, hour);
        CronTimerBuilder::new(cron_expr)
            .id(id)
            .task_type("cron:daily")
            .build(run_loop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RunLoopConfig;

    #[tokio::test]
    async fn test_cron_timer_creation() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = CronTimer::new(
            "test-cron",
            "0 * * * * *", // Every minute
            || Task::new("test:cron", serde_json::Value::Null),
            run_loop.clone(),
        )
        .expect("Valid cron expression");

        assert_eq!(timer.id(), "test-cron");
        assert_eq!(timer.cron_expr(), "0 * * * * *");
        assert!(timer.is_valid());
        assert_eq!(timer.fire_count(), 1); // First schedule
    }

    #[tokio::test]
    async fn test_cron_timer_invalid_expr() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let result = CronTimer::new(
            "bad-cron",
            "invalid cron expression",
            || Task::new("test", serde_json::Value::Null),
            run_loop.clone(),
        );

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cron_timer_cancel() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = CronTimer::new(
            "cancel-test",
            "0 * * * * *",
            || Task::new("test", serde_json::Value::Null),
            run_loop.clone(),
        )
        .unwrap();

        assert!(timer.is_valid());
        timer.cancel();
        assert!(!timer.is_valid());
    }

    #[tokio::test]
    async fn test_cron_timer_builder() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = CronTimerBuilder::new("0 */5 * * * *")
            .id("builder-test")
            .task_type("custom:task")
            .priority(TaskPriority::High)
            .payload(json!({"key": "value"}))
            .build(run_loop)
            .unwrap();

        assert_eq!(timer.id(), "builder-test");
        assert_eq!(timer.cron_expr(), "0 */5 * * * *");
    }

    #[tokio::test]
    async fn test_cron_timer_next_fire_time() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = CronTimer::new(
            "next-fire",
            "0 * * * * *",
            || Task::new("test", serde_json::Value::Null),
            run_loop.clone(),
        )
        .unwrap();

        let next = timer.next_fire_time();
        assert!(next.is_some());
        assert!(next.unwrap() > Utc::now());
    }

    #[tokio::test]
    async fn test_schedule_presets() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = schedules::every_seconds("every-5s", 5, run_loop.clone()).unwrap();
        assert!(timer.is_valid());

        let timer = schedules::every_minutes("every-10m", 10, run_loop.clone()).unwrap();
        assert!(timer.is_valid());

        let timer = schedules::every_hours("every-2h", 2, run_loop.clone()).unwrap();
        assert!(timer.is_valid());

        let timer = schedules::daily_at("daily-9am", 9, 0, run_loop.clone()).unwrap();
        assert!(timer.is_valid());
    }
}
