    use super::*;

    #[test]
    fn test_signal_event_display() {
        assert_eq!(SignalEvent::Shutdown.to_string(), "shutdown");
        assert_eq!(SignalEvent::Reload.to_string(), "reload");
        assert_eq!(SignalEvent::User1.to_string(), "user1");
        assert_eq!(SignalEvent::User2.to_string(), "user2");
    }

    #[tokio::test]
    async fn test_signal_source1() {
        let source = SignalSource1::new();
        assert_eq!(source.id(), "signal");
        assert!(source.is_valid());
    }

    #[tokio::test]
    async fn test_signal_source1_handle() {
        let source = SignalSource1::new();

        let msg = SignalSource1::create_message(SignalEvent::Shutdown);
        let events = source.handle(msg).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].task_type, "system:shutdown");
        assert_eq!(events[0].priority, TaskPriority::System);
    }

    #[tokio::test]
    async fn test_signal_source1_reload() {
        let source = SignalSource1::new();

        let msg = SignalSource1::create_message(SignalEvent::Reload);
        let events = source.handle(msg).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].task_type, "system:reload");
        assert_eq!(events[0].priority, TaskPriority::High);
    }

    #[tokio::test]
    async fn test_signal_sender() {
        let source = SignalSource1::new();
        let (receiver, signal_sender) = source.create_receiver();

        signal_sender.shutdown().await.unwrap();

        // The message should be in the receiver's channel
        let msg = receiver.receiver.lock().await.recv().await.unwrap();
        assert_eq!(msg.payload["signal"], "shutdown");
    }

    #[tokio::test]
    async fn test_signal_source1_cancel() {
        let source = SignalSource1::new();
        assert!(source.is_valid());

        source.cancel();
        assert!(!source.is_valid());
    }
