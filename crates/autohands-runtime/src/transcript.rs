//! Session transcript recording in JSONL format.
//!
//! Records all conversation events (messages, tool calls, tool results) to JSONL files
//! similar to Claude Code's session transcripts.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tracing::{debug, error, warn};
use uuid::Uuid;

/// Transcript entry types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TranscriptEntry {
    /// Session started
    SessionStart {
        session_id: String,
        timestamp: DateTime<Utc>,
        cwd: String,
        version: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        task: Option<String>,
    },

    /// User message
    User {
        uuid: String,
        session_id: String,
        timestamp: DateTime<Utc>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_uuid: Option<String>,
        message: TranscriptMessage,
    },

    /// Assistant message
    Assistant {
        uuid: String,
        session_id: String,
        timestamp: DateTime<Utc>,
        parent_uuid: String,
        message: TranscriptMessage,
        #[serde(skip_serializing_if = "Option::is_none")]
        stop_reason: Option<String>,
    },

    /// Tool use request (from assistant)
    ToolUse {
        uuid: String,
        session_id: String,
        timestamp: DateTime<Utc>,
        parent_uuid: String,
        tool_use_id: String,
        tool_name: String,
        tool_input: serde_json::Value,
    },

    /// Tool result (response to tool use)
    ToolResult {
        uuid: String,
        session_id: String,
        timestamp: DateTime<Utc>,
        parent_uuid: String,
        tool_use_id: String,
        tool_name: String,
        result: TranscriptToolResult,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
    },

    /// Session ended
    SessionEnd {
        session_id: String,
        timestamp: DateTime<Utc>,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        total_turns: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
    },
}

/// Message content in transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptMessage {
    pub role: String,
    pub content: serde_json::Value,
}

/// Tool result in transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptToolResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Truncated output indicator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

/// Transcript writer for a single session.
pub struct TranscriptWriter {
    session_id: String,
    file: Mutex<File>,
    last_uuid: Mutex<Option<String>>,
}

impl TranscriptWriter {
    /// Create a new transcript writer.
    pub async fn new(session_id: &str, base_dir: &PathBuf) -> std::io::Result<Self> {
        // Create directory if needed
        tokio::fs::create_dir_all(base_dir).await?;

        let file_path = base_dir.join(format!("{}.jsonl", session_id));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;

        debug!("Created transcript file: {:?}", file_path);

        Ok(Self {
            session_id: session_id.to_string(),
            file: Mutex::new(file),
            last_uuid: Mutex::new(None),
        })
    }

