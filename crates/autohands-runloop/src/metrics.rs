//! RunLoop metrics collection.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use chrono::{DateTime, Utc};

/// RunLoop metrics.
#[derive(Debug, Default)]
pub struct RunLoopMetrics {
    /// Total number of loop iterations.
    pub iterations: AtomicU64,

    /// Total events processed.
    pub events_processed: AtomicU64,

    /// Total events enqueued.
    pub events_enqueued: AtomicU64,

    /// Total Source0 performs.
    pub source0_performs: AtomicU64,

    /// Total Source1 messages handled.
    pub source1_messages: AtomicU64,

    /// Total observer notifications.
    pub observer_notifications: AtomicU64,

    /// Total time spent waiting (microseconds).
    pub wait_time_us: AtomicU64,

    /// Total time spent processing (microseconds).
    pub process_time_us: AtomicU64,

    /// Number of wakeups.
    pub wakeups: AtomicU64,

    /// Current pending events count.
    pub pending_events: AtomicU64,

    /// Current active spawned tasks count.
    pub active_tasks: AtomicU64,

    /// Start time.
    start_time: parking_lot::RwLock<Option<Instant>>,
}

impl RunLoopMetrics {
    /// Create new metrics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the start of the RunLoop.
    pub fn mark_start(&self) {
        *self.start_time.write() = Some(Instant::now());
    }

    /// Get uptime in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.start_time
            .read()
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }

    /// Record a loop iteration.
    pub fn record_iteration(&self) {
        self.iterations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record events processed.
    pub fn record_events_processed(&self, count: u64) {
        self.events_processed.fetch_add(count, Ordering::Relaxed);
    }

    /// Record event enqueued.
    pub fn record_event_enqueued(&self) {
        self.events_enqueued.fetch_add(1, Ordering::Relaxed);
    }

    /// Record Source0 perform.
    pub fn record_source0_perform(&self) {
        self.source0_performs.fetch_add(1, Ordering::Relaxed);
    }

    /// Record Source1 message.
    pub fn record_source1_message(&self) {
        self.source1_messages.fetch_add(1, Ordering::Relaxed);
    }

    /// Record observer notification.
    pub fn record_observer_notification(&self) {
        self.observer_notifications.fetch_add(1, Ordering::Relaxed);
    }

    /// Record wait time.
    pub fn record_wait_time(&self, duration_us: u64) {
        self.wait_time_us.fetch_add(duration_us, Ordering::Relaxed);
    }

    /// Record process time.
    pub fn record_process_time(&self, duration_us: u64) {
        self.process_time_us.fetch_add(duration_us, Ordering::Relaxed);
    }

    /// Record a wakeup.
    pub fn record_wakeup(&self) {
        self.wakeups.fetch_add(1, Ordering::Relaxed);
    }

    /// Set pending events count.
    pub fn set_pending_events(&self, count: u64) {
        self.pending_events.store(count, Ordering::Relaxed);
    }

    /// Set active spawned tasks count.
    pub fn set_active_tasks(&self, count: u64) {
        self.active_tasks.store(count, Ordering::Relaxed);
    }

    /// Get a snapshot of the metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            timestamp: Utc::now(),
            uptime_secs: self.uptime_secs(),
            iterations: self.iterations.load(Ordering::Relaxed),
            events_processed: self.events_processed.load(Ordering::Relaxed),
            events_enqueued: self.events_enqueued.load(Ordering::Relaxed),
            source0_performs: self.source0_performs.load(Ordering::Relaxed),
            source1_messages: self.source1_messages.load(Ordering::Relaxed),
            observer_notifications: self.observer_notifications.load(Ordering::Relaxed),
            wait_time_us: self.wait_time_us.load(Ordering::Relaxed),
            process_time_us: self.process_time_us.load(Ordering::Relaxed),
            wakeups: self.wakeups.load(Ordering::Relaxed),
            pending_events: self.pending_events.load(Ordering::Relaxed),
            active_tasks: self.active_tasks.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of metrics at a point in time.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub uptime_secs: u64,
    pub iterations: u64,
    pub events_processed: u64,
    pub events_enqueued: u64,
    pub source0_performs: u64,
    pub source1_messages: u64,
    pub observer_notifications: u64,
    pub wait_time_us: u64,
    pub process_time_us: u64,
    pub wakeups: u64,
    pub pending_events: u64,
    pub active_tasks: u64,
}

impl MetricsSnapshot {
    /// Calculate events per second.
    pub fn events_per_second(&self) -> f64 {
        if self.uptime_secs == 0 {
            return 0.0;
        }
        self.events_processed as f64 / self.uptime_secs as f64
    }

    /// Calculate average wait time in milliseconds.
    pub fn avg_wait_time_ms(&self) -> f64 {
        if self.wakeups == 0 {
            return 0.0;
        }
        (self.wait_time_us as f64 / self.wakeups as f64) / 1000.0
    }

    /// Calculate average process time in milliseconds.
    pub fn avg_process_time_ms(&self) -> f64 {
        if self.iterations == 0 {
            return 0.0;
        }
        (self.process_time_us as f64 / self.iterations as f64) / 1000.0
    }
}

#[cfg(test)]
#[path = "metrics_tests.rs"]
mod tests;
