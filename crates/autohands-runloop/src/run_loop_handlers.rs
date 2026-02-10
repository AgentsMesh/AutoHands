//! RunLoop observer notification and cleanup methods.

use std::sync::Arc;

use crate::mode::{RunLoopMode, RunLoopPhase};
use crate::run_loop::RunLoop;

impl RunLoop {
    /// Notify observers of a phase.
    pub(crate) async fn notify_observers(&self, phase: RunLoopPhase, mode: &RunLoopMode) {
        // Global observers
        {
            let observers = self.global_observers.read().await;
            for handle in observers.iter() {
                if handle.should_trigger(phase) {
                    self.metrics.record_observer_notification();
                    handle.observer().on_phase(phase, self).await;
                    handle.mark_fired();
                }
            }
        }

        // Mode-specific observers
        if let Some(mode_data) = self.modes.get(mode) {
            let observers = mode_data.observers.read().await;
            for handle in observers.iter() {
                if handle.should_trigger(phase) {
                    self.metrics.record_observer_notification();
                    handle.observer().on_phase(phase, self).await;
                    handle.mark_fired();
                }
            }
        }
    }

    /// Clean up non-repeating observers.
    pub(crate) async fn cleanup_observers(&self, mode: &RunLoopMode) {
        self.global_observers
            .write()
            .await
            .retain(|h| !h.should_remove());

        if let Some(mode_data) = self.modes.get(mode) {
            mode_data
                .observers
                .write()
                .await
                .retain(|h| !h.should_remove());
        }
    }

    /// Add a global observer (notified in all modes).
    pub async fn add_observer(&self, id: impl Into<String>, observer: Arc<dyn crate::observer::RunLoopObserver>) {
        let handle = crate::observer::ObserverHandle::new(id, observer);
        self.global_observers.write().await.push(handle);
        // Sort by priority
        self.global_observers
            .write()
            .await
            .sort_by_key(|h| h.observer().priority());
    }

    /// Add an observer to a specific mode.
    pub async fn add_mode_observer(
        &self,
        mode: &RunLoopMode,
        id: impl Into<String>,
        observer: Arc<dyn crate::observer::RunLoopObserver>,
    ) {
        if let Some(mode_data) = self.modes.get(mode) {
            let handle = crate::observer::ObserverHandle::new(id, observer);
            mode_data.observers.write().await.push(handle);
            mode_data
                .observers
                .write()
                .await
                .sort_by_key(|h| h.observer().priority());
        }
    }

    /// Remove an observer by ID.
    pub async fn remove_observer(&self, id: &str) {
        self.global_observers
            .write()
            .await
            .retain(|h| h.id() != id);
        for mode_data in self.modes.iter() {
            mode_data
                .observers
                .write()
                .await
                .retain(|h| h.id() != id);
        }
    }
}
