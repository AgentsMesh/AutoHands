//! RunLoop source management methods (Source0 and Source1).

use std::sync::Arc;

use tracing::{debug, warn};

use crate::error::RunLoopResult;
use crate::mode::RunLoopMode;
use crate::run_loop::{ModeData, RunLoop};
use crate::source::PortMessage;
use crate::task::Task;

impl RunLoop {
    /// Add a Source0 to the specified modes.
    pub async fn add_source0(&self, source: Arc<dyn crate::source::Source0>) {
        for mode in source.modes() {
            if *mode == RunLoopMode::Common {
                // Add to all common modes
                let common = self.common_modes.read().await;
                for m in common.iter() {
                    if let Some(mode_data) = self.modes.get(m) {
                        mode_data.sources0.write().await.push(source.clone());
                    }
                }
            } else if let Some(mode_data) = self.modes.get(mode) {
                mode_data.sources0.write().await.push(source.clone());
            }
        }
    }

    /// Add a Source1 receiver.
    pub async fn add_source1(&self, receiver: crate::source::Source1Receiver) {
        self.source1_receivers.write().await.push(receiver);
    }

    /// Remove a Source0 by ID.
    pub async fn remove_source0(&self, source_id: &str) {
        for mode_data in self.modes.iter() {
            mode_data
                .sources0
                .write()
                .await
                .retain(|s| s.id() != source_id);
        }
    }

    /// Process signaled Source0s.
    pub(crate) async fn process_sources0(&self, mode_data: &ModeData) -> RunLoopResult<Vec<Task>> {
        let mut tasks = Vec::new();

        let sources = mode_data.sources0.read().await;
        for source in sources.iter() {
            if !source.is_valid() {
                continue;
            }
            if source.is_signaled() {
                self.metrics.record_source0_perform();
                match source.perform().await {
                    Ok(source_tasks) => {
                        tasks.extend(source_tasks);
                    }
                    Err(e) => {
                        warn!("Source0 {} perform error: {}", source.id(), e);
                    }
                }
            }
        }

        Ok(tasks)
    }

    /// Try to process Source1 messages (non-blocking).
    pub(crate) async fn try_process_source1(&self) -> RunLoopResult<Option<Vec<Task>>> {
        let receivers = self.source1_receivers.read().await;

        for receiver in receivers.iter() {
            if !receiver.source.is_valid() {
                continue;
            }

            if let Some(msg) = receiver.try_recv() {
                self.metrics.record_source1_message();
                let tasks = receiver.source.handle(msg).await?;
                return Ok(Some(tasks));
            }
        }

        drop(receivers);

        // Clean up invalid sources (need write lock)
        let mut receivers = self.source1_receivers.write().await;
        receivers.retain(|r| r.source.is_valid());

        Ok(None)
    }

    /// Handle a Source1 message.
    pub(crate) async fn handle_source1_message(
        &self,
        source_id: &str,
        message: PortMessage,
    ) -> RunLoopResult<Vec<Task>> {
        let receivers = self.source1_receivers.read().await;

        for receiver in receivers.iter() {
            if receiver.source.id() == source_id && receiver.source.is_valid() {
                return receiver.source.handle(message).await;
            }
        }

        debug!("No receiver found for source: {}", source_id);
        Ok(Vec::new())
    }
}
