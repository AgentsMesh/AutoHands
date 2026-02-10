use super::*;

// === New type tests ===

#[test]
fn test_reply_address_new() {
    let addr = ReplyAddress::new("web", "conn-123");
    assert_eq!(addr.channel_id, "web");
    assert_eq!(addr.target, "conn-123");
    assert!(addr.thread_id.is_none());
}

#[test]
fn test_reply_address_with_thread() {
    let addr = ReplyAddress::with_thread("telegram", "chat-456", "thread-789");
    assert_eq!(addr.channel_id, "telegram");
    assert_eq!(addr.target, "chat-456");
    assert_eq!(addr.thread_id, Some("thread-789".to_string()));
}

#[test]
fn test_reply_address_serialization() {
    let addr = ReplyAddress::new("web", "conn-123");
    let json = serde_json::to_string(&addr).unwrap();
    assert!(json.contains("web"));
    assert!(json.contains("conn-123"));
}

#[test]
fn test_reply_address_deserialization() {
    let json = r#"{"channel_id":"web","target":"conn-123"}"#;
    let addr: ReplyAddress = serde_json::from_str(json).unwrap();
    assert_eq!(addr.channel_id, "web");
    assert_eq!(addr.target, "conn-123");
}

#[test]
fn test_reply_address_eq() {
    let addr1 = ReplyAddress::new("web", "conn-123");
    let addr2 = ReplyAddress::new("web", "conn-123");
    let addr3 = ReplyAddress::new("web", "conn-456");
    assert_eq!(addr1, addr2);
    assert_ne!(addr1, addr3);
}

#[test]
fn test_inbound_message_new() {
    let reply_to = ReplyAddress::new("web", "conn-123");
    let msg = InboundMessage::new("msg-1", "Hello", reply_to);
    assert_eq!(msg.id, "msg-1");
    assert_eq!(msg.content, "Hello");
    assert_eq!(msg.reply_to.channel_id, "web");
}

#[test]
fn test_inbound_message_with_metadata() {
    let reply_to = ReplyAddress::new("web", "conn-123");
    let msg = InboundMessage::new("msg-1", "Hello", reply_to)
        .with_metadata("user_agent", serde_json::json!("Mozilla/5.0"));
    assert!(msg.metadata.contains_key("user_agent"));
}

#[test]
fn test_outbound_message_text() {
    let msg = OutboundMessage::text("Hello!");
    assert_eq!(msg.content, "Hello!");
    assert!(msg.reply_to_message_id.is_none());
    assert!(msg.metadata.is_empty());
}

#[test]
fn test_outbound_message_reply() {
    let msg = OutboundMessage::reply("Thanks!", "msg-123");
    assert_eq!(msg.content, "Thanks!");
    assert_eq!(msg.reply_to_message_id, Some("msg-123".to_string()));
}

#[test]
fn test_outbound_message_with_metadata() {
    let msg = OutboundMessage::text("Hello")
        .with_metadata("format", serde_json::json!("markdown"));
    assert!(msg.metadata.contains_key("format"));
}

#[test]
fn test_outbound_message_with_attachment() {
    let attachment = Attachment {
        name: "file.txt".to_string(),
        content_type: "text/plain".to_string(),
        url: None,
        data: Some(vec![1, 2, 3]),
    };
    let msg = OutboundMessage::text("See attached").with_attachment(attachment);
    assert_eq!(msg.attachments.len(), 1);
}

// === Legacy type tests (backward compatibility) ===

#[test]
fn test_channel_capabilities_default() {
    let caps = ChannelCapabilities::default();
    assert!(!caps.supports_images);
    assert!(!caps.supports_files);
    assert!(!caps.supports_reactions);
    assert!(!caps.supports_threads);
    assert!(!caps.supports_editing);
    assert!(caps.max_message_length.is_none());
}

