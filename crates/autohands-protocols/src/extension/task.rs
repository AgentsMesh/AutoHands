//! Task submission types for extension communication.
//!
//! Extensions use tasks to communicate with the RunLoop system.
//! The `TaskSubmitter` trait provides a simple, unified interface for
//! submitting tasks that will be processed by the RunLoop.

use async_trait::async_trait;

use crate::error::ExtensionError;

/// Simplified trait for submitting tasks to the RunLoop.
///
/// This provides a single
/// unified interface. All task communication goes through the RunLoop.
#[async_trait]
pub trait TaskSubmitter: Send + Sync {
    /// Submit a task to the RunLoop.
    ///
    /// The task will be queued and processed asynchronously by the RunLoop.
    async fn submit_task(
        &self,
        task_type: &str,
        payload: serde_json::Value,
        correlation_id: Option<String>,
    ) -> Result<(), ExtensionError>;
}
