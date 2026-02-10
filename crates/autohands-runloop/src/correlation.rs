//! Correlation guard for scoped correlation ID management.
//!
//! Provides [`CorrelationGuard`] which automatically restores the previous
//! correlation context when dropped, enabling scoped correlation ID tracking
//! across async task boundaries.

use crate::spawner::RunLoopSpawner;

/// Correlation guard for scoped correlation ID.
///
/// Automatically restores the previous correlation context when dropped.
///
/// # Design Note
///
/// Since we can't await in Drop, we use `tokio::task::block_in_place` to
/// synchronously restore the context when running in a multi-threaded runtime.
/// For single-threaded runtimes, we spawn a task to restore the context.
pub struct CorrelationGuard<'a> {
    spawner: &'a RunLoopSpawner,
    /// Previous context to restore. Wrapped in Option<Option<String>> where:
    /// - Some(previous) = need to restore `previous` on drop
    /// - None = already restored via restore(), don't restore again
    previous: Option<Option<String>>,
}

impl<'a> CorrelationGuard<'a> {
    /// Create a new correlation guard.
    ///
    /// Saves the current correlation context and sets a new one.
    /// When the guard is dropped, the previous context is restored.
    pub async fn new(spawner: &'a RunLoopSpawner, correlation_id: String) -> Self {
        let previous = spawner.correlation_context().await;
        spawner
            .set_correlation_context(Some(correlation_id))
            .await;
        Self {
            spawner,
            previous: Some(previous),
        }
    }

    /// Manually restore the previous correlation context.
    ///
    /// Call this method instead of relying on Drop if you want to ensure
    /// the context is restored synchronously in an async context.
    /// This consumes the guard without triggering Drop's restore logic.
    pub async fn restore(mut self) {
        if let Some(previous) = self.previous.take() {
            self.spawner.set_correlation_context(previous).await;
        }
        // When self is dropped, previous is None, so Drop won't overwrite
    }
}

impl<'a> Drop for CorrelationGuard<'a> {
    fn drop(&mut self) {
        // If previous was already taken (via restore()), skip restoration
        let Some(previous) = self.previous.take() else {
            return;
        };

        // Try to use block_in_place for multi-threaded runtime
        // This allows us to synchronously acquire the write lock
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread {
                // Safe to block in multi-threaded runtime
                let context = self.spawner.correlation_context_arc();
                tokio::task::block_in_place(|| {
                    handle.block_on(async {
                        *context.write().await = previous;
                    });
                });
                return;
            }
        }

        // Fallback: spawn a task to restore (best-effort for current_thread runtime)
        // Clone what we need since we can't move spawner
        let context = self.spawner.correlation_context_arc();
        tokio::spawn(async move {
            *context.write().await = previous;
        });
    }
}
