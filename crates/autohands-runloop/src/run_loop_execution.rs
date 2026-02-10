//! RunLoop event loop execution (`run` and `run_in_mode`).

use std::time::{Duration, Instant};

use tracing::{debug, error, info};

use crate::error::{RunLoopError, RunLoopResult};
use crate::mode::{RunLoopMode, RunLoopPhase, RunLoopRunResult, RunLoopState};
use crate::run_loop::{RunLoop, WakeupSignal};

impl RunLoop {
    /// Run the RunLoop (blocking until stopped).
    pub async fn run(&self) -> RunLoopResult<()> {
        self.run_in_mode(RunLoopMode::Default, Duration::MAX).await?;
        Ok(())
    }

    /// Run the RunLoop in a specific mode.
    ///
    /// Returns when stopped, timed out, or error.
    pub async fn run_in_mode(
        &self,
        mode: RunLoopMode,
        timeout: Duration,
    ) -> RunLoopResult<RunLoopRunResult> {
        let deadline = Instant::now() + timeout;

        *self.current_mode.write().await = mode.clone();
        self.set_state(RunLoopState::Running);
        self.metrics.mark_start();

        let mode_data = self
            .modes
            .get(&mode)
            .ok_or(RunLoopError::ModeNotFound(mode.clone()))?;

        debug!("RunLoop: Entry");
        self.notify_observers(RunLoopPhase::Entry, &mode).await;

        loop {
            self.metrics.record_iteration();

            if self.state() == RunLoopState::Stopping {
                break;
            }
            if Instant::now() >= deadline {
                self.notify_observers(RunLoopPhase::Exit, &mode).await;
                return Ok(RunLoopRunResult::TimedOut);
            }

            let process_start = Instant::now();

            debug!("RunLoop: BeforeTimers");
            self.notify_observers(RunLoopPhase::BeforeTimers, &mode).await;
            self.task_queue.promote_delayed().await;

            debug!("RunLoop: BeforeSources");
            self.notify_observers(RunLoopPhase::BeforeSources, &mode).await;
            let source0_tasks = self.process_sources0(&mode_data).await?;
            for task in source0_tasks {
                self.task_queue.enqueue(task).await?;
            }

            if let Some(tasks) = self.try_process_source1().await? {
                for task in tasks {
                    self.task_queue.enqueue(task).await?;
                }
                continue;
            }

            if let Some(task) = self.task_queue.dequeue().await {
                info!("Processing task: {} (type: {})", task.id, task.task_type);
                self.metrics.record_events_processed(1);
                if let Err(e) = self.process_task(task).await {
                    error!("Task processing error: {}", e);
                }
                continue;
            }

            self.metrics
                .record_process_time(process_start.elapsed().as_micros() as u64);

            debug!("RunLoop: BeforeWaiting");
            self.notify_observers(RunLoopPhase::BeforeWaiting, &mode).await;
            self.set_state(RunLoopState::Waiting);
            self.cleanup_observers(&mode).await;

            let wait_start = Instant::now();
            let wakeup = self.wait_for_wakeup(deadline).await;
            self.metrics
                .record_wait_time(wait_start.elapsed().as_micros() as u64);

            self.set_state(RunLoopState::Running);
            debug!("RunLoop: AfterWaiting (wakeup: {:?})", wakeup);
            self.notify_observers(RunLoopPhase::AfterWaiting, &mode).await;

            match wakeup {
                WakeupSignal::Stop => break,
                WakeupSignal::SourceReady { source_id, message } => {
                    debug!("Source1 ready: {}", source_id);
                    let tasks = self.handle_source1_message(&source_id, message).await?;
                    for task in tasks {
                        self.task_queue.enqueue(task).await?;
                    }
                }
                WakeupSignal::Explicit { reason } => {
                    debug!("Explicit wakeup: {}", reason);
                }
            }
        }

        self.set_state(RunLoopState::Stopping);
        debug!("RunLoop: Exit");
        self.notify_observers(RunLoopPhase::Exit, &mode).await;
        self.set_state(RunLoopState::Stopped);

        info!("RunLoop stopped");
        Ok(RunLoopRunResult::Stopped)
    }
}
