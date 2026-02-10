    use super::*;

    #[test]
    fn test_websocket_source1_new() {
        let source = WebSocketSource1::new("test-ws");
        assert_eq!(source.id(), "test-ws");
        assert!(source.is_valid());
    }

    #[test]
    fn test_websocket_source1_default() {
        let source = WebSocketSource1::default();
        assert_eq!(source.id(), "websocket");
    }

    #[tokio::test]
    async fn test_websocket_source1_handle_chat() {
        let source = WebSocketSource1::new("test");

        let msg = WebSocketSource1::create_chat_message(
            Some("session-1".to_string()),
            "Hello, world!",
            "conn-1",
        );

        let events = source.handle(msg).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].task_type, "agent:execute");
        assert_eq!(events[0].source, TaskSource::WebSocket);
    }

    #[tokio::test]
    async fn test_websocket_source1_handle_ping() {
        let source = WebSocketSource1::new("test");

        let msg = PortMessage::new(
            "websocket",
            json!({
                "type": "ping",
                "timestamp": 12345,
            }),
        );

        let events = source.handle(msg).await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_websocket_sender() {
        let source = WebSocketSource1::new("test");
        let (receiver, sender) = source.create_receiver();

        sender
            .send_chat(Some("session-1".to_string()), "Hello", "conn-1")
            .await
            .unwrap();

        // Receive the message
        let msg = receiver.receiver.lock().await.recv().await.unwrap();
        assert_eq!(msg.payload["type"], "chat");
        assert_eq!(msg.payload["content"], "Hello");
    }

    #[test]
    fn test_websocket_source1_cancel() {
        let source = WebSocketSource1::new("test");
        assert!(source.is_valid());

        source.cancel();
        assert!(!source.is_valid());
    }
