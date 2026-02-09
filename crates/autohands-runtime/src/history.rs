//! Conversation history management.

use std::collections::HashMap;
use std::sync::Arc;

use autohands_protocols::types::Message;
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
}

/// History manager for multiple sessions.
pub struct HistoryManager {
    histories: Arc<RwLock<HashMap<String, ConversationHistory>>>,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            histories: Arc::new(RwLock::new(HashMap::new())),
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
    pub fn push(&self, session_id: &str, message: Message) {
        let mut histories = self.histories.write();
        histories
            .entry(session_id.to_string())
            .or_default()
            .push(message);
    }

    /// Clear history for a session.
    pub fn clear(&self, session_id: &str) {
        if let Some(history) = self.histories.write().get_mut(session_id) {
            history.clear();
        }
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
