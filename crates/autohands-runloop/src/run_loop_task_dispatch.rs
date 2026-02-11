//! RunLoop task dispatch and processing logic.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use chrono::{Duration as ChronoDuration, Utc};
use cron::Schedule;
use futures::FutureExt;
use tracing::{debug, error, info, warn};

use autohands_protocols::channel::OutboundMessage;

use crate::agent_source::AgentTaskInjector;
use crate::error::RunLoopResult;
use crate::run_loop::RunLoop;
use crate::task::{Task, TaskSource};

impl RunLoop {
    /// Process a task using the configured handler.
    ///
    /// Agent-class long-running tasks (`agent:*`, `trigger:*`, unknown fallback)
    /// are dispatched via `tokio::spawn` so they do not block the RunLoop event
    /// loop. Timer and system tasks are handled synchronously (they are cheap).
    pub(crate) async fn process_task(&self, task: Task) -> RunLoopResult<()> {
        let handler_guard = self.handler.read().await;
        let handler = match handler_guard.as_ref() {
            Some(h) => h.clone(),
            None => {
                warn!("No handler configured, task {} ignored", task.id);
                return Ok(());
            }
        };
        drop(handler_guard); // Release lock before async operation

        // Route task to appropriate handler method based on task type
        match task.task_type.as_str() {
            t if t.starts_with("timer:") || t.starts_with("system:") => {
                debug!(
                    "Processing timer/system task: task_id={}, type={}",
                    task.id, task.task_type
                );
                // If the task is a repeating timer, reschedule before returning
                if task.metadata.get("timer_repeat")
                    == Some(&serde_json::Value::Bool(true))
                {
                    self.reschedule_repeating_timer(&task).await?;
                }
                Ok(())
            }
            t if t.starts_with("cron:") => {
                debug!(
                    "Processing cron task: task_id={}, type={}",
                    task.id, task.task_type
                );
                // Reschedule the next cron occurrence
                self.reschedule_cron_timer(&task).await?;
                Ok(())
            }
            // Agent-class tasks: spawn into background to avoid blocking the RunLoop
            _ => {
                self.spawn_agent_task(handler, task);
                Ok(())
            }
        }
    }

    /// Spawn an agent-class task in the background via `tokio::spawn`.
    ///
    /// The RunLoop event loop continues processing timers, sources, and new tasks
    /// while the agent executes (which can take minutes).
    fn spawn_agent_task(
        &self,
        handler: Arc<dyn crate::agent_driver::AgentEventHandler>,
        task: Task,
    ) {
        let task_queue = self.task_queue.clone();
        // Clone the Arc<RwLock<...>> so we can read().await inside the spawn closure
        let channel_registry_lock = self.channel_registry.clone();

        let task_id = task.id;
        let task_type = task.task_type.clone();

        info!(
            "Spawning agent task: task_id={}, type={}, correlation_id={:?}",
            task_id, task_type, task.correlation_id
        );

        tokio::spawn(async move {
            let result = AssertUnwindSafe(async {
                // Acquire channel registry inside the spawn to guarantee read access
                let channel_registry = channel_registry_lock.read().await.clone();

                // Create an injector with direct queue access
                let injector = AgentTaskInjector::with_queue(task_queue.clone());

                let result = match task.task_type.as_str() {
                    "agent:execute" => handler.handle_execute(&task, &injector).await,
                    "agent:subtask" => handler.handle_subtask(&task, &injector).await,
                    "agent:delayed" => handler.handle_delayed(&task, &injector).await,
                    t if t.starts_with("trigger:") => handler.handle_execute(&task, &injector).await,
                    _ => {
                        warn!(
                            "Unknown task type: {}, attempting handle_execute as fallback",
                            task.task_type
                        );
                        handler.handle_execute(&task, &injector).await
                    }
                };

                match result {
                    Ok(agent_result) => {
                        if let Err(e) =
                            Self::handle_agent_result_static(&task, agent_result, &task_queue, channel_registry.as_ref()).await
                        {
                            error!("Failed to handle agent result: task_id={}, error={}", task_id, e);
                        }
                    }
                    Err(e) => {
                        error!("Task execution failed: task_id={}, error={}", task_id, e);
                    }
                }
            })
            .catch_unwind()
            .await;

            if let Err(panic_info) = result {
                let msg = panic_info
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| panic_info.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".to_string());
                error!("Agent task panicked: task_id={}, panic={}", task_id, msg);
            }
        });
    }

