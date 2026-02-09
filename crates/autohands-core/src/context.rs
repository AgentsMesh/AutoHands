//! Execution context for tool and agent execution.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use autohands_protocols::tool::AbortSignal;

/// Execution context passed to tools and agents.
#[derive(Clone)]
pub struct ExecutionContext {
    /// Session ID.
    pub session_id: String,

    /// Correlation ID for tracing.
    pub correlation_id: String,

    /// Working directory.
    pub work_dir: PathBuf,

    /// Abort signal for cancellation.
    pub abort_signal: Arc<AbortSignal>,

    /// Parent context (for sub-contexts).
    parent: Option<Arc<ExecutionContext>>,

    /// Context data.
    data: Arc<parking_lot::RwLock<HashMap<String, serde_json::Value>>>,
}

impl ExecutionContext {
    /// Create a new execution context.
    pub fn new(session_id: impl Into<String>, work_dir: PathBuf) -> Self {
        Self {
            session_id: session_id.into(),
            correlation_id: uuid::Uuid::new_v4().to_string(),
            work_dir,
            abort_signal: Arc::new(AbortSignal::new()),
            parent: None,
            data: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    /// Create a sub-context with its own abort signal.
    pub fn sub_context(&self) -> Self {
        Self {
            session_id: self.session_id.clone(),
            correlation_id: uuid::Uuid::new_v4().to_string(),
            work_dir: self.work_dir.clone(),
            abort_signal: Arc::new(AbortSignal::new()),
            parent: Some(Arc::new(self.clone())),
            data: self.data.clone(),
        }
    }

    /// Check if execution should be aborted.
    pub fn is_aborted(&self) -> bool {
        if self.abort_signal.is_aborted() {
            return true;
        }
        if let Some(ref parent) = self.parent {
            return parent.is_aborted();
        }
        false
    }

    /// Trigger abort.
    pub fn abort(&self) {
        self.abort_signal.abort();
    }

    /// Get a value from context data.
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let data = self.data.read();
        data.get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set a value in context data.
    pub fn set<T: serde::Serialize>(&self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            let mut data = self.data.write();
            data.insert(key.into(), v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context_new() {
        let ctx = ExecutionContext::new("session-1", PathBuf::from("/tmp"));
        assert_eq!(ctx.session_id, "session-1");
        assert_eq!(ctx.work_dir, PathBuf::from("/tmp"));
        assert!(!ctx.correlation_id.is_empty());
    }

    #[test]
    fn test_execution_context_sub_context() {
        let ctx = ExecutionContext::new("session-1", PathBuf::from("/tmp"));
        let sub = ctx.sub_context();
        assert_eq!(sub.session_id, "session-1");
        assert_ne!(sub.correlation_id, ctx.correlation_id);
        assert!(sub.parent.is_some());
    }

    #[test]
    fn test_execution_context_abort() {
        let ctx = ExecutionContext::new("session-1", PathBuf::from("/tmp"));
        assert!(!ctx.is_aborted());
        ctx.abort();
        assert!(ctx.is_aborted());
    }

    #[test]
    fn test_execution_context_parent_abort() {
        let ctx = ExecutionContext::new("session-1", PathBuf::from("/tmp"));
        let sub = ctx.sub_context();
        assert!(!sub.is_aborted());
        ctx.abort();
        assert!(sub.is_aborted());
    }

    #[test]
    fn test_execution_context_get_set() {
        let ctx = ExecutionContext::new("session-1", PathBuf::from("/tmp"));
        ctx.set("key", "value");
        let result: Option<String> = ctx.get("key");
        assert_eq!(result, Some("value".to_string()));
    }

    #[test]
    fn test_execution_context_get_missing() {
        let ctx = ExecutionContext::new("session-1", PathBuf::from("/tmp"));
        let result: Option<String> = ctx.get("missing");
        assert!(result.is_none());
    }

    #[test]
    fn test_execution_context_set_complex_value() {
        let ctx = ExecutionContext::new("session-1", PathBuf::from("/tmp"));
        ctx.set("count", 42i32);
        let result: Option<i32> = ctx.get("count");
        assert_eq!(result, Some(42));
    }

    #[test]
    fn test_execution_context_shared_data() {
        let ctx = ExecutionContext::new("session-1", PathBuf::from("/tmp"));
        let sub = ctx.sub_context();
        ctx.set("shared", "data");
        let result: Option<String> = sub.get("shared");
        assert_eq!(result, Some("data".to_string()));
    }

    #[test]
    fn test_execution_context_clone() {
        let ctx = ExecutionContext::new("session-1", PathBuf::from("/tmp"));
        let cloned = ctx.clone();
        assert_eq!(cloned.session_id, ctx.session_id);
        assert_eq!(cloned.correlation_id, ctx.correlation_id);
    }
}
