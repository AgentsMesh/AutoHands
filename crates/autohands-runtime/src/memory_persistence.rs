//! Memory persistence and formatting utilities for the agent loop.
//!
//! Free functions that receive parameters instead of `&self`, improving
//! testability and enabling reuse outside `AgentLoop`.

use std::sync::Arc;

use tracing::{debug, info, warn};

use autohands_protocols::memory::{MemoryBackend, MemorySearchResult};
use autohands_protocols::types::Message;

/// Format memory search results as context for the agent.
pub(crate) fn format_memory_context(results: &[MemorySearchResult]) -> String {
    let mut output = String::from(
        "The following are relevant memories from previous conversations:\n\n",
    );
    for (i, result) in results.iter().enumerate() {
        let entry = &result.entry;
        let created = entry
            .created_at
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();
        output.push_str(&format!(
            "[Memory #{}] (type: {}, relevance: {:.2}{})\n{}\n\n",
            i + 1,
            entry.memory_type,
            result.relevance,
            if created.is_empty() {
                String::new()
            } else {
                format!(", date: {}", created)
            },
            entry.content,
        ));
    }
    output
}

/// Extract key information from messages and persist to memory backend.
///
/// `flush_tag` distinguishes the source: `"auto-flush"` (context overflow) vs
/// `"session-end-flush"` (normal/max-turns exit).
pub(crate) async fn flush_memories_to_backend(
    messages: &[Message],
    memory: &Arc<dyn MemoryBackend>,
    flush_tag: &str,
) {
    use autohands_protocols::memory::MemoryEntry;

    // Expanded heuristic patterns -- scan both User and Assistant messages
    let patterns = [
        ("preference", &[
            "prefer", "like to", "always use", "favorite", "default to",
            "my preferred", "i usually",
        ][..]),
        ("decision", &[
            "decided", "will use", "chose", "going with", "let's go with",
            "we agreed", "the plan is",
        ]),
        ("fact", &[
            "is located at", "the password", "the key", "api key", "endpoint is",
            "version is", "my name is", "i work on", "the project is",
        ]),
        ("todo", &[
            "todo", "need to", "don't forget", "remember to", "should do",
            "next time", "for later", "follow up",
        ]),
    ];

    let mut stored_count = 0u32;

    for msg in messages {
        // Scan both User and Assistant messages (skip system/tool)
        if !matches!(
            msg.role,
            autohands_protocols::types::MessageRole::User
                | autohands_protocols::types::MessageRole::Assistant
        ) {
            continue;
        }

        let content_lower = msg.content.text().to_lowercase();
        for (memory_type, keywords) in &patterns {
            if keywords.iter().any(|kw| content_lower.contains(kw)) {
                let content = msg.content.text();
                if content.len() > 10 && content.len() < 2000 {
                    let entry = MemoryEntry::new(content, *memory_type)
                        .with_importance(0.6)
                        .with_tags(vec![flush_tag.to_string()]);
                    match memory.store(entry).await {
                        Ok(id) => {
                            debug!("Memory flush: stored {} entry (id: {})", memory_type, id);
                            stored_count += 1;
                        }
                        Err(e) => {
                            warn!("Memory flush: failed to store entry: {}", e);
                        }
                    }
                }
                break; // One extraction per message
            }
        }
    }

    if stored_count > 0 {
        info!(
            "Memory flush ({}): stored {} entries from {} messages",
            flush_tag,
            stored_count,
            messages.len()
        );
    }
}

/// Store a lightweight session summary into memory for cross-session search.
///
/// Extracts user topics and assistant key responses from in-memory messages,
/// avoiding any dependency on JSONL transcript files.
pub(crate) async fn store_session_summary(
    messages: &[Message],
    session_id: &str,
    memory: &Arc<dyn MemoryBackend>,
) {
    use autohands_protocols::memory::MemoryEntry;

    // Extract user topics: first line of each User message (truncated to 200 chars)
    let user_topics: Vec<String> = messages
        .iter()
        .filter(|m| matches!(m.role, autohands_protocols::types::MessageRole::User))
        .filter_map(|m| {
            let text = m.content.text();
            let first_line = text.lines().next().unwrap_or("").trim();
            if first_line.is_empty() {
                None
            } else if first_line.len() > 200 {
                Some(format!("{}...", &first_line[..floor_char_boundary(first_line, 200)]))
            } else {
                Some(first_line.to_string())
            }
        })
        .collect();

    // Extract assistant key responses: first line of each Assistant message (take up to 5)
    let key_responses: Vec<String> = messages
        .iter()
        .filter(|m| matches!(m.role, autohands_protocols::types::MessageRole::Assistant))
        .filter_map(|m| {
            let text = m.content.text();
            let first_line = text.lines().next().unwrap_or("").trim();
            if first_line.is_empty() {
                None
            } else if first_line.len() > 200 {
                Some(format!("{}...", &first_line[..floor_char_boundary(first_line, 200)]))
            } else {
                Some(first_line.to_string())
            }
        })
        .take(5)
        .collect();

    if user_topics.is_empty() && key_responses.is_empty() {
        return;
    }

    let summary = format!(
        "Session conversation summary:\nUser topics: {}\nKey responses: {}",
        user_topics.join("; "),
        key_responses.join("; "),
    );

    let entry = MemoryEntry::new(summary, "conversation")
        .with_importance(0.4)
        .with_tags(vec![
            "session-summary".to_string(),
            format!("session:{}", session_id),
        ]);

    match memory.store(entry).await {
        Ok(id) => {
            debug!(
                "Session summary stored (id: {}) for session {}",
                id, session_id
            );
        }
        Err(e) => {
            warn!("Failed to store session summary: {}", e);
        }
    }
}

/// Find the nearest char boundary at or before `index` (stable replacement for
/// `str::floor_char_boundary`).
pub(crate) fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}
