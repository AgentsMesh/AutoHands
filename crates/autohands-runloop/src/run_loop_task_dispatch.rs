//! RunLoop task dispatch and processing logic.

use tracing::{debug, error, info, warn};

use autohands_protocols::channel::OutboundMessage;

use crate::agent_source::AgentTaskInjector;
use crate::error::RunLoopResult;
use crate::run_loop::RunLoop;
use crate::task::Task;

impl RunLoop {
    /// Process a task using the configured handler.
    ///
    /// This is the core task processing logic:
    /// 1. Check if handler is configured
    /// 2. Execute the task via handler
    /// 3. Send response back via channel if reply_to is set
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

        // Create an injector with direct queue access
        // Follow-up tasks from AgentResult.tasks will be enqueued after processing
        let injector = AgentTaskInjector::with_queue(self.task_queue.clone());

        // Route task to appropriate handler method based on task type
        let result = match task.task_type.as_str() {
            "agent:execute" => {
                info!(
                    "Executing agent task: task_id={}, correlation_id={:?}",
                    task.id, task.correlation_id
                );
                handler.handle_execute(&task, &injector).await
            }
            "agent:subtask" => {
                debug!(
                    "Executing subtask: task_id={}, correlation_id={:?}",
                    task.id, task.correlation_id
                );
                handler.handle_subtask(&task, &injector).await
            }
            "agent:delayed" => {
                debug!(
                    "Executing delayed task: task_id={}, correlation_id={:?}",
                    task.id, task.correlation_id
                );
                handler.handle_delayed(&task, &injector).await
            }
            _ => {
                debug!("Unknown task type: {}, ignoring", task.task_type);
                return Ok(());
            }
        };

        match result {
            Ok(agent_result) => {
                self.handle_agent_result(&task, agent_result).await
            }
            Err(e) => {
                error!("Task execution failed: task_id={}, error={}", task.id, e);
                Err(e)
            }
        }
    }

    /// Handle a successful agent result: inject follow-up tasks, send response.
    async fn handle_agent_result(
        &self,
        task: &Task,
        agent_result: crate::agent_driver::AgentResult,
    ) -> RunLoopResult<()> {
        // Inject follow-up tasks
        if !agent_result.tasks.is_empty() {
            info!("Injecting {} follow-up tasks", agent_result.tasks.len());
            for follow_up in agent_result.tasks {
                self.task_queue.enqueue(follow_up).await?;
            }
        }

        // Send response back via channel if reply_to is set
        if let Some(ref reply_to) = task.reply_to {
            if let Some(ref response) = agent_result.response {
                let channel_guard = self.channel_registry.read().await;
                if let Some(ref registry) = *channel_guard {
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

        if agent_result.is_complete {
            info!(
                "Task completed: task_id={}, has_response={}",
                task.id,
                agent_result.response.is_some()
            );
        }

        Ok(())
    }
}
