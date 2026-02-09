//! WebSocket connection management.

use dashmap::DashMap;
use tokio::sync::mpsc;

use super::message::WsMessage;

/// WebSocket connection manager.
pub struct WsConnectionManager {
    connections: DashMap<String, mpsc::Sender<WsMessage>>,
}

impl WsConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
        }
    }

    pub fn add(&self, id: String, sender: mpsc::Sender<WsMessage>) {
        self.connections.insert(id, sender);
    }

    pub fn remove(&self, id: &str) {
        self.connections.remove(id);
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    pub async fn broadcast(&self, message: WsMessage) {
        for entry in self.connections.iter() {
            let _ = entry.value().send(message.clone()).await;
        }
    }

    pub async fn send_to(&self, id: &str, message: WsMessage) -> bool {
        if let Some(sender) = self.connections.get(id) {
            sender.send(message).await.is_ok()
        } else {
            false
        }
    }
}

impl Default for WsConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_connection_manager_new() {
        let manager = WsConnectionManager::new();
        assert_eq!(manager.connection_count(), 0);
    }

    #[tokio::test]
    async fn test_ws_connection_manager_add_remove() {
        let manager = WsConnectionManager::new();
        let (tx, _rx) = mpsc::channel(10);

        manager.add("conn-1".to_string(), tx);
        assert_eq!(manager.connection_count(), 1);

        manager.remove("conn-1");
        assert_eq!(manager.connection_count(), 0);
    }

    #[tokio::test]
    async fn test_ws_connection_manager_send_to() {
        let manager = WsConnectionManager::new();
        let (tx, mut rx) = mpsc::channel(10);

        manager.add("conn-1".to_string(), tx);

        let msg = WsMessage::Pong { timestamp: 123 };
        let sent = manager.send_to("conn-1", msg).await;
        assert!(sent);

        let received = rx.recv().await.unwrap();
        match received {
            WsMessage::Pong { timestamp } => assert_eq!(timestamp, 123),
            _ => panic!("Wrong message type"),
        }
    }

    #[tokio::test]
    async fn test_ws_connection_manager_send_to_nonexistent() {
        let manager = WsConnectionManager::new();
        let msg = WsMessage::Pong { timestamp: 123 };
        let sent = manager.send_to("nonexistent", msg).await;
        assert!(!sent);
    }

    #[test]
    fn test_ws_connection_manager_default() {
        let manager = WsConnectionManager::default();
        assert_eq!(manager.connection_count(), 0);
    }

    #[tokio::test]
    async fn test_ws_connection_manager_broadcast() {
        let manager = WsConnectionManager::new();
        let (tx1, mut rx1) = mpsc::channel(10);
        let (tx2, mut rx2) = mpsc::channel(10);

        manager.add("conn-1".to_string(), tx1);
        manager.add("conn-2".to_string(), tx2);

        let msg = WsMessage::Ping { timestamp: 999 };
        manager.broadcast(msg).await;

        let received1 = rx1.recv().await.unwrap();
        let received2 = rx2.recv().await.unwrap();

        match received1 {
            WsMessage::Ping { timestamp } => assert_eq!(timestamp, 999),
            _ => panic!("Wrong message type"),
        }
        match received2 {
            WsMessage::Ping { timestamp } => assert_eq!(timestamp, 999),
            _ => panic!("Wrong message type"),
        }
    }

    #[tokio::test]
    async fn test_ws_connection_manager_multiple_connections() {
        let manager = WsConnectionManager::new();
        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);
        let (tx3, _rx3) = mpsc::channel(10);

        manager.add("conn-1".to_string(), tx1);
        manager.add("conn-2".to_string(), tx2);
        manager.add("conn-3".to_string(), tx3);

        assert_eq!(manager.connection_count(), 3);

        manager.remove("conn-2");
        assert_eq!(manager.connection_count(), 2);

        manager.remove("conn-1");
        manager.remove("conn-3");
        assert_eq!(manager.connection_count(), 0);
    }

    #[tokio::test]
    async fn test_ws_connection_manager_remove_nonexistent() {
        let manager = WsConnectionManager::new();
        // Should not panic when removing non-existent connection
        manager.remove("nonexistent");
        assert_eq!(manager.connection_count(), 0);
    }
}
