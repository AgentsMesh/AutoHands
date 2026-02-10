use super::*;

fn create_test_config() -> SseTransportConfig {
    SseTransportConfig {
        sse_url: "http://localhost:8080/sse".to_string(),
        http_url: "http://localhost:8080/mcp".to_string(),
        timeout_seconds: 30,
        authorization: None,
        reconnect_delay_ms: 1000,
    }
}

#[test]
fn test_sse_config_defaults() {
    let json = serde_json::json!({
        "sse_url": "http://localhost/sse",
        "http_url": "http://localhost/mcp"
    });
    let config: SseTransportConfig = serde_json::from_value(json).unwrap();
    assert_eq!(config.timeout_seconds, 30);
    assert_eq!(config.reconnect_delay_ms, 1000);
}

#[test]
fn test_sse_config_serialization() {
    let config = create_test_config();
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("sse_url"));
    assert!(json.contains("http_url"));
}

#[test]
fn test_sse_config_with_all_fields() {
    let config = SseTransportConfig {
        sse_url: "https://api.example.com/sse".to_string(),
        http_url: "https://api.example.com/mcp".to_string(),
        timeout_seconds: 60,
        authorization: Some("Bearer token123".to_string()),
        reconnect_delay_ms: 2000,
    };
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("https://api.example.com/sse"));
    assert!(json.contains("60"));
    assert!(json.contains("Bearer token123"));
    assert!(json.contains("2000"));
}

#[test]
fn test_sse_config_deserialization() {
    let json = r#"{
        "sse_url": "http://test/sse",
        "http_url": "http://test/mcp",
        "timeout_seconds": 45,
        "authorization": "API-Key xyz",
        "reconnect_delay_ms": 500
    }"#;
    let config: SseTransportConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.sse_url, "http://test/sse");
    assert_eq!(config.http_url, "http://test/mcp");
    assert_eq!(config.timeout_seconds, 45);
    assert_eq!(config.authorization, Some("API-Key xyz".to_string()));
    assert_eq!(config.reconnect_delay_ms, 500);
}

#[test]
fn test_sse_config_without_authorization() {
    let json = r#"{
        "sse_url": "http://test/sse",
        "http_url": "http://test/mcp"
    }"#;
    let config: SseTransportConfig = serde_json::from_str(json).unwrap();
    assert!(config.authorization.is_none());
}

#[test]
fn test_sse_config_clone() {
    let config = create_test_config();
    let cloned = config.clone();
    assert_eq!(cloned.sse_url, config.sse_url);
    assert_eq!(cloned.http_url, config.http_url);
    assert_eq!(cloned.timeout_seconds, config.timeout_seconds);
}

#[test]
fn test_sse_config_debug() {
    let config = create_test_config();
    let debug = format!("{:?}", config);
    assert!(debug.contains("SseTransportConfig"));
    assert!(debug.contains("localhost"));
}

#[test]
fn test_default_timeout() {
    assert_eq!(default_timeout(), 30);
}

#[test]
fn test_default_reconnect_delay() {
    assert_eq!(default_reconnect_delay(), 1000);
}

#[test]
fn test_request_id_to_string_number() {
    let id = RequestId::Number(42);
    assert_eq!(SseTransport::request_id_to_string(&id), "42");
}

#[test]
fn test_request_id_to_string_string() {
    let id = RequestId::String("req-123".to_string());
    assert_eq!(SseTransport::request_id_to_string(&id), "req-123");
}

#[test]
fn test_request_id_to_string_zero() {
    let id = RequestId::Number(0);
    assert_eq!(SseTransport::request_id_to_string(&id), "0");
}

#[test]
fn test_request_id_to_string_empty_string() {
    let id = RequestId::String(String::new());
    assert_eq!(SseTransport::request_id_to_string(&id), "");
}
