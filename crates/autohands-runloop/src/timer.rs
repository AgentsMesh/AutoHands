//! Timer - High-level scheduling abstraction built on top of RunLoop events.
//!
//! IMPORTANT: Timer is NOT a built-in RunLoop component; it's a higher-level
//! abstraction that generates events with scheduled_at timestamps.
//!
//! This design keeps RunLoop simple (only cares about events and their
//! scheduled times) while providing flexible scheduling through Timer.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde_json::json;
use tracing::debug;
use uuid::Uuid;

use crate::task::{Task, TaskPriority, TaskSource};
use crate::RunLoop;

/// Timer - High-level scheduling abstraction.
///
/// Timer generates events with scheduled_at timestamps that flow
/// through the RunLoop's event queue. When the scheduled time arrives,
/// the event is promoted from the delayed queue to the immediate queue.
pub struct Timer {
    /// Timer ID.
    id: String,

    /// Timer interval.
    interval: Duration,

    /// Whether the timer repeats.
    repeating: bool,

    /// Event factory function.
    event_factory: Arc<dyn Fn() -> Task + Send + Sync>,

    /// RunLoop reference.
    run_loop: Arc<RunLoop>,

    /// Whether the timer is valid (not cancelled).
    valid: AtomicBool,

    /// Fire count.
    fire_count: AtomicU64,
}

impl Timer {
    /// Create a one-shot timer.
    ///
    /// Fires once after the specified delay.
    pub fn once<F>(
        id: impl Into<String>,
        delay: Duration,
        event_factory: F,
        run_loop: Arc<RunLoop>,
    ) -> Arc<Self>
    where
        F: Fn() -> Task + Send + Sync + 'static,
    {
        let timer = Arc::new(Self {
            id: id.into(),
            interval: delay,
            repeating: false,
            event_factory: Arc::new(event_factory),
            run_loop,
            valid: AtomicBool::new(true),
            fire_count: AtomicU64::new(0),
        });

        // Schedule the first (and only) fire
        timer.schedule_next();
        timer
    }

    /// Create a repeating timer.
    ///
    /// Fires repeatedly at the specified interval.
    pub fn repeating<F>(
        id: impl Into<String>,
        interval: Duration,
        event_factory: F,
        run_loop: Arc<RunLoop>,
    ) -> Arc<Self>
    where
        F: Fn() -> Task + Send + Sync + 'static,
    {
        let timer = Arc::new(Self {
            id: id.into(),
            interval,
            repeating: true,
            event_factory: Arc::new(event_factory),
            run_loop,
            valid: AtomicBool::new(true),
            fire_count: AtomicU64::new(0),
        });

        // Schedule the first fire
        timer.schedule_next();
        timer
    }

    /// Get the timer ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the interval.
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Check if the timer repeats.
    pub fn is_repeating(&self) -> bool {
        self.repeating
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
    pub fn cancel(&self) {
        self.valid.store(false, Ordering::SeqCst);
        debug!("Timer {} cancelled", self.id);
    }

    /// Schedule the next fire.
    fn schedule_next(&self) {
        if !self.is_valid() {
            return;
        }

        let scheduled_at = Utc::now()
            + chrono::Duration::from_std(self.interval)
                .unwrap_or_else(|_| chrono::Duration::seconds(1));

        let mut event = (self.event_factory)();
        event.scheduled_at = Some(scheduled_at);

        // Add timer metadata for repeating timers
        if self.repeating {
            event.metadata.insert("timer_id".to_string(), json!(self.id));
            event.metadata.insert("timer_repeat".to_string(), json!(true));
            event.metadata.insert(
                "timer_interval_ms".to_string(),
                json!(self.interval.as_millis()),
            );
        }

        debug!(
            "Timer {} scheduled for {}",
            self.id,
            scheduled_at.to_rfc3339()
        );

        // Inject into RunLoop
        let run_loop = self.run_loop.clone();
        tokio::spawn(async move {
            if let Err(e) = run_loop.inject_task(event).await {
                tracing::warn!("Failed to inject timer task: {}", e);
            }
        });

        self.fire_count.fetch_add(1, Ordering::Relaxed);
    }
}

/// Timer builder for convenient timer creation.
pub struct TimerBuilder {
    id: Option<String>,
    interval: Duration,
    repeating: bool,
    priority: TaskPriority,
    task_type: String,
    payload: serde_json::Value,
}

impl TimerBuilder {
    /// Create a new timer builder.
    pub fn new() -> Self {
        Self {
            id: None,
            interval: Duration::from_secs(1),
            repeating: false,
            priority: TaskPriority::Normal,
            task_type: "timer:fired".to_string(),
            payload: serde_json::Value::Null,
        }
    }

