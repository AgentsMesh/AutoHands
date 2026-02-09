//! Tool execution context.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::extension::TaskSubmitter;

/// Context for tool execution.
#[derive(Clone)]
pub struct ToolContext {
    /// Session ID for the current session.
    pub session_id: String,

    /// Correlation ID for tracing.
    pub correlation_id: String,

    /// Working directory for file operations.
    pub work_dir: std::path::PathBuf,

    /// Abort signal for cancellation.
    pub abort_signal: Arc<AbortSignal>,

    /// Task submitter for publishing tasks to RunLoop.
    pub task_submitter: Option<Arc<dyn TaskSubmitter>>,

    /// Additional context data.
    pub data: HashMap<String, serde_json::Value>,
}

impl ToolContext {
    /// Create a new tool context.
    pub fn new(session_id: impl Into<String>, work_dir: std::path::PathBuf) -> Self {
        Self {
            session_id: session_id.into(),
            correlation_id: uuid::Uuid::new_v4().to_string(),
            work_dir,
            abort_signal: Arc::new(AbortSignal::new()),
            task_submitter: None,
            data: HashMap::new(),
        }
    }

    /// Check if the operation should be aborted.
    pub fn is_aborted(&self) -> bool {
        self.abort_signal.is_aborted()
    }

    /// Get a value from the context data.
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set a value in the context data.
    pub fn set<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.into(), v);
        }
    }
}

/// Signal for aborting operations.
pub struct AbortSignal {
    aborted: std::sync::atomic::AtomicBool,
}

impl AbortSignal {
    /// Create a new abort signal.
    pub fn new() -> Self {
        Self {
            aborted: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Check if aborted.
    pub fn is_aborted(&self) -> bool {
        self.aborted.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Trigger the abort.
    pub fn abort(&self) {
        self.aborted
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Default for AbortSignal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_tool_context_new() {
        let ctx = ToolContext::new("session-1", PathBuf::from("/tmp"));
        assert_eq!(ctx.session_id, "session-1");
        assert_eq!(ctx.work_dir, PathBuf::from("/tmp"));
        assert!(!ctx.correlation_id.is_empty());
        assert!(ctx.task_submitter.is_none());
        assert!(ctx.data.is_empty());
    }

    #[test]
    fn test_tool_context_is_aborted() {
        let ctx = ToolContext::new("session-1", PathBuf::from("/tmp"));
        assert!(!ctx.is_aborted());
    }

    #[test]
    fn test_tool_context_abort() {
        let ctx = ToolContext::new("session-1", PathBuf::from("/tmp"));
        assert!(!ctx.is_aborted());
        ctx.abort_signal.abort();
        assert!(ctx.is_aborted());
    }

    #[test]
    fn test_tool_context_get_set() {
        let mut ctx = ToolContext::new("session-1", PathBuf::from("/tmp"));
        ctx.set("key", "value");
        let result: Option<String> = ctx.get("key");
        assert_eq!(result, Some("value".to_string()));
    }

    #[test]
    fn test_tool_context_get_missing() {
        let ctx = ToolContext::new("session-1", PathBuf::from("/tmp"));
        let result: Option<String> = ctx.get("missing");
        assert!(result.is_none());
    }

    #[test]
    fn test_tool_context_set_number() {
        let mut ctx = ToolContext::new("session-1", PathBuf::from("/tmp"));
        ctx.set("count", 42i32);
        let result: Option<i32> = ctx.get("count");
        assert_eq!(result, Some(42));
    }

    #[test]
    fn test_tool_context_set_complex_value() {
        let mut ctx = ToolContext::new("session-1", PathBuf::from("/tmp"));
        let data = serde_json::json!({"nested": {"key": "value"}});
        ctx.set("complex", data.clone());
        let result: Option<serde_json::Value> = ctx.get("complex");
        assert_eq!(result, Some(data));
    }

    #[test]
    fn test_tool_context_clone() {
        let ctx = ToolContext::new("session-1", PathBuf::from("/tmp"));
        let cloned = ctx.clone();
        assert_eq!(cloned.session_id, ctx.session_id);
        assert_eq!(cloned.correlation_id, ctx.correlation_id);
        assert_eq!(cloned.work_dir, ctx.work_dir);
    }

    #[test]
    fn test_abort_signal_new() {
        let signal = AbortSignal::new();
        assert!(!signal.is_aborted());
    }

    #[test]
    fn test_abort_signal_abort() {
        let signal = AbortSignal::new();
        assert!(!signal.is_aborted());
        signal.abort();
        assert!(signal.is_aborted());
    }

    #[test]
    fn test_abort_signal_default() {
        let signal = AbortSignal::default();
        assert!(!signal.is_aborted());
    }

    #[test]
    fn test_abort_signal_multiple_aborts() {
        let signal = AbortSignal::new();
        signal.abort();
        signal.abort(); // Should be idempotent
        assert!(signal.is_aborted());
    }

    #[test]
    fn test_tool_context_shared_abort_signal() {
        let ctx = ToolContext::new("session-1", PathBuf::from("/tmp"));
        let signal = ctx.abort_signal.clone();
        assert!(!ctx.is_aborted());
        signal.abort();
        assert!(ctx.is_aborted());
    }

    #[test]
    fn test_tool_context_correlation_id_unique() {
        let ctx1 = ToolContext::new("session-1", PathBuf::from("/tmp"));
        let ctx2 = ToolContext::new("session-1", PathBuf::from("/tmp"));
        assert_ne!(ctx1.correlation_id, ctx2.correlation_id);
    }
}
