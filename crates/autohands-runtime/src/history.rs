//! Conversation history management.

use std::collections::HashMap;
use std::sync::Arc;

use autohands_protocols::types::{Message, MessageRole};
use parking_lot::RwLock;

/// History for a single session.
#[derive(Debug, Clone, Default)]
pub struct ConversationHistory {
    messages: Vec<Message>,
}

impl ConversationHistory {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Remove the oldest `count` non-System messages from the history.
    ///
    /// System messages are always preserved to maintain context integrity.
    pub fn trim_oldest(&mut self, count: usize) {
        let mut removed = 0;
        self.messages.retain(|msg| {
            if removed >= count {
                return true;
            }
            if msg.role == MessageRole::System {
                return true; // Always preserve System messages
            }
            removed += 1;
            false
        });
    }
}

/// Default maximum messages per session before oldest messages are trimmed.
const DEFAULT_MAX_MESSAGES_PER_SESSION: usize = 200;

/// History manager for multiple sessions.
pub struct HistoryManager {
    histories: Arc<RwLock<HashMap<String, ConversationHistory>>>,
    /// Maximum messages per session. Older messages are dropped when exceeded.
    max_messages_per_session: usize,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            histories: Arc::new(RwLock::new(HashMap::new())),
            max_messages_per_session: DEFAULT_MAX_MESSAGES_PER_SESSION,
        }
    }

    /// Create a HistoryManager with a custom per-session message limit.
    pub fn with_max_messages(max_messages: usize) -> Self {
        Self {
            histories: Arc::new(RwLock::new(HashMap::new())),
            max_messages_per_session: max_messages,
        }
    }

    /// Get history for a session.
    pub fn get(&self, session_id: &str) -> ConversationHistory {
        self.histories
            .read()
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Add a message to a session's history.
    ///
    /// If the session exceeds `max_messages_per_session`, the oldest messages
    /// are dropped to stay within the limit.
    pub fn push(&self, session_id: &str, message: Message) {
        let mut histories = self.histories.write();
        let history = histories.entry(session_id.to_string()).or_default();
        history.push(message);

        // Trim oldest messages when limit is exceeded
        if history.len() > self.max_messages_per_session {
            let excess = history.len() - self.max_messages_per_session;
            history.trim_oldest(excess);
        }
    }

    /// Clear history for a session.
    pub fn clear(&self, session_id: &str) {
        if let Some(history) = self.histories.write().get_mut(session_id) {
            history.clear();
        }
    }

    /// Remove a session's history entirely.
    pub fn remove(&self, session_id: &str) {
        self.histories.write().remove(session_id);
    }

    /// Get the number of tracked sessions.
    pub fn session_count(&self) -> usize {
        self.histories.read().len()
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_history_new() {
        let history = ConversationHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_conversation_history_push() {
        let mut history = ConversationHistory::new();
        history.push(Message::user("Hello"));

        assert!(!history.is_empty());
        assert_eq!(history.len(), 1);
        assert_eq!(history.messages()[0].content.text(), "Hello");
    }

    #[test]
    fn test_conversation_history_clear() {
        let mut history = ConversationHistory::new();
        history.push(Message::user("Hello"));
        history.push(Message::assistant("Hi"));

        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn test_history_manager_new() {
        let manager = HistoryManager::new();
        let history = manager.get("session-1");
        assert!(history.is_empty());
    }

    #[test]
    fn test_history_manager_default() {
        let manager = HistoryManager::default();
        assert!(manager.get("any").is_empty());
    }

    #[test]
    fn test_history_manager_push() {
        let manager = HistoryManager::new();
        manager.push("session-1", Message::user("Hello"));
        manager.push("session-1", Message::assistant("Hi"));

        let history = manager.get("session-1");
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_history_manager_multiple_sessions() {
        let manager = HistoryManager::new();
        manager.push("session-1", Message::user("Hello 1"));
        manager.push("session-2", Message::user("Hello 2"));

        assert_eq!(manager.get("session-1").len(), 1);
        assert_eq!(manager.get("session-2").len(), 1);
    }

    #[test]
    fn test_history_manager_clear() {
        let manager = HistoryManager::new();
        manager.push("session-1", Message::user("Hello"));
        manager.push("session-1", Message::assistant("Hi"));

        manager.clear("session-1");
        assert!(manager.get("session-1").is_empty());
    }

    #[test]
    fn test_history_manager_clear_nonexistent() {
        let manager = HistoryManager::new();
        // Should not panic
        manager.clear("nonexistent");
    }
}