    /// Set the timer ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the interval.
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Make the timer repeating.
    pub fn repeating(mut self) -> Self {
        self.repeating = true;
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

    /// Build the timer.
    pub fn build(self, run_loop: Arc<RunLoop>) -> Arc<Timer> {
        let id = self.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let task_type = self.task_type;
        let payload = self.payload;
        let priority = self.priority;

        let task_factory = move || {
            Task::new(task_type.clone(), payload.clone())
                .with_priority(priority)
                .with_source(TaskSource::Timer)
        };

        if self.repeating {
            Timer::repeating(id, self.interval, task_factory, run_loop)
        } else {
            Timer::once(id, self.interval, task_factory, run_loop)
        }
    }
}

impl Default for TimerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for creating common timers.
pub mod timers {
    use super::*;

    /// Create a heartbeat timer.
    pub fn heartbeat(interval_secs: u64, run_loop: Arc<RunLoop>) -> Arc<Timer> {
        TimerBuilder::new()
            .id("heartbeat")
            .interval(Duration::from_secs(interval_secs))
            .repeating()
            .task_type("system:heartbeat")
            .priority(TaskPriority::Low)
            .build(run_loop)
    }

    /// Create a cleanup timer.
    pub fn cleanup(interval_secs: u64, run_loop: Arc<RunLoop>) -> Arc<Timer> {
        TimerBuilder::new()
            .id("cleanup")
            .interval(Duration::from_secs(interval_secs))
            .repeating()
            .task_type("system:cleanup")
            .priority(TaskPriority::Low)
            .build(run_loop)
    }

    /// Create a metrics timer.
    pub fn metrics(interval_secs: u64, run_loop: Arc<RunLoop>) -> Arc<Timer> {
        TimerBuilder::new()
            .id("metrics")
            .interval(Duration::from_secs(interval_secs))
            .repeating()
            .task_type("system:metrics")
            .priority(TaskPriority::Low)
            .build(run_loop)
    }

    /// Create a one-shot reminder.
    pub fn reminder(
        id: impl Into<String>,
        delay: Duration,
        message: impl Into<String>,
        run_loop: Arc<RunLoop>,
    ) -> Arc<Timer> {
        let msg = message.into();
        TimerBuilder::new()
            .id(id)
            .interval(delay)
            .task_type("timer:reminder")
            .payload(json!({ "message": msg }))
            .build(run_loop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RunLoopConfig;

    #[tokio::test]
    async fn test_timer_once() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = Timer::once(
            "test-once",
            Duration::from_millis(100),
            || Task::new("test:event", serde_json::Value::Null),
            run_loop.clone(),
        );

        assert_eq!(timer.id(), "test-once");
        assert!(!timer.is_repeating());
        assert!(timer.is_valid());
        assert_eq!(timer.fire_count(), 1);
    }

    #[tokio::test]
    async fn test_timer_repeating() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = Timer::repeating(
            "test-repeat",
            Duration::from_millis(100),
            || Task::new("test:event", serde_json::Value::Null),
            run_loop.clone(),
        );

        assert_eq!(timer.id(), "test-repeat");
        assert!(timer.is_repeating());
        assert!(timer.is_valid());
    }

    #[tokio::test]
    async fn test_timer_cancel() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = Timer::once(
            "test-cancel",
            Duration::from_secs(60),
            || Task::new("test:event", serde_json::Value::Null),
            run_loop.clone(),
        );

        assert!(timer.is_valid());
        timer.cancel();
        assert!(!timer.is_valid());
    }

    #[tokio::test]
    async fn test_timer_builder() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = TimerBuilder::new()
            .id("builder-test")
            .interval(Duration::from_secs(5))
            .repeating()
            .task_type("custom:event")
            .priority(TaskPriority::High)
            .payload(json!({"key": "value"}))
            .build(run_loop);

        assert_eq!(timer.id(), "builder-test");
        assert!(timer.is_repeating());
        assert_eq!(timer.interval(), Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_timer_builder_default() {
        let builder = TimerBuilder::default();
        assert!(builder.id.is_none());
        assert!(!builder.repeating);
    }

    #[tokio::test]
    async fn test_heartbeat_timer() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = timers::heartbeat(10, run_loop);
        assert_eq!(timer.id(), "heartbeat");
        assert!(timer.is_repeating());
    }

    #[tokio::test]
    async fn test_reminder_timer() {
        let run_loop = Arc::new(RunLoop::new(RunLoopConfig::default()));

        let timer = timers::reminder("my-reminder", Duration::from_secs(300), "Check email", run_loop);
        assert_eq!(timer.id(), "my-reminder");
        assert!(!timer.is_repeating());
    }
}
