//! RunLoop trait implementations (Default, TaskSubmitter).

use crate::config::RunLoopConfig;
use crate::run_loop::RunLoop;

impl Default for RunLoop {
    fn default() -> Self {
        Self::new(RunLoopConfig::default())
    }
}

// Implement TaskSubmitter trait for RunLoop to allow direct task submission
// from extensions and tools without needing a separate adapter.
#[async_trait::async_trait]
impl autohands_protocols::extension::TaskSubmitter for RunLoop {
    async fn submit_task(
        &self,
        task_type: &str,
        payload: serde_json::Value,
        correlation_id: Option<String>,
    ) -> Result<(), autohands_protocols::error::ExtensionError> {
        use crate::task::{Task, TaskPriority, TaskSource};

        // Create Task from parameters
        let mut task = Task::new(task_type.to_string(), payload.clone())
            .with_source(TaskSource::Custom("task_submitter".to_string()));

        // Map priority if present in payload
        if let Some(priority) = payload.get("priority") {
            if let Some(p) = priority.as_str() {
                task = task.with_priority(match p {
                    "low" => TaskPriority::Low,
                    "high" => TaskPriority::High,
                    "critical" => TaskPriority::Critical,
                    _ => TaskPriority::Normal,
                });
            }
        }

        // Copy correlation ID
        if let Some(ref cid) = correlation_id {
            task = task.with_correlation_id(cid.clone());
        }

        // Inject into RunLoop
        self.inject_task(task).await.map_err(|e| {
            autohands_protocols::error::ExtensionError::Custom(format!(
                "Failed to submit task: {}",
                e
            ))
        })
    }
}