#[test]
fn test_channel_capabilities_serialization() {
    let caps = ChannelCapabilities {
        supports_images: true,
        supports_files: true,
        supports_reactions: false,
        supports_threads: true,
        supports_editing: false,
        max_message_length: Some(4096),
    };
    let json = serde_json::to_string(&caps).unwrap();
    assert!(json.contains("supports_images"));
    assert!(json.contains("4096"));
}

#[test]
fn test_channel_capabilities_deserialization() {
    let json = r#"{"supports_images":true,"supports_files":false,"supports_reactions":false,"supports_threads":false,"supports_editing":false,"max_message_length":null}"#;
    let caps: ChannelCapabilities = serde_json::from_str(json).unwrap();
    assert!(caps.supports_images);
    assert!(!caps.supports_files);
}

#[test]
fn test_message_target_serialization() {
    let target = MessageTarget {
        channel_id: "chan-123".to_string(),
        thread_id: Some("thread-456".to_string()),
        user_id: None,
    };
    let json = serde_json::to_string(&target).unwrap();
    assert!(json.contains("chan-123"));
    assert!(json.contains("thread-456"));
    // user_id should be skipped when None
    assert!(!json.contains("user_id"));
}

#[test]
fn test_message_target_deserialization() {
    let json = r#"{"channel_id":"chan-123"}"#;
    let target: MessageTarget = serde_json::from_str(json).unwrap();
    assert_eq!(target.channel_id, "chan-123");
    assert!(target.thread_id.is_none());
    assert!(target.user_id.is_none());
}

#[test]
fn test_reply_address_to_message_target() {
    let addr = ReplyAddress::with_thread("web", "user-123", "thread-456");
    let target: MessageTarget = addr.into();
    assert_eq!(target.channel_id, "web");
    assert_eq!(target.user_id, Some("user-123".to_string()));
    assert_eq!(target.thread_id, Some("thread-456".to_string()));
}

#[test]
fn test_message_target_to_reply_address() {
    let target = MessageTarget {
        channel_id: "telegram".to_string(),
        user_id: Some("chat-123".to_string()),
        thread_id: None,
    };
    let addr: ReplyAddress = target.into();
    assert_eq!(addr.channel_id, "telegram");
    assert_eq!(addr.target, "chat-123");
}