    /// Write an entry to the transcript.
    pub async fn write(&self, entry: &TranscriptEntry) -> std::io::Result<()> {
        let json = serde_json::to_string(entry)?;
        let mut file = self.file.lock().await;
        file.write_all(json.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        Ok(())
    }

    /// Record session start.
    pub async fn record_session_start(&self, task: Option<&str>) -> std::io::Result<String> {
        let uuid = Uuid::new_v4().to_string();
        let entry = TranscriptEntry::SessionStart {
            session_id: self.session_id.clone(),
            timestamp: Utc::now(),
            cwd: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            task: task.map(String::from),
        };
        self.write(&entry).await?;
        *self.last_uuid.lock().await = Some(uuid.clone());
        Ok(uuid)
    }

    /// Record a user message.
    pub async fn record_user_message(&self, content: serde_json::Value) -> std::io::Result<String> {
        let uuid = Uuid::new_v4().to_string();
        let parent_uuid = self.last_uuid.lock().await.clone();

        let entry = TranscriptEntry::User {
            uuid: uuid.clone(),
            session_id: self.session_id.clone(),
            timestamp: Utc::now(),
            parent_uuid,
            message: TranscriptMessage {
                role: "user".to_string(),
                content,
            },
        };
        self.write(&entry).await?;
        *self.last_uuid.lock().await = Some(uuid.clone());
        Ok(uuid)
    }

    /// Record an assistant message.
    pub async fn record_assistant_message(
        &self,
        content: serde_json::Value,
        stop_reason: Option<&str>,
    ) -> std::io::Result<String> {
        let uuid = Uuid::new_v4().to_string();
        let parent_uuid = self.last_uuid.lock().await.clone().unwrap_or_default();

        let entry = TranscriptEntry::Assistant {
            uuid: uuid.clone(),
            session_id: self.session_id.clone(),
            timestamp: Utc::now(),
            parent_uuid,
            message: TranscriptMessage {
                role: "assistant".to_string(),
                content,
            },
            stop_reason: stop_reason.map(String::from),
        };
        self.write(&entry).await?;
        *self.last_uuid.lock().await = Some(uuid.clone());
        Ok(uuid)
    }

    /// Record a tool use request.
    pub async fn record_tool_use(
        &self,
        tool_use_id: &str,
        tool_name: &str,
        tool_input: serde_json::Value,
    ) -> std::io::Result<String> {
        let uuid = Uuid::new_v4().to_string();
        let parent_uuid = self.last_uuid.lock().await.clone().unwrap_or_default();

        let entry = TranscriptEntry::ToolUse {
            uuid: uuid.clone(),
            session_id: self.session_id.clone(),
            timestamp: Utc::now(),
            parent_uuid,
            tool_use_id: tool_use_id.to_string(),
            tool_name: tool_name.to_string(),
            tool_input,
        };
        self.write(&entry).await?;
        *self.last_uuid.lock().await = Some(uuid.clone());
        Ok(uuid)
    }

    /// Record a tool result.
    pub async fn record_tool_result(
        &self,
        tool_use_id: &str,
        tool_name: &str,
        success: bool,
        output: Option<&str>,
        error: Option<&str>,
        duration_ms: Option<u64>,
    ) -> std::io::Result<String> {
        let uuid = Uuid::new_v4().to_string();
        let parent_uuid = self.last_uuid.lock().await.clone().unwrap_or_default();

        // Truncate long outputs
        let (output, truncated) = if let Some(out) = output {
            if out.len() > 50000 {
                (Some(format!("{}... [truncated]", &out[..50000])), Some(true))
            } else {
                (Some(out.to_string()), None)
            }
        } else {
            (None, None)
        };

        let entry = TranscriptEntry::ToolResult {
            uuid: uuid.clone(),
            session_id: self.session_id.clone(),
            timestamp: Utc::now(),
            parent_uuid,
            tool_use_id: tool_use_id.to_string(),
            tool_name: tool_name.to_string(),
            result: TranscriptToolResult {
                success,
                output,
                error: error.map(String::from),
                truncated,
            },
            duration_ms,
        };
        self.write(&entry).await?;
        *self.last_uuid.lock().await = Some(uuid.clone());
        Ok(uuid)
    }

    /// Record session end.
    pub async fn record_session_end(
        &self,
        status: &str,
        error: Option<&str>,
        total_turns: u32,
        duration_ms: Option<u64>,
    ) -> std::io::Result<()> {
        let entry = TranscriptEntry::SessionEnd {
            session_id: self.session_id.clone(),
            timestamp: Utc::now(),
            status: status.to_string(),
            error: error.map(String::from),
            total_turns,
            duration_ms,
        };
        self.write(&entry).await
    }
}

/// Transcript manager for multiple sessions.
pub struct TranscriptManager {
    base_dir: PathBuf,
    writers: Mutex<std::collections::HashMap<String, Arc<TranscriptWriter>>>,
}

impl TranscriptManager {
    /// Create a new transcript manager.
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            writers: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Get or create a transcript writer for a session.
    pub async fn get_writer(&self, session_id: &str) -> std::io::Result<Arc<TranscriptWriter>> {
        let mut writers = self.writers.lock().await;

        if let Some(writer) = writers.get(session_id) {
            return Ok(writer.clone());
        }

        let writer = Arc::new(TranscriptWriter::new(session_id, &self.base_dir).await?);
        writers.insert(session_id.to_string(), writer.clone());
        Ok(writer)
    }

