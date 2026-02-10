//! RunLoop wakeup waiting and Source1 activity monitoring.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::run_loop::RunLoop;
use crate::source::PortMessage;

impl RunLoop {
    /// Wait for wakeup.
    pub(crate) async fn wait_for_wakeup(&self, deadline: Instant) -> crate::run_loop::WakeupSignal {
        use crate::run_loop::WakeupSignal;

        // Calculate wait timeout
        let next_delayed = self.task_queue.next_delayed_time().await;
        let wait_timeout = self.calculate_wait_timeout(deadline, next_delayed);

        let mut wakeup_rx = self.wakeup_rx.write().await;

        tokio::select! {
            // Explicit wakeup signal
            Some(signal) = wakeup_rx.recv() => signal,

            // Source1 activity
            result = self.wait_source1_activity() => {
                match result {
                    Some((source_id, msg)) => WakeupSignal::SourceReady {
                        source_id,
                        message: msg,
                    },
                    None => WakeupSignal::Explicit {
                        reason: "source1_closed".to_string(),
                    },
                }
            }

            // Timeout
            _ = tokio::time::sleep(wait_timeout) => {
                WakeupSignal::Explicit {
                    reason: "timeout".to_string(),
                }
            }
        }
    }

    /// Calculate wait timeout.
    pub(crate) fn calculate_wait_timeout(
        &self,
        deadline: Instant,
        next_delayed: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Duration {
        let now = Instant::now();
        let to_deadline = deadline.saturating_duration_since(now);

        match next_delayed {
            Some(delayed_time) => {
                let delayed_instant = {
                    let now_utc = chrono::Utc::now();
                    let diff = delayed_time - now_utc;
                    if diff.num_milliseconds() <= 0 {
                        Duration::ZERO
                    } else {
                        Duration::from_millis(diff.num_milliseconds() as u64)
                    }
                };
                std::cmp::min(to_deadline, delayed_instant)
            }
            None => std::cmp::min(to_deadline, Duration::from_secs(1)), // Default 1s poll
        }
    }

    /// Wait for Source1 activity.
    ///
    /// Uses FuturesUnordered to concurrently wait on all Source1 receivers.
    /// This is a true event-driven implementation without polling.
    pub(crate) async fn wait_source1_activity(&self) -> Option<(String, PortMessage)> {
        use futures::stream::{FuturesUnordered, StreamExt};
        use tokio::sync::Mutex;
        use tokio::sync::mpsc;

        // Collect valid receivers with their source info (only need read lock)
        let receivers = self.source1_receivers.read().await;

        // Collect Arc clones so we can release the read lock before awaiting
        let receiver_infos: Vec<(String, Arc<Mutex<mpsc::Receiver<PortMessage>>>)> = receivers
            .iter()
            .filter(|r| r.source.is_valid())
            .map(|r| (r.source.id().to_string(), r.receiver_arc()))
            .collect();

        drop(receivers); // Release read lock before async waiting

        if receiver_infos.is_empty() {
            // No valid sources - return pending future that never completes
            // This ensures the select! will choose wakeup_rx or timeout instead
            return std::future::pending().await;
        }

        // Build FuturesUnordered to wait on all receivers concurrently
        let mut futures: FuturesUnordered<_> = receiver_infos
            .into_iter()
            .map(|(source_id, receiver_arc)| async move {
                let mut guard = receiver_arc.lock().await;
                match guard.recv().await {
                    Some(msg) => Some((source_id, msg)),
                    None => None, // Channel closed
                }
            })
            .collect();

        // Wait for the first receiver to produce a message
        while let Some(result) = futures.next().await {
            if let Some((source_id, msg)) = result {
                return Some((source_id, msg));
            }
            // If None, the channel was closed - continue waiting on others
        }

        // All channels closed
        None
    }
}