#[test]
fn test_incoming_message_serialization() {
    let msg = IncomingMessage {
        id: "msg-123".to_string(),
        channel_id: "chan-456".to_string(),
        sender_id: "user-789".to_string(),
        content: "Hello, world!".to_string(),
        timestamp: chrono::Utc::now(),
        thread_id: None,
        attachments: Vec::new(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("msg-123"));
    assert!(json.contains("Hello, world!"));
}

#[test]
fn test_incoming_message_deserialization() {
    let json = r#"{"id":"msg-123","channel_id":"chan-456","sender_id":"user-789","content":"Test","timestamp":"2024-01-01T00:00:00Z","attachments":[]}"#;
    let msg: IncomingMessage = serde_json::from_str(json).unwrap();
    assert_eq!(msg.id, "msg-123");
    assert_eq!(msg.content, "Test");
}

#[test]
fn test_inbound_to_incoming_message() {
    let reply_to = ReplyAddress::with_thread("web", "user-123", "thread-456");
    let inbound = InboundMessage::new("msg-1", "Hello", reply_to);
    let incoming: IncomingMessage = inbound.into();
    assert_eq!(incoming.id, "msg-1");
    assert_eq!(incoming.channel_id, "web");
    assert_eq!(incoming.sender_id, "user-123");
    assert_eq!(incoming.thread_id, Some("thread-456".to_string()));
}

#[test]
fn test_outgoing_message_legacy_text() {
    let msg = OutgoingMessage::text("Hello!");
    assert_eq!(msg.content, "Hello!");
    assert!(msg.reply_to.is_none());
    assert!(msg.attachments.is_empty());
}

#[test]
fn test_outgoing_message_serialization() {
    let msg = OutgoingMessage {
        content: "Hello!".to_string(),
        reply_to: Some("msg-123".to_string()),
        attachments: Vec::new(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("Hello!"));
    assert!(json.contains("msg-123"));
}

#[test]
fn test_outbound_to_outgoing_message() {
    let outbound = OutboundMessage::reply("Thanks!", "msg-123");
    let outgoing: OutgoingMessage = outbound.into();
    assert_eq!(outgoing.content, "Thanks!");
    assert_eq!(outgoing.reply_to, Some("msg-123".to_string()));
}

#[test]
fn test_outgoing_to_outbound_message() {
    let outgoing = OutgoingMessage {
        content: "Hello".to_string(),
        reply_to: Some("msg-1".to_string()),
        attachments: Vec::new(),
    };
    let outbound: OutboundMessage = outgoing.into();
    assert_eq!(outbound.content, "Hello");
    assert_eq!(outbound.reply_to_message_id, Some("msg-1".to_string()));
}

#[test]
fn test_sent_message_serialization() {
    let msg = SentMessage {
        id: "msg-123".to_string(),
        timestamp: chrono::Utc::now(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("msg-123"));
}

#[test]
fn test_attachment_serialization() {
    let attachment = Attachment {
        name: "test.txt".to_string(),
        content_type: "text/plain".to_string(),
        url: Some("https://example.com/test.txt".to_string()),
        data: None,
    };
    let json = serde_json::to_string(&attachment).unwrap();
    assert!(json.contains("test.txt"));
    assert!(json.contains("text/plain"));
}

#[test]
fn test_attachment_with_data() {
    let attachment = Attachment {
        name: "data.bin".to_string(),
        content_type: "application/octet-stream".to_string(),
        url: None,
        data: Some(vec![1, 2, 3, 4]),
    };
    let json = serde_json::to_string(&attachment).unwrap();
    assert!(json.contains("data.bin"));
}

#[test]
fn test_channel_capabilities_clone() {
    let caps = ChannelCapabilities {
        supports_images: true,
        max_message_length: Some(1000),
        ..Default::default()
    };
    let cloned = caps.clone();
    assert_eq!(cloned.supports_images, caps.supports_images);
    assert_eq!(cloned.max_message_length, caps.max_message_length);
}

#[test]
fn test_message_target_clone() {
    let target = MessageTarget {
        channel_id: "chan-123".to_string(),
        thread_id: Some("thread-456".to_string()),
        user_id: Some("user-789".to_string()),
    };
    let cloned = target.clone();
    assert_eq!(cloned.channel_id, target.channel_id);
    assert_eq!(cloned.thread_id, target.thread_id);
    assert_eq!(cloned.user_id, target.user_id);
}

#[test]
fn test_incoming_message_with_attachments() {
    let msg = IncomingMessage {
        id: "msg-123".to_string(),
        channel_id: "chan-456".to_string(),
        sender_id: "user-789".to_string(),
        content: "File attached".to_string(),
        timestamp: chrono::Utc::now(),
        thread_id: None,
        attachments: vec![Attachment {
            name: "doc.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            url: Some("https://example.com/doc.pdf".to_string()),
            data: None,
        }],
    };
    assert_eq!(msg.attachments.len(), 1);
    assert_eq!(msg.attachments[0].name, "doc.pdf");
}

#[test]
fn test_channel_capabilities_debug() {
    let caps = ChannelCapabilities::default();
    let debug = format!("{:?}", caps);
    assert!(debug.contains("ChannelCapabilities"));
}

#[test]
fn test_message_target_debug() {
    let target = MessageTarget {
        channel_id: "chan".to_string(),
        thread_id: None,
        user_id: None,
    };
    let debug = format!("{:?}", target);
    assert!(debug.contains("MessageTarget"));
}

#[test]
fn test_attachment_debug() {
    let attachment = Attachment {
        name: "test.txt".to_string(),
        content_type: "text/plain".to_string(),
        url: None,
        data: None,
    };
    let debug = format!("{:?}", attachment);
    assert!(debug.contains("Attachment"));
}
