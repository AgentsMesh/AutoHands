use super::*;

#[test]
fn test_completion_request_new() {
    let request = CompletionRequest::new("gpt-4", vec![Message::user("Hello")]);
    assert_eq!(request.model, "gpt-4");
    assert_eq!(request.messages.len(), 1);
    assert!(request.system.is_none());
    assert!(request.tools.is_empty());
    assert!(matches!(request.tool_choice, ToolChoice::Auto));
}

#[test]
fn test_completion_request_with_system() {
    let request = CompletionRequest::new("gpt-4", vec![])
        .with_system("You are helpful");
    assert_eq!(request.system, Some("You are helpful".to_string()));
}

#[test]
fn test_completion_request_with_tools() {
    let tools = vec![
        ToolDefinition::new("read_file", "Read File", "Read a file"),
    ];
    let request = CompletionRequest::new("gpt-4", vec![])
        .with_tools(tools);
    assert_eq!(request.tools.len(), 1);
}

#[test]
fn test_completion_request_with_max_tokens() {
    let request = CompletionRequest::new("gpt-4", vec![])
        .with_max_tokens(1000);
    assert_eq!(request.max_tokens, Some(1000));
}

#[test]
fn test_completion_request_with_temperature() {
    let request = CompletionRequest::new("gpt-4", vec![])
        .with_temperature(0.7);
    assert!((request.temperature.unwrap() - 0.7).abs() < 0.001);
}

#[test]
fn test_completion_request_builder_chain() {
    let request = CompletionRequest::new("claude-3-5-sonnet", vec![])
        .with_system("Be helpful")
        .with_max_tokens(2000)
        .with_temperature(0.5);

    assert_eq!(request.model, "claude-3-5-sonnet");
    assert!(request.system.is_some());
    assert_eq!(request.max_tokens, Some(2000));
}

#[test]
fn test_completion_request_serialization() {
    let request = CompletionRequest::new("gpt-4", vec![Message::user("Test")]);
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("gpt-4"));
    assert!(json.contains("Test"));
}

#[test]
fn test_completion_request_clone() {
    let request = CompletionRequest::new("gpt-4", vec![]);
    let cloned = request.clone();
    assert_eq!(cloned.model, request.model);
}

#[test]
fn test_tool_choice_default() {
    let choice = ToolChoice::default();
    assert!(matches!(choice, ToolChoice::Auto));
}

#[test]
fn test_tool_choice_none() {
    let choice = ToolChoice::None;
    let json = serde_json::to_string(&choice).unwrap();
    assert_eq!(json, "\"none\"");
}

#[test]
fn test_tool_choice_required() {
    let choice = ToolChoice::Required;
    let json = serde_json::to_string(&choice).unwrap();
    assert_eq!(json, "\"required\"");
}

#[test]
fn test_tool_choice_specific_tool() {
    let choice = ToolChoice::Tool { name: "read_file".to_string() };
    let json = serde_json::to_string(&choice).unwrap();
    assert!(json.contains("read_file"));
}

#[test]
fn test_tool_choice_deserialization() {
    let json = "\"auto\"";
    let choice: ToolChoice = serde_json::from_str(json).unwrap();
    assert!(matches!(choice, ToolChoice::Auto));
}
