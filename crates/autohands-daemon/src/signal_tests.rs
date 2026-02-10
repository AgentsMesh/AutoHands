
    use super::*;

    #[test]
    fn test_signal_display() {
        assert_eq!(DaemonSignal::Shutdown.to_string(), "SHUTDOWN");
        assert_eq!(DaemonSignal::Reload.to_string(), "RELOAD");
        assert_eq!(DaemonSignal::Terminate.to_string(), "TERMINATE");
    }

    #[test]
    fn test_signal_handler_new() {
        let handler = SignalHandler::new();
        assert!(!handler.is_shutdown_requested());
        assert!(!handler.is_reload_requested());
    }

    #[test]
    fn test_request_shutdown() {
        let handler = SignalHandler::new();
        handler.request_shutdown();
        assert!(handler.is_shutdown_requested());
    }

    #[test]
    fn test_request_reload() {
        let handler = SignalHandler::new();
        handler.request_reload();
        assert!(handler.is_reload_requested());
    }

    #[test]
    fn test_clear_reload_flag() {
        let handler = SignalHandler::new();
        handler.request_reload();
        assert!(handler.is_reload_requested());

        handler.clear_reload_flag();
        assert!(!handler.is_reload_requested());
    }

    #[tokio::test]
    async fn test_signal_subscription() {
        let handler = SignalHandler::new();
        let mut rx = handler.subscribe();

        handler.send(DaemonSignal::Shutdown);

        let received = rx.recv().await.unwrap();
        assert_eq!(received, DaemonSignal::Shutdown);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let handler = SignalHandler::new();
        let mut rx1 = handler.subscribe();
        let mut rx2 = handler.subscribe();

        handler.send(DaemonSignal::Reload);

        assert_eq!(rx1.recv().await.unwrap(), DaemonSignal::Reload);
        assert_eq!(rx2.recv().await.unwrap(), DaemonSignal::Reload);
    }

    #[test]
    fn test_signal_eq() {
        assert_eq!(DaemonSignal::Shutdown, DaemonSignal::Shutdown);
        assert_ne!(DaemonSignal::Shutdown, DaemonSignal::Reload);
    }

    #[test]
    fn test_signal_clone() {
        let signal = DaemonSignal::Shutdown;
        let cloned = signal;
        assert_eq!(signal, cloned);
    }

    #[test]
    fn test_handler_clone() {
        let handler = SignalHandler::new();
        let cloned = handler.clone();

        handler.request_shutdown();
        // Cloned handler shares the same state
        assert!(cloned.is_shutdown_requested());
    }
