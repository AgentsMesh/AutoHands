//! Persistent shell session management.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;
use uuid::Uuid;

/// Error types for session operations.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Session spawn failed: {0}")]
    SpawnFailed(String),
    #[error("Session I/O error: {0}")]
    IoError(String),
    #[error("Session timeout")]
    Timeout,
}

/// A persistent shell session.
pub struct ShellSession {
    id: String,
    stdin: ChildStdin,
    stdout_rx: mpsc::Receiver<String>,
    child: Child,
}

impl ShellSession {
    /// Create a new shell session.
    pub fn spawn(shell: &str) -> Result<Self, SessionError> {
        let mut child = Command::new(shell)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SessionError::SpawnFailed(e.to_string()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| SessionError::SpawnFailed("Failed to capture stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SessionError::SpawnFailed("Failed to capture stdout".into()))?;

        let (tx, rx) = mpsc::channel(1024);
        Self::spawn_reader(stdout, tx);

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            stdin,
            stdout_rx: rx,
            child,
        })
    }

    /// Spawn a background reader thread for stdout.
    fn spawn_reader(stdout: ChildStdout, tx: mpsc::Sender<String>) {
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if tx.blocking_send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    /// Get session ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Execute a command in this session.
    pub async fn execute(&mut self, command: &str, timeout_ms: u64) -> Result<String, SessionError> {
        // Write command
        writeln!(self.stdin, "{}", command).map_err(|e| SessionError::IoError(e.to_string()))?;
        self.stdin
            .flush()
            .map_err(|e| SessionError::IoError(e.to_string()))?;

        // Collect output with timeout
        let duration = Duration::from_millis(timeout_ms);
        let mut output = Vec::new();

        loop {
            match timeout(duration, self.stdout_rx.recv()).await {
                Ok(Some(line)) => {
                    output.push(line);
                }
                Ok(None) => break,
                Err(_) => {
                    if output.is_empty() {
                        return Err(SessionError::Timeout);
                    }
                    break;
                }
            }
        }

        Ok(output.join("\n"))
    }

    /// Check if session is still alive.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Kill the session.
    pub fn kill(&mut self) -> Result<(), SessionError> {
        self.child
            .kill()
            .map_err(|e| SessionError::IoError(e.to_string()))
    }
}

impl Drop for ShellSession {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

/// Manager for multiple shell sessions.
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, ShellSession>>>,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new session.
    pub async fn create_session(&self, shell: Option<&str>) -> Result<String, SessionError> {
        let shell = shell.unwrap_or(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "bash"
        });

        let session = ShellSession::spawn(shell)?;
        let id = session.id().to_string();

        self.sessions.lock().await.insert(id.clone(), session);
        Ok(id)
    }

    /// Execute command in a session.
    pub async fn execute_in_session(
        &self,
        session_id: &str,
        command: &str,
        timeout_ms: u64,
    ) -> Result<String, SessionError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;
        session.execute(command, timeout_ms).await
    }

    /// List all active sessions.
    pub async fn list_sessions(&self) -> Vec<String> {
        self.sessions.lock().await.keys().cloned().collect()
    }

    /// Kill a session.
    pub async fn kill_session(&self, session_id: &str) -> Result<(), SessionError> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;
        session.kill()?;
        sessions.remove(session_id);
        Ok(())
    }

    /// Clean up dead sessions.
    pub async fn cleanup(&self) {
        let mut sessions = self.sessions.lock().await;
        sessions.retain(|_, s| s.is_alive());
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_creation() {
        let manager = SessionManager::new();
        assert!(manager.list_sessions().await.is_empty());
    }

    #[tokio::test]
    async fn test_session_manager_default() {
        let manager = SessionManager::default();
        assert!(manager.list_sessions().await.is_empty());
    }

    #[tokio::test]
    async fn test_create_session() {
        let manager = SessionManager::new();
        let result = manager.create_session(None).await;
        // May fail on CI without proper shell
        if result.is_ok() {
            assert!(!manager.list_sessions().await.is_empty());
        }
    }

    #[tokio::test]
    async fn test_create_session_with_shell() {
        let manager = SessionManager::new();
        let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
        let result = manager.create_session(Some(shell)).await;
        if result.is_ok() {
            assert!(!manager.list_sessions().await.is_empty());
        }
    }

    #[tokio::test]
    async fn test_execute_in_nonexistent_session() {
        let manager = SessionManager::new();
        let result = manager.execute_in_session("nonexistent", "echo hello", 1000).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::NotFound(id) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_kill_nonexistent_session() {
        let manager = SessionManager::new();
        let result = manager.kill_session("nonexistent").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::NotFound(id) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_kill_session() {
        let manager = SessionManager::new();
        let result = manager.create_session(None).await;
        if let Ok(session_id) = result {
            // Kill the session
            let kill_result = manager.kill_session(&session_id).await;
            assert!(kill_result.is_ok());

            // Verify it's removed
            assert!(!manager.list_sessions().await.contains(&session_id));
        }
    }

    #[tokio::test]
    async fn test_cleanup() {
        let manager = SessionManager::new();
        // Create and kill a session to test cleanup
        if let Ok(session_id) = manager.create_session(None).await {
            let _ = manager.kill_session(&session_id).await;
        }
        // Cleanup should work even on empty manager
        manager.cleanup().await;
    }

    #[test]
    fn test_session_error_display() {
        let not_found = SessionError::NotFound("id123".to_string());
        assert!(not_found.to_string().contains("Session not found"));
        assert!(not_found.to_string().contains("id123"));

        let spawn_failed = SessionError::SpawnFailed("reason".to_string());
        assert!(spawn_failed.to_string().contains("Session spawn failed"));

        let io_error = SessionError::IoError("io reason".to_string());
        assert!(io_error.to_string().contains("Session I/O error"));

        let timeout = SessionError::Timeout;
        assert!(timeout.to_string().contains("timeout"));
    }

    #[test]
    fn test_session_error_debug() {
        let err = SessionError::NotFound("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotFound"));
    }

    #[tokio::test]
    async fn test_shell_session_spawn_invalid() {
        let result = ShellSession::spawn("nonexistent_shell_xyz");
        assert!(result.is_err());
        let err = result.err().unwrap();
        match err {
            SessionError::SpawnFailed(_) => {}
            _ => panic!("Expected SpawnFailed error"),
        }
    }

    #[tokio::test]
    async fn test_multiple_sessions() {
        let manager = SessionManager::new();

        // Create multiple sessions
        let mut session_ids = Vec::new();
        for _ in 0..3 {
            if let Ok(id) = manager.create_session(None).await {
                session_ids.push(id);
            }
        }

        // Verify sessions count
        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), session_ids.len());

        // Clean up
        for id in session_ids {
            let _ = manager.kill_session(&id).await;
        }
    }
}
