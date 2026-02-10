use super::*;
use std::path::PathBuf;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

#[test]
fn test_tool_definition() {
    let tool = WebFetchTool::new();
    assert_eq!(tool.definition().id, "web_fetch");
    assert_eq!(tool.definition().risk_level, RiskLevel::Medium);
}

#[test]
fn test_tool_default() {
    let tool = WebFetchTool::default();
    assert_eq!(tool.definition().id, "web_fetch");
}

#[test]
fn test_default_method() {
    assert_eq!(default_method(), "GET");
}

#[test]
fn test_default_timeout() {
    assert_eq!(default_timeout(), 30);
}

#[test]
fn test_fetch_params_parsing() {
    let json = serde_json::json!({
        "url": "https://example.com"
    });
    let params: FetchParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.url, "https://example.com");
    assert_eq!(params.method, "GET");
    assert_eq!(params.timeout, 30);
    assert!(params.body.is_none());
    assert!(params.headers.is_empty());
}

#[test]
fn test_fetch_params_full() {
    let json = serde_json::json!({
        "url": "https://api.example.com",
        "method": "POST",
        "body": "{\"key\": \"value\"}",
        "headers": {"Content-Type": "application/json"},
        "timeout": 60
    });
    let params: FetchParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.method, "POST");
    assert_eq!(params.body, Some("{\"key\": \"value\"}".to_string()));
    assert_eq!(params.timeout, 60);
    assert_eq!(params.headers.get("Content-Type"), Some(&"application/json".to_string()));
}

#[test]
fn test_fetch_result_serialize() {
    let result = FetchResult {
        status: 200,
        headers: std::collections::HashMap::new(),
        body: "test body".to_string(),
        url: "https://example.com".to_string(),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("200"));
    assert!(json.contains("test body"));
    assert!(json.contains("example.com"));
}

#[tokio::test]
async fn test_fetch_get() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Hello, World!"))
        .mount(&mock_server)
        .await;

    let tool = WebFetchTool::new();
    let ctx = ToolContext::new("test", PathBuf::from("."));
    let params = serde_json::json!({
        "url": format!("{}/test", mock_server.uri())
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("200"));
    assert!(result.content.contains("Hello, World!"));
}

#[tokio::test]
async fn test_fetch_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .respond_with(ResponseTemplate::new(201).set_body_string("{\"id\": 1}"))
        .mount(&mock_server)
        .await;

    let tool = WebFetchTool::new();
    let ctx = ToolContext::new("test", PathBuf::from("."));
    let params = serde_json::json!({
        "url": format!("{}/api", mock_server.uri()),
        "method": "POST",
        "body": "{\"name\": \"test\"}"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("201"));
}

#[tokio::test]
async fn test_fetch_put() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/resource/1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let tool = WebFetchTool::new();
    let ctx = ToolContext::new("test", PathBuf::from("."));
    let params = serde_json::json!({
        "url": format!("{}/resource/1", mock_server.uri()),
        "method": "PUT",
        "body": "{\"updated\": true}"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.success);
}

#[tokio::test]
async fn test_fetch_delete() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/resource/1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let tool = WebFetchTool::new();
    let ctx = ToolContext::new("test", PathBuf::from("."));
    let params = serde_json::json!({
        "url": format!("{}/resource/1", mock_server.uri()),
        "method": "DELETE"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("204"));
}

#[tokio::test]
async fn test_fetch_head() {
    let mock_server = MockServer::start().await;

    Mock::given(method("HEAD"))
        .and(path("/info"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let tool = WebFetchTool::new();
    let ctx = ToolContext::new("test", PathBuf::from("."));
    let params = serde_json::json!({
        "url": format!("{}/info", mock_server.uri()),
        "method": "HEAD"
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("200"));
}

#[tokio::test]
async fn test_fetch_invalid_url() {
    let tool = WebFetchTool::new();
    let ctx = ToolContext::new("test", PathBuf::from("."));
    let params = serde_json::json!({
        "url": "not-a-url"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_fetch_unsupported_method() {
    let tool = WebFetchTool::new();
    let ctx = ToolContext::new("test", PathBuf::from("."));
    let params = serde_json::json!({
        "url": "https://example.com",
        "method": "PATCH"
    });

    let result = tool.execute(params, ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_fetch_with_headers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/auth"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let tool = WebFetchTool::new();
    let ctx = ToolContext::new("test", PathBuf::from("."));
    let params = serde_json::json!({
        "url": format!("{}/auth", mock_server.uri()),
        "headers": {
            "Authorization": "Bearer token123"
        }
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("200"));
}

#[tokio::test]
async fn test_fetch_404() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/notfound"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&mock_server)
        .await;

    let tool = WebFetchTool::new();
    let ctx = ToolContext::new("test", PathBuf::from("."));
    let params = serde_json::json!({
        "url": format!("{}/notfound", mock_server.uri())
    });

    let result = tool.execute(params, ctx).await.unwrap();
    assert!(result.content.contains("404"));
}
