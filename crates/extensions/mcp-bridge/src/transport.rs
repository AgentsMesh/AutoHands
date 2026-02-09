//! Transport layer for MCP communication.

use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::protocol::{McpRequest, McpResponse};

/// Transport trait for MCP communication.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a request and receive a response.
    async fn send(&self, request: McpRequest) -> Result<McpResponse, TransportError>;

    /// Close the transport.
    async fn close(&self) -> Result<(), TransportError>;
}

/// Transport errors.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Process error: {0}")]
    Process(String),

    #[error("Connection closed")]
    Closed,
}

/// Stdio transport for subprocess MCP servers.
pub struct StdioTransport {
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<tokio::process::ChildStdin>>,
    stdout: Mutex<Option<BufReader<tokio::process::ChildStdout>>>,
}

impl StdioTransport {
    /// Create a new stdio transport by spawning a process.
    pub async fn spawn(command: &str, args: &[&str]) -> Result<Self, TransportError> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take()
            .ok_or_else(|| TransportError::Process("Failed to capture stdin".to_string()))?;

        let stdout = child.stdout.take()
            .ok_or_else(|| TransportError::Process("Failed to capture stdout".to_string()))?;

        Ok(Self {
            child: Mutex::new(Some(child)),
            stdin: Mutex::new(Some(stdin)),
            stdout: Mutex::new(Some(BufReader::new(stdout))),
        })
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&self, request: McpRequest) -> Result<McpResponse, TransportError> {
        let mut stdin_guard = self.stdin.lock().await;
        let stdin = stdin_guard.as_mut().ok_or(TransportError::Closed)?;

        let mut stdout_guard = self.stdout.lock().await;
        let stdout = stdout_guard.as_mut().ok_or(TransportError::Closed)?;

        // Write request as JSON line
        let json = serde_json::to_string(&request)?;
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        // Read response line
        let mut line = String::new();
        stdout.read_line(&mut line).await?;

        // Parse response
        let response: McpResponse = serde_json::from_str(&line)?;
        Ok(response)
    }

    async fn close(&self) -> Result<(), TransportError> {
        // Drop stdin/stdout
        *self.stdin.lock().await = None;
        *self.stdout.lock().await = None;

        // Kill child process
        if let Some(mut child) = self.child.lock().await.take() {
            child.kill().await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_error_display() {
        let err = TransportError::Process("test".to_string());
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn test_closed_error() {
        let err = TransportError::Closed;
        assert_eq!(err.to_string(), "Connection closed");
    }

    #[test]
    fn test_transport_error_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = TransportError::Io(io_err);
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_transport_error_json() {
        let json_str = "invalid json {";
        let json_err: Result<serde_json::Value, _> = serde_json::from_str(json_str);
        if let Err(e) = json_err {
            let err = TransportError::Json(e);
            assert!(err.to_string().contains("JSON error"));
        }
    }

    #[test]
    fn test_transport_error_process() {
        let err = TransportError::Process("process crashed".to_string());
        assert_eq!(err.to_string(), "Process error: process crashed");
    }

    #[test]
    fn test_transport_error_debug() {
        let err = TransportError::Closed;
        let debug = format!("{:?}", err);
        assert!(debug.contains("Closed"));

        let err = TransportError::Process("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Process"));
    }

    #[test]
    fn test_transport_error_io_debug() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = TransportError::Io(io_err);
        let debug = format!("{:?}", err);
        assert!(debug.contains("Io"));
    }

    #[test]
    fn test_transport_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broken");
        let transport_err: TransportError = io_err.into();
        assert!(transport_err.to_string().contains("IO error"));
    }

    #[test]
    fn test_transport_error_empty_process_message() {
        let err = TransportError::Process(String::new());
        assert_eq!(err.to_string(), "Process error: ");
    }

    #[test]
    fn test_transport_error_long_process_message() {
        let long_msg = "x".repeat(1000);
        let err = TransportError::Process(long_msg.clone());
        assert!(err.to_string().contains(&long_msg));
    }
}
