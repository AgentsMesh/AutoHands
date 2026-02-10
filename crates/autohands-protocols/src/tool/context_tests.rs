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
