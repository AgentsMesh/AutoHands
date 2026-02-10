use super::*;

#[test]
fn test_web_channel_config_default() {
    let config = WebChannelConfig::default();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8080);
}

#[test]
fn test_web_channel_config_serialization() {
    let config = WebChannelConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
    };
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("0.0.0.0"));
    assert!(json.contains("3000"));
}

#[test]
fn test_web_channel_config_deserialization() {
    let json = r#"{"host":"localhost","port":9000}"#;
    let config: WebChannelConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 9000);
}

#[test]
fn test_web_channel_creation() {
    let config = WebChannelConfig::default();
    let channel = WebChannel::new("web", config);
    assert_eq!(channel.id(), "web");
    assert_eq!(channel.address(), "127.0.0.1:8080");
}

#[test]
fn test_web_channel_capabilities() {
    let channel = WebChannel::new("web", WebChannelConfig::default());
    let caps = channel.capabilities();
    assert!(!caps.supports_images);
    assert!(!caps.supports_files);
    assert_eq!(caps.max_message_length, Some(65536));
}

#[test]
fn test_web_channel_state() {
    let state = WebChannelState::new("web");
    assert_eq!(state.id, "web");
    assert!(state.connections.is_empty());
    assert!(!state.started.load(Ordering::SeqCst));
}

#[test]
fn test_web_channel_inbound_receiver() {
    let channel = WebChannel::new("web", WebChannelConfig::default());
    let _rx = channel.inbound();
    // Should not panic
}

#[tokio::test]
async fn test_send_when_not_started() {
    let channel = WebChannel::new("web", WebChannelConfig::default());
    let target = ReplyAddress::new("web", "conn-123");
    let message = OutboundMessage::text("Hello");

    let result = channel.send(&target, message).await;
    assert!(matches!(result, Err(ChannelError::Disconnected)));
}
