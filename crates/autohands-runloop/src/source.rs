//! Source definitions for the RunLoop.
//!
//! Implements the dual-track source design inspired by iOS CFRunLoop:
//! - Source0: Manually triggered sources (like CFRunLoopSourceContext version 0)
//! - Source1: Port-triggered sources (like CFRunLoopSourceContext version 1)

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};

use crate::error::RunLoopResult;
use crate::task::Task;
use crate::mode::RunLoopMode;

/// Source0 - Manually triggered source.
///
/// Similar to CFRunLoopSourceContext version 0.
/// Requires explicit signal() + wakeup() to trigger.
/// Suitable for: Scheduler tick, Agent-generated tasks.
#[async_trait]
pub trait Source0: Send + Sync {
    /// Get the source ID.
    fn id(&self) -> &str;

    /// Check if the source has been signaled.
    fn is_signaled(&self) -> bool;

    /// Mark the source as signaled.
    /// Must be combined with RunLoop::wakeup() to trigger processing.
    /// Similar to CFRunLoopSourceSignal().
    fn signal(&self);

    /// Clear the signal state.
    fn clear_signal(&self);

    /// Perform the source's work, returning any generated tasks.
    async fn perform(&self) -> RunLoopResult<Vec<Task>>;

    /// Cancel the source.
    fn cancel(&self);

    /// Get the modes this source is associated with.
    fn modes(&self) -> &[RunLoopMode];

    /// Check if the source is valid (not cancelled).
    fn is_valid(&self) -> bool {
        true
    }
}

/// Source1 - Port-triggered source.
///
/// Similar to CFRunLoopSourceContext version 1.
/// Automatically triggered via channel, no manual signal needed.
/// Suitable for: WebSocket, Webhook, FileWatcher.
#[async_trait]
pub trait Source1: Send + Sync {
    /// Get the source ID.
    fn id(&self) -> &str;

    /// Handle a port message, returning any generated tasks.
    async fn handle(&self, msg: PortMessage) -> RunLoopResult<Vec<Task>>;

    /// Get the modes this source is associated with.
    fn modes(&self) -> &[RunLoopMode];

    /// Check if the source is valid.
    fn is_valid(&self) -> bool {
        true
    }

    /// Cancel the source.
    fn cancel(&self);
}

/// Port message for Source1.
///
/// Similar to Mach Message in iOS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMessage {
    /// Source ID that sent this message.
    pub source_id: String,

    /// Message payload.
    pub payload: serde_json::Value,

    /// Timestamp when the message was created.
    pub timestamp: DateTime<Utc>,
}

impl PortMessage {
    /// Create a new port message.
    pub fn new(source_id: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            source_id: source_id.into(),
            payload,
            timestamp: Utc::now(),
        }
    }
}

/// Base implementation for Source0 with common functionality.
pub struct Source0Base {
    id: String,
    signaled: AtomicBool,
    cancelled: AtomicBool,
    modes: Vec<RunLoopMode>,
}

impl Source0Base {
    /// Create a new Source0Base.
    pub fn new(id: impl Into<String>, modes: Vec<RunLoopMode>) -> Self {
        Self {
            id: id.into(),
            signaled: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            modes,
        }
    }

    /// Get the source ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Check if signaled.
    pub fn is_signaled(&self) -> bool {
        self.signaled.load(Ordering::SeqCst)
    }

    /// Signal the source.
    pub fn signal(&self) {
        self.signaled.store(true, Ordering::SeqCst);
    }

    /// Clear the signal.
    pub fn clear_signal(&self) {
        self.signaled.store(false, Ordering::SeqCst);
    }

    /// Cancel the source.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Check if valid (not cancelled).
    pub fn is_valid(&self) -> bool {
        !self.is_cancelled()
    }

    /// Get the modes.
    pub fn modes(&self) -> &[RunLoopMode] {
        &self.modes
    }
}

/// Source1 receiver wrapper.
///
/// Wraps a Source1 trait object with its message receiver.
/// The receiver is wrapped in Arc<Mutex<...>> to allow concurrent
/// async waiting on multiple receivers without holding the parent lock.
pub struct Source1Receiver {
    /// The source implementation.
    pub source: Arc<dyn Source1>,

    /// Message receiver (wrapped for concurrent access).
    pub receiver: Arc<Mutex<mpsc::Receiver<PortMessage>>>,
}

impl Source1Receiver {
    /// Create a new Source1Receiver.
    pub fn new(source: Arc<dyn Source1>, receiver: mpsc::Receiver<PortMessage>) -> Self {
        Self {
            source,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    /// Try to receive a message without blocking.
    pub fn try_recv(&self) -> Option<PortMessage> {
        // Use try_lock to avoid blocking
        if let Ok(mut guard) = self.receiver.try_lock() {
            guard.try_recv().ok()
        } else {
            None
        }
    }

    /// Async receive a message.
    /// Returns the receiver Arc for use in concurrent waiting.
    pub fn receiver_arc(&self) -> Arc<Mutex<mpsc::Receiver<PortMessage>>> {
        self.receiver.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_message_new() {
        let msg = PortMessage::new("test", serde_json::json!({"key": "value"}));
        assert_eq!(msg.source_id, "test");
        assert_eq!(msg.payload["key"], "value");
    }

    #[test]
    fn test_source0_base() {
        let base = Source0Base::new("test", vec![RunLoopMode::Default]);

        assert_eq!(base.id(), "test");
        assert!(!base.is_signaled());
        assert!(base.is_valid());

        base.signal();
        assert!(base.is_signaled());

        base.clear_signal();
        assert!(!base.is_signaled());

        base.cancel();
        assert!(!base.is_valid());
    }

    #[test]
    fn test_source0_base_modes() {
        let modes = vec![RunLoopMode::Default, RunLoopMode::AgentProcessing];
        let base = Source0Base::new("test", modes.clone());

        assert_eq!(base.modes().len(), 2);
        assert!(base.modes().contains(&RunLoopMode::Default));
        assert!(base.modes().contains(&RunLoopMode::AgentProcessing));
    }
}