    /// Handle a successful agent result (static version for use inside `tokio::spawn`).
    ///
    /// Injects follow-up tasks, sends response via channel, and resets the task chain.
    async fn handle_agent_result_static(
        task: &Task,
        agent_result: crate::agent_driver::AgentResult,
        task_queue: &Arc<crate::task_queue::TaskQueue>,
        channel_registry: Option<&Arc<autohands_core::registry::ChannelRegistry>>,
    ) -> RunLoopResult<()> {
        // Inject follow-up tasks
        if !agent_result.tasks.is_empty() {
            info!("Injecting {} follow-up tasks", agent_result.tasks.len());
            for follow_up in agent_result.tasks {
                task_queue.enqueue(follow_up).await?;
            }
        }

        // Send response back via channel if reply_to is set
        if let Some(ref reply_to) = task.reply_to {
            if let Some(ref response) = agent_result.response {
                if let Some(registry) = channel_registry {
                    let outbound = OutboundMessage::text(response);
                    match registry.send(reply_to, outbound).await {
                        Ok(_) => {
                            info!(
                                "Response sent to channel: {} target: {}",
                                reply_to.channel_id, reply_to.target
                            );
                        }
                        Err(e) => {
                            error!(
                                "Failed to send response to channel {}: {}",
                                reply_to.channel_id, e
                            );
                        }
                    }
                } else {
                    warn!("No channel registry configured, cannot send response");
                }
            }
        }

        // Reset task chain when the task completes (#7A)
        if let Some(ref correlation_id) = task.correlation_id {
            task_queue.chain_tracker().reset_chain(correlation_id);
            debug!("Task chain reset: correlation_id={}", correlation_id);
        }

        if agent_result.is_complete {
            info!(
                "Task completed: task_id={}, has_response={}",
                task.id,
                agent_result.response.is_some()
            );
        }

        Ok(())
    }

    /// Reschedule a repeating timer task.
    ///
    /// Reads `timer_interval_ms` from the task's metadata, creates a new Task
    /// with the same type/payload/metadata and a `scheduled_at` offset, then enqueues it.
    async fn reschedule_repeating_timer(&self, task: &Task) -> RunLoopResult<()> {
        let interval_ms = task
            .metadata
            .get("timer_interval_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(60_000); // Default to 60s if not specified

        let next_scheduled = Utc::now() + ChronoDuration::milliseconds(interval_ms as i64);

        let new_task = Task::new(task.task_type.clone(), task.payload.clone())
            .with_source(TaskSource::Timer)
            .with_scheduled_at(next_scheduled);

        // Copy over timer metadata to the new task
        let mut new_task = new_task;
        for (key, value) in &task.metadata {
            new_task.metadata.insert(key.clone(), value.clone());
        }

        info!(
            "Rescheduling repeating timer: type={}, interval={}ms, next_at={}",
            task.task_type, interval_ms, next_scheduled
        );

        self.task_queue.enqueue(new_task).await?;
        Ok(())
    }

    /// Reschedule a cron timer task.
    ///
    /// Reads `cron_timer_expr` from the task's metadata, computes the next
    /// fire time via `cron::Schedule`, and enqueues a new task at that time.
    async fn reschedule_cron_timer(&self, task: &Task) -> RunLoopResult<()> {
        let cron_expr = match task
            .metadata
            .get("cron_timer_expr")
            .and_then(|v| v.as_str())
        {
            Some(expr) => expr.to_string(),
            None => {
                warn!(
                    "Cron task {} has no cron_timer_expr in metadata, skipping reschedule",
                    task.id
                );
                return Ok(());
            }
        };

        let schedule: Schedule = match cron_expr.parse() {
            Ok(s) => s,
            Err(e) => {
                error!(
                    "Failed to parse cron expression '{}' for task {}: {}",
                    cron_expr, task.id, e
                );
                return Ok(());
            }
        };

        let next_time = match schedule.upcoming(Utc).next() {
            Some(t) => t,
            None => {
                debug!("Cron schedule '{}' has no upcoming times", cron_expr);
                return Ok(());
            }
        };

        let mut new_task = Task::new(task.task_type.clone(), task.payload.clone())
            .with_source(TaskSource::Scheduler)
            .with_scheduled_at(next_time);

        // Copy over cron metadata to the new task
        for (key, value) in &task.metadata {
            new_task.metadata.insert(key.clone(), value.clone());
        }

        info!(
            "Rescheduling cron timer: type={}, expr='{}', next_at={}",
            task.task_type, cron_expr, next_time
        );

        self.task_queue.enqueue(new_task).await?;
        Ok(())
    }
}