    /// Remove a writer (called when session ends).
    pub async fn remove_writer(&self, session_id: &str) {
        self.writers.lock().await.remove(session_id);
    }

    /// Get the transcript file path for a session.
    pub fn transcript_path(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(format!("{}.jsonl", session_id))
    }

    /// List all transcript files.
    pub async fn list_transcripts(&self) -> std::io::Result<Vec<String>> {
        let mut transcripts = Vec::new();

        if !self.base_dir.exists() {
            return Ok(transcripts);
        }

        let mut entries = tokio::fs::read_dir(&self.base_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(id) = name.strip_suffix(".jsonl") {
                    transcripts.push(id.to_string());
                }
            }
        }

        Ok(transcripts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_transcript_writer_create() {
        let temp_dir = TempDir::new().unwrap();
        let writer = TranscriptWriter::new("test-session", &temp_dir.path().to_path_buf())
            .await
            .unwrap();

        assert_eq!(writer.session_id, "test-session");
    }

    #[tokio::test]
    async fn test_transcript_writer_record_session() {
        let temp_dir = TempDir::new().unwrap();
        let writer = TranscriptWriter::new("test-session", &temp_dir.path().to_path_buf())
            .await
            .unwrap();

        writer.record_session_start(Some("Test task")).await.unwrap();
        writer
            .record_user_message(serde_json::json!("Hello"))
            .await
            .unwrap();
        writer
            .record_assistant_message(serde_json::json!("Hi there!"), Some("end_turn"))
            .await
            .unwrap();
        writer
            .record_session_end("completed", None, 1, Some(1000))
            .await
            .unwrap();

        // Verify file exists and has content
        let file_path = temp_dir.path().join("test-session.jsonl");
        assert!(file_path.exists());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 4);

        // Verify each line is valid JSON
        for line in lines {
            let _: serde_json::Value = serde_json::from_str(line).unwrap();
        }
    }

    #[tokio::test]
    async fn test_transcript_writer_tool_use() {
        let temp_dir = TempDir::new().unwrap();
        let writer = TranscriptWriter::new("test-session", &temp_dir.path().to_path_buf())
            .await
            .unwrap();

        writer.record_session_start(None).await.unwrap();
        writer
            .record_tool_use(
                "tool_123",
                "read_file",
                serde_json::json!({"path": "/tmp/test.txt"}),
            )
            .await
            .unwrap();
        writer
            .record_tool_result(
                "tool_123",
                "read_file",
                true,
                Some("file contents"),
                None,
                Some(50),
            )
            .await
            .unwrap();

        let file_path = temp_dir.path().join("test-session.jsonl");
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3);
    }

    #[tokio::test]
    async fn test_transcript_manager() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TranscriptManager::new(temp_dir.path().to_path_buf());

        let writer1 = manager.get_writer("session-1").await.unwrap();
        let writer2 = manager.get_writer("session-2").await.unwrap();

        writer1.record_session_start(None).await.unwrap();
        writer2.record_session_start(None).await.unwrap();

        let transcripts = manager.list_transcripts().await.unwrap();
        assert_eq!(transcripts.len(), 2);
    }

    #[tokio::test]
    async fn test_transcript_truncation() {
        let temp_dir = TempDir::new().unwrap();
        let writer = TranscriptWriter::new("test-session", &temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // Create a very long output
        let long_output = "x".repeat(100000);

        writer
            .record_tool_result("tool_123", "exec", true, Some(&long_output), None, None)
            .await
            .unwrap();

        let file_path = temp_dir.path().join("test-session.jsonl");
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();

        // Verify output was truncated
        assert!(content.contains("[truncated]"));
        assert!(content.len() < 100000);
    }
}
