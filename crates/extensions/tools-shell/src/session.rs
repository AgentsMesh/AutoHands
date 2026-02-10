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
#[path = "session_tests.rs"]
mod tests;
