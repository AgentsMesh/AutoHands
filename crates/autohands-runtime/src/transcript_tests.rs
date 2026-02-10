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
