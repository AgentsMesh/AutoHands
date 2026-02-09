//! WebSocket message types.

use serde::{Deserialize, Serialize};

/// WebSocket message types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// Ping/heartbeat.
    Ping { timestamp: i64 },

    /// Pong response.
    Pong { timestamp: i64 },

    /// Chat message from client.
    Chat {
        session_id: Option<String>,
        content: String,
        #[serde(default)]
        stream: bool,
    },

    /// Response from server.
    Response {
        session_id: String,
        content: String,
        #[serde(default)]
        done: bool,
    },

    /// Error message.
    Error { code: String, message: String },

    /// Connection established.
    Connected { connection_id: String },

    /// Stream chunk.
    Chunk {
        session_id: String,
        content: String,
        index: u32,
    },

    /// Agent execution started (sent when task is queued to RunLoop).
    ExecutionStarted {
        session_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
    },

    /// Agent execution progress update.
    ExecutionProgress {
        session_id: String,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
}

impl WsMessage {
    /// Create a new error message.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Create a new response message.
    pub fn response(session_id: impl Into<String>, content: impl Into<String>, done: bool) -> Self {
        Self::Response {
            session_id: session_id.into(),
            content: content.into(),
            done,
        }
    }

    /// Create an execution started message.
    pub fn execution_started(session_id: impl Into<String>, agent_id: Option<String>) -> Self {
        Self::ExecutionStarted {
            session_id: session_id.into(),
            agent_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_message_ping_serialization() {
        let msg = WsMessage::Ping { timestamp: 12345 };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("ping"));
        assert!(json.contains("12345"));
    }

    #[test]
    fn test_ws_message_chat_serialization() {
        let msg = WsMessage::Chat {
            session_id: Some("sess-1".to_string()),
            content: "Hello".to_string(),
            stream: true,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("chat"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_ws_message_response_serialization() {
        let msg = WsMessage::Response {
            session_id: "sess-1".to_string(),
            content: "Hi".to_string(),
            done: true,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("response"));
        assert!(json.contains("done"));
    }

    #[test]
    fn test_ws_message_deserialization() {
        let json = r#"{"type":"ping","timestamp":12345}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();
        match msg {
            WsMessage::Ping { timestamp } => assert_eq!(timestamp, 12345),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_ws_message_error_helper() {
        let msg = WsMessage::error("ERR001", "Something went wrong");
        match msg {
            WsMessage::Error { code, message } => {
                assert_eq!(code, "ERR001");
                assert_eq!(message, "Something went wrong");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_ws_message_execution_started() {
        let msg = WsMessage::execution_started("sess-123", Some("general".to_string()));
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("execution_started"));
        assert!(json.contains("sess-123"));
        assert!(json.contains("general"));
    }
}
