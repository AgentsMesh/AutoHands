use super::*;

#[test]
fn test_request_serialization() {
    let req = McpRequest::new(1i64, "initialize")
        .with_params(serde_json::json!({"capabilities": {}}));

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("jsonrpc"));
    assert!(json.contains("initialize"));
}

#[test]
fn test_response_success() {
    let resp = McpResponse::success(1i64, serde_json::json!({"tools": []}));
    assert!(!resp.is_error());
    assert!(resp.result.is_some());
}

#[test]
fn test_response_error() {
    let resp = McpResponse::error(1i64, McpError::method_not_found());
    assert!(resp.is_error());
    assert_eq!(resp.error.as_ref().unwrap().code, -32601);
}

#[test]
fn test_request_id_variants() {
    let id1: RequestId = 42i64.into();
    let id2: RequestId = "abc".into();

    assert_eq!(id1, RequestId::Number(42));
    assert_eq!(id2, RequestId::String("abc".to_string()));
}

#[test]
fn test_mcp_method() {
    assert_eq!(McpMethod::Initialize.as_str(), "initialize");
    assert_eq!(McpMethod::ListTools.as_str(), "tools/list");
    assert_eq!(McpMethod::CallTool.as_str(), "tools/call");
}

#[test]
fn test_mcp_error_codes() {
    assert_eq!(McpError::parse_error().code, -32700);
    assert_eq!(McpError::invalid_request().code, -32600);
    assert_eq!(McpError::method_not_found().code, -32601);
    assert_eq!(McpError::invalid_params().code, -32602);
    assert_eq!(McpError::internal_error().code, -32603);
}

#[test]
fn test_tool_definition_deserialization() {
    let json = r#"{
        "name": "test_tool",
        "description": "A test tool",
        "inputSchema": {"type": "object", "properties": {}}
    }"#;

    let tool: McpToolDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(tool.name, "test_tool");
    assert_eq!(tool.description, Some("A test tool".to_string()));
}

#[test]
fn test_mcp_content() {
    let text = McpContent::Text { text: "Hello".to_string() };
    let json = serde_json::to_string(&text).unwrap();
    assert!(json.contains("text"));
}

#[test]
fn test_request_id_from_string() {
    let id: RequestId = String::from("request-123").into();
    assert_eq!(id, RequestId::String("request-123".to_string()));
}

#[test]
fn test_request_without_params() {
    let req = McpRequest::new(1i64, "tools/list");
    let json = serde_json::to_string(&req).unwrap();
    assert!(!json.contains("params")); // Skipped when None
}

#[test]
fn test_response_serialization() {
    let resp = McpResponse::success(1i64, serde_json::json!("result"));
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("result"));
    assert!(!json.contains("error")); // Skipped when None
}

#[test]
fn test_mcp_error_with_data() {
    let mut err = McpError::new(-1, "Custom error");
    err.data = Some(serde_json::json!({"details": "extra info"}));
    let json = serde_json::to_string(&err).unwrap();
    assert!(json.contains("extra info"));
}

#[test]
fn test_mcp_method_resources() {
    assert_eq!(McpMethod::ListResources.as_str(), "resources/list");
    assert_eq!(McpMethod::ReadResource.as_str(), "resources/read");
}

#[test]
fn test_mcp_method_prompts() {
    assert_eq!(McpMethod::ListPrompts.as_str(), "prompts/list");
    assert_eq!(McpMethod::GetPrompt.as_str(), "prompts/get");
}

#[test]
fn test_mcp_content_image() {
    let img = McpContent::Image {
        data: "base64data".to_string(),
        mime_type: "image/png".to_string(),
    };
    let json = serde_json::to_string(&img).unwrap();
    assert!(json.contains("image"));
    assert!(json.contains("base64data"));
}

#[test]
fn test_mcp_content_resource() {
    let res = McpContent::Resource {
        uri: "file:///path/to/file".to_string(),
        mime_type: Some("text/plain".to_string()),
    };
    let json = serde_json::to_string(&res).unwrap();
    assert!(json.contains("resource"));
    assert!(json.contains("file:///path/to/file"));
}

#[test]
fn test_mcp_tool_result() {
    let result = McpToolResult {
        content: vec![McpContent::Text { text: "Result".to_string() }],
        is_error: false,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("content"));
    assert!(json.contains("Result"));
}

#[test]
fn test_mcp_tool_result_error() {
    let result = McpToolResult {
        content: vec![McpContent::Text { text: "Error message".to_string() }],
        is_error: true,
    };
    assert!(result.is_error);
}

#[test]
fn test_request_clone() {
    let req = McpRequest::new(1i64, "test");
    let cloned = req.clone();
    assert_eq!(cloned.method, req.method);
}

#[test]
fn test_response_clone() {
    let resp = McpResponse::success(1i64, serde_json::json!({}));
    let cloned = resp.clone();
    assert!(!cloned.is_error());
}

#[test]
fn test_mcp_error_clone() {
    let err = McpError::internal_error();
    let cloned = err.clone();
    assert_eq!(cloned.code, err.code);
}

#[test]
fn test_tool_definition_no_description() {
    let json = r#"{
        "name": "simple_tool",
        "inputSchema": {"type": "object"}
    }"#;
    let tool: McpToolDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(tool.name, "simple_tool");
    assert!(tool.description.is_none());
}

#[test]
fn test_request_id_debug() {
    let id = RequestId::Number(42);
    let debug = format!("{:?}", id);
    assert!(debug.contains("42"));
}

#[test]
fn test_request_id_eq() {
    let id1 = RequestId::Number(1);
    let id2 = RequestId::Number(1);
    let id3 = RequestId::Number(2);
    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}
