use super::*;

#[test]
fn test_tool_result_success() {
    let result = ToolResult::success("OK");
    assert!(result.success);
    assert_eq!(result.content, "OK");
    assert!(result.error.is_none());
    assert!(result.structured_output.is_none());
}

#[test]
fn test_tool_result_success_json() {
    let output = serde_json::json!({"key": "value"});
    let result = ToolResult::success_json("OK", output);
    assert!(result.success);
    assert!(result.structured_output.is_some());
    assert_eq!(result.structured_output.as_ref().unwrap()["key"], "value");
}

#[test]
fn test_tool_result_error() {
    let result = ToolResult::error("Something went wrong");
    assert!(!result.success);
    assert!(result.content.is_empty());
    assert_eq!(result.error, Some("Something went wrong".to_string()));
}

#[test]
fn test_tool_result_with_metadata() {
    let result = ToolResult::success("OK")
        .with_metadata("duration", serde_json::json!(100));
    assert_eq!(result.metadata.get("duration").unwrap(), &serde_json::json!(100));
}

#[test]
fn test_tool_result_serialization() {
    let result = ToolResult::success("OK");
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("OK"));
    assert!(json.contains("true"));
}

#[test]
fn test_tool_result_chunk() {
    let chunk = ToolResultChunk {
        content: "partial".to_string(),
        is_final: false,
        error: None,
    };
    assert_eq!(chunk.content, "partial");
    assert!(!chunk.is_final);
}

#[test]
fn test_tool_result_chunk_final() {
    let chunk = ToolResultChunk {
        content: "done".to_string(),
        is_final: true,
        error: None,
    };
    assert!(chunk.is_final);
}

#[test]
fn test_tool_result_chunk_with_error() {
    let chunk = ToolResultChunk {
        content: String::new(),
        is_final: true,
        error: Some("Error occurred".to_string()),
    };
    assert!(chunk.error.is_some());
}

#[test]
fn test_tool_result_chunk_serialization() {
    let chunk = ToolResultChunk {
        content: "data".to_string(),
        is_final: false,
        error: None,
    };
    let json = serde_json::to_string(&chunk).unwrap();
    assert!(json.contains("data"));
    assert!(json.contains("false"));
}

#[test]
fn test_tool_result_clone() {
    let result = ToolResult::success("test");
    let cloned = result.clone();
    assert_eq!(cloned.success, result.success);
    assert_eq!(cloned.content, result.content);
}

#[test]
fn test_tool_result_debug() {
    let result = ToolResult::success("debug test");
    let debug = format!("{:?}", result);
    assert!(debug.contains("ToolResult"));
    assert!(debug.contains("debug test"));
}

#[test]
fn test_tool_result_deserialization() {
    let json = r#"{"success":true,"content":"OK"}"#;
    let result: ToolResult = serde_json::from_str(json).unwrap();
    assert!(result.success);
    assert_eq!(result.content, "OK");
}

#[test]
fn test_tool_result_multiple_metadata() {
    let result = ToolResult::success("OK")
        .with_metadata("key1", serde_json::json!("value1"))
        .with_metadata("key2", serde_json::json!(42));
    assert_eq!(result.metadata.len(), 2);
    assert_eq!(result.metadata.get("key1").unwrap(), "value1");
    assert_eq!(result.metadata.get("key2").unwrap(), 42);
}

#[test]
fn test_tool_result_json_complex() {
    let output = serde_json::json!({
        "items": [1, 2, 3],
        "nested": {"a": "b"}
    });
    let result = ToolResult::success_json("Complex output", output);
    assert!(result.structured_output.is_some());
    let so = result.structured_output.unwrap();
    assert!(so["items"].is_array());
    assert!(so["nested"].is_object());
}

#[test]
fn test_tool_result_chunk_clone() {
    let chunk = ToolResultChunk {
        content: "content".to_string(),
        is_final: true,
        error: None,
    };
    let cloned = chunk.clone();
    assert_eq!(cloned.content, "content");
    assert!(cloned.is_final);
}

#[test]
fn test_tool_result_chunk_debug() {
    let chunk = ToolResultChunk {
        content: "data".to_string(),
        is_final: false,
        error: Some("err".to_string()),
    };
    let debug = format!("{:?}", chunk);
    assert!(debug.contains("ToolResultChunk"));
}

#[test]
fn test_tool_result_chunk_deserialization() {
    let json = r#"{"content":"test","is_final":true}"#;
    let chunk: ToolResultChunk = serde_json::from_str(json).unwrap();
    assert_eq!(chunk.content, "test");
    assert!(chunk.is_final);
}

#[test]
fn test_tool_result_error_with_empty_message() {
    let result = ToolResult::error("");
    assert!(!result.success);
    assert_eq!(result.error, Some(String::new()));
}

#[test]
fn test_tool_result_success_with_empty_content() {
    let result = ToolResult::success("");
    assert!(result.success);
    assert!(result.content.is_empty());
}

#[test]
fn test_tool_result_serialization_skips_none() {
    let result = ToolResult::success("OK");
    let json = serde_json::to_string(&result).unwrap();
    // structured_output and error should not be present when None
    assert!(!json.contains("structured_output"));
    assert!(!json.contains("error"));
}
