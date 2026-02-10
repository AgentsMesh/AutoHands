//! RunLoop observer definitions.
//!
//! Observers are notified at specific phases of the RunLoop,
//! similar to CFRunLoopObserver in iOS.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;

use crate::mode::RunLoopPhase;
use crate::RunLoop;

#[path = "observer_builtin.rs"]
mod observer_builtin;
#[path = "spawner_observer.rs"]
mod spawner_observer;

pub use observer_builtin::*;
pub use spawner_observer::SpawnerObserver;

#[cfg(test)]
#[path = "observer_tests.rs"]
mod tests;

/// RunLoop observer trait.
///
/// Similar to CFRunLoopObserver in iOS.
/// Observers are triggered at specific phases of the RunLoop.
#[async_trait]
pub trait RunLoopObserver: Send + Sync {
    /// Get the activity mask (which phases to observe).
    /// Use RunLoopPhase::ALL to observe all phases.
    fn activities(&self) -> u32;

    /// Whether the observer repeats (false = triggered once then removed).
    fn repeats(&self) -> bool {
        true
    }

    /// Observer priority (lower = executed first).
    fn priority(&self) -> i32 {
        0
    }

    /// Called when the observed phase is triggered.
    async fn on_phase(&self, phase: RunLoopPhase, run_loop: &RunLoop);
}

/// Observer registration handle.
pub struct ObserverHandle {
    id: String,
    observer: Arc<dyn RunLoopObserver>,
    fired: AtomicBool,
}

impl ObserverHandle {
    /// Create a new observer handle.
    pub fn new(id: impl Into<String>, observer: Arc<dyn RunLoopObserver>) -> Self {
        Self {
            id: id.into(),
            observer,
            fired: AtomicBool::new(false),
        }
    }

    /// Get the observer ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the observer.
    pub fn observer(&self) -> &Arc<dyn RunLoopObserver> {
        &self.observer
    }

    /// Check if this observer should be triggered for the given phase.
    pub fn should_trigger(&self, phase: RunLoopPhase) -> bool {
        if self.fired.load(Ordering::SeqCst) && !self.observer.repeats() {
            return false;
        }
        phase.matches(self.observer.activities())
    }

    /// Mark as fired.
    pub fn mark_fired(&self) {
        self.fired.store(true, Ordering::SeqCst);
    }

    /// Check if should be removed (fired and non-repeating).
    pub fn should_remove(&self) -> bool {
        self.fired.load(Ordering::SeqCst) && !self.observer.repeats()
    }
}
