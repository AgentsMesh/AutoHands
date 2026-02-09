//! Channel errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChannelError {
    #[error("Channel not found: {0}")]
    NotFound(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Channel disconnected")]
    Disconnected,

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Rate limited: retry after {retry_after_seconds} seconds")]
    RateLimited { retry_after_seconds: u64 },

    #[error("Message too large: {size} bytes, max {max} bytes")]
    MessageTooLarge { size: usize, max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = ChannelError::NotFound("channel-1".to_string());
        let display = err.to_string();
        assert!(display.contains("not found"));
        assert!(display.contains("channel-1"));
    }

    #[test]
    fn test_connection_failed_error() {
        let err = ChannelError::ConnectionFailed("timeout".to_string());
        let display = err.to_string();
        assert!(display.contains("Connection failed"));
        assert!(display.contains("timeout"));
    }

    #[test]
    fn test_send_failed_error() {
        let err = ChannelError::SendFailed("buffer full".to_string());
        let display = err.to_string();
        assert!(display.contains("Send failed"));
        assert!(display.contains("buffer full"));
    }

    #[test]
    fn test_receive_failed_error() {
        let err = ChannelError::ReceiveFailed("closed".to_string());
        let display = err.to_string();
        assert!(display.contains("Receive failed"));
        assert!(display.contains("closed"));
    }

    #[test]
    fn test_disconnected_error() {
        let err = ChannelError::Disconnected;
        assert!(err.to_string().contains("disconnected"));
    }

    #[test]
    fn test_authentication_failed_error() {
        let err = ChannelError::AuthenticationFailed("invalid token".to_string());
        let display = err.to_string();
        assert!(display.contains("Authentication failed"));
        assert!(display.contains("invalid token"));
    }

    #[test]
    fn test_rate_limited_error() {
        let err = ChannelError::RateLimited {
            retry_after_seconds: 30,
        };
        let display = err.to_string();
        assert!(display.contains("Rate limited"));
        assert!(display.contains("30"));
    }

    #[test]
    fn test_message_too_large_error() {
        let err = ChannelError::MessageTooLarge {
            size: 10000,
            max: 8192,
        };
        let display = err.to_string();
        assert!(display.contains("too large"));
        assert!(display.contains("10000"));
        assert!(display.contains("8192"));
    }

    #[test]
    fn test_error_debug() {
        let err = ChannelError::Disconnected;
        let debug = format!("{:?}", err);
        assert!(debug.contains("Disconnected"));
    }

    #[test]
    fn test_all_error_variants() {
        let errors: Vec<ChannelError> = vec![
            ChannelError::NotFound("a".to_string()),
            ChannelError::ConnectionFailed("b".to_string()),
            ChannelError::SendFailed("c".to_string()),
            ChannelError::ReceiveFailed("d".to_string()),
            ChannelError::Disconnected,
            ChannelError::AuthenticationFailed("e".to_string()),
            ChannelError::RateLimited {
                retry_after_seconds: 60,
            },
            ChannelError::MessageTooLarge { size: 100, max: 50 },
        ];

        for err in errors {
            let display = err.to_string();
            assert!(!display.is_empty());
        }
    }
}
