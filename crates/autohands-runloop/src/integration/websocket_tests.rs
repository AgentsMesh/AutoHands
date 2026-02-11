    use super::*;

    #[test]
    fn test_ws_message_type_chat_serialize() {
        let msg = WsMessageType::Chat {
            session_id: Some("s1".to_string()),
            content: "hello".to_string(),
            stream: false,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "chat");
        assert_eq!(json["content"], "hello");
    }

    #[test]
    fn test_ws_message_type_ping_serialize() {
        let msg = WsMessageType::Ping { timestamp: 12345 };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "ping");
        assert_eq!(json["timestamp"], 12345);
    }

    #[test]
    fn test_ws_message_type_connected_serialize() {
        let msg = WsMessageType::Connected {
            connection_id: "conn-1".to_string(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "connected");
        assert_eq!(json["connection_id"], "conn-1");
    }

    #[test]
    fn test_ws_message_type_error_serialize() {
        let msg = WsMessageType::Error {
            code: "E001".to_string(),
            message: "test error".to_string(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "error");
        assert_eq!(json["code"], "E001");
    }

    #[test]
    fn test_ws_message_type_chat_deserialize() {
        let json = r#"{"type":"chat","content":"hello","stream":true}"#;
        let msg: WsMessageType = serde_json::from_str(json).unwrap();
        match msg {
            WsMessageType::Chat { content, stream, .. } => {
                assert_eq!(content, "hello");
                assert!(stream);
            }
            _ => panic!("Expected Chat variant"),
        }
    }
