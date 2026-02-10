
    use super::*;

    #[test]
    fn test_message_content_as_text() {
        let content = MessageContent::Text("Hello".to_string());
        assert_eq!(content.as_text(), "Hello");

        let parts = MessageContent::Parts(vec![
            ContentPart::Text {
                text: "Hello ".to_string(),
            },
            ContentPart::Text {
                text: "World".to_string(),
            },
        ]);
        assert_eq!(parts.as_text(), "Hello World");
    }

    #[test]
    fn test_chat_message_deserialize() {
        let json = r#"{"role": "user", "content": "Hello"}"#;
        let msg: ChatMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, "user");
    }
