use super::*;

use autohands_protocols::channel::{Channel, OutboundMessage, ReplyAddress};
use tokio::sync::mpsc;

#[test]
fn test_api_ws_channel_creation() {
    let channel = ApiWsChannel::new();
    assert_eq!(channel.id(), "api-ws");
    assert_eq!(channel.connection_count(), 0);
}

#[test]
fn test_api_ws_channel_default() {
    let channel = ApiWsChannel::default();
    assert_eq!(channel.id(), "api-ws");
}

#[test]
fn test_api_ws_channel_capabilities() {
    let channel = ApiWsChannel::new();
    let caps = channel.capabilities();
    assert!(!caps.supports_images);
    assert!(!caps.supports_threads);
    assert_eq!(caps.max_message_length, Some(65536));
}

#[test]
fn test_api_ws_channel_register_unregister() {
    let channel = ApiWsChannel::new();
    let (tx, _rx) = mpsc::channel(10);

    channel.register_connection("conn-1".to_string(), tx);
    assert_eq!(channel.connection_count(), 1);

    channel.unregister_connection("conn-1");
    assert_eq!(channel.connection_count(), 0);
}

#[test]
fn test_api_ws_channel_register_multiple() {
    let channel = ApiWsChannel::new();
    let (tx1, _rx1) = mpsc::channel(10);
    let (tx2, _rx2) = mpsc::channel(10);
    let (tx3, _rx3) = mpsc::channel(10);

    channel.register_connection("conn-1".to_string(), tx1);
    channel.register_connection("conn-2".to_string(), tx2);
    channel.register_connection("conn-3".to_string(), tx3);
    assert_eq!(channel.connection_count(), 3);

    channel.unregister_connection("conn-2");
    assert_eq!(channel.connection_count(), 2);
}

#[tokio::test]
async fn test_api_ws_channel_start_stop() {
    let channel = ApiWsChannel::new();

    assert!(channel.start().await.is_ok());
    assert!(channel.started.load(Ordering::SeqCst));

    assert!(channel.stop().await.is_ok());
    assert!(!channel.started.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_api_ws_channel_send_when_stopped() {
    let channel = ApiWsChannel::new();
    let target = ReplyAddress::new("api-ws", "conn-1");
    let message = OutboundMessage::text("Hello");

    let result = channel.send(&target, message).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_api_ws_channel_send_to_nonexistent() {
    let channel = ApiWsChannel::new();
    channel.start().await.unwrap();

    let target = ReplyAddress::new("api-ws", "nonexistent");
    let message = OutboundMessage::text("Hello");

    let result = channel.send(&target, message).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_api_ws_channel_send_success() {
    let channel = ApiWsChannel::new();
    channel.start().await.unwrap();

    let (tx, mut rx) = mpsc::channel(10);
    channel.register_connection("conn-1".to_string(), tx);

    let target = ReplyAddress::new("api-ws", "conn-1");
    let message = OutboundMessage::text("Hello from RunLoop");

    let result = channel.send(&target, message).await;
    assert!(result.is_ok());

    // Verify the WsMessage was received
    let ws_msg = rx.recv().await.unwrap();
    match ws_msg {
        WsMessage::Response {
            session_id,
            content,
            done,
        } => {
            assert_eq!(session_id, "conn-1"); // Falls back to connection_id
            assert_eq!(content, "Hello from RunLoop");
            assert!(done);
        }
        _ => panic!("Expected WsMessage::Response, got {:?}", ws_msg),
    }
}

#[tokio::test]
async fn test_api_ws_channel_send_with_thread_id() {
    let channel = ApiWsChannel::new();
    channel.start().await.unwrap();

    let (tx, mut rx) = mpsc::channel(10);
    channel.register_connection("conn-1".to_string(), tx);

    // Use thread_id as session_id
    let target = ReplyAddress::with_thread("api-ws", "conn-1", "session-abc");
    let message = OutboundMessage::text("Response with session");

    let result = channel.send(&target, message).await;
    assert!(result.is_ok());

    let ws_msg = rx.recv().await.unwrap();
    match ws_msg {
        WsMessage::Response {
            session_id,
            content,
            done,
        } => {
            assert_eq!(session_id, "session-abc");
            assert_eq!(content, "Response with session");
            assert!(done);
        }
        _ => panic!("Expected WsMessage::Response, got {:?}", ws_msg),
    }
}

#[tokio::test]
async fn test_api_ws_channel_inbound() {
    let channel = ApiWsChannel::new();
    let _rx = channel.inbound();
    // Just verify we can subscribe without errors
}

#[tokio::test]
async fn test_api_ws_channel_stop_clears_connections() {
    let channel = ApiWsChannel::new();
    channel.start().await.unwrap();

    let (tx, _rx) = mpsc::channel(10);
    channel.register_connection("conn-1".to_string(), tx);
    assert_eq!(channel.connection_count(), 1);

    channel.stop().await.unwrap();
    assert_eq!(channel.connection_count(), 0);
}

#[test]
fn test_api_ws_channel_unregister_nonexistent() {
    let channel = ApiWsChannel::new();
    // Should not panic
    channel.unregister_connection("nonexistent");
    assert_eq!(channel.connection_count(), 0);
}
