//! System signal integration with RunLoop.
//!
//! Provides a Source1 for handling system signals (SIGTERM, SIGINT, SIGHUP).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::error::RunLoopResult;
use crate::task::{Task, TaskPriority, TaskSource};
use crate::mode::RunLoopMode;
use crate::source::{PortMessage, Source1, Source1Receiver};

/// Signal event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalEvent {
    /// Shutdown signal (SIGTERM, SIGINT).
    Shutdown,
    /// Reload signal (SIGHUP).
    Reload,
    /// User signal 1 (SIGUSR1).
    User1,
    /// User signal 2 (SIGUSR2).
    User2,
}

impl std::fmt::Display for SignalEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalEvent::Shutdown => write!(f, "shutdown"),
            SignalEvent::Reload => write!(f, "reload"),
            SignalEvent::User1 => write!(f, "user1"),
            SignalEvent::User2 => write!(f, "user2"),
        }
    }
}

/// Signal Source1.
///
/// Receives system signals and produces RunLoop events.
pub struct SignalSource1 {
    id: String,
    cancelled: AtomicBool,
    modes: Vec<RunLoopMode>,
}

impl SignalSource1 {
    /// Create a new signal source.
    pub fn new() -> Self {
        Self {
            id: "signal".to_string(),
            cancelled: AtomicBool::new(false),
            modes: vec![RunLoopMode::Common],
        }
    }

    /// Create a Source1Receiver and start listening for signals.
    ///
    /// Note: This only works on Unix systems.
    #[cfg(unix)]
    pub fn create_receiver_with_handlers(self) -> (Source1Receiver, SignalSender) {
        let (tx, rx) = mpsc::channel(16);
        let source = Arc::new(self);

        // Create signal sender
        let signal_sender = SignalSender { sender: tx.clone() };

        // Start signal handlers
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            Self::handle_signals(tx_clone).await;
        });

        (Source1Receiver::new(source, rx), signal_sender)
    }

    /// Create a receiver without OS signal handlers (for testing).
    pub fn create_receiver(self) -> (Source1Receiver, SignalSender) {
        let (tx, rx) = mpsc::channel(16);
        let source = Arc::new(self);
        let signal_sender = SignalSender { sender: tx };
        (Source1Receiver::new(source, rx), signal_sender)
    }

    /// Handle OS signals (Unix only).
    #[cfg(unix)]
    async fn handle_signals(tx: mpsc::Sender<PortMessage>) {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to create SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to create SIGINT handler");
        let mut sighup = signal(SignalKind::hangup()).expect("Failed to create SIGHUP handler");

        loop {
            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM");
                    let _ = tx.send(Self::create_message(SignalEvent::Shutdown)).await;
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT");
                    let _ = tx.send(Self::create_message(SignalEvent::Shutdown)).await;
                }
                _ = sighup.recv() => {
                    info!("Received SIGHUP");
                    let _ = tx.send(Self::create_message(SignalEvent::Reload)).await;
                }
            }
        }
    }

    /// Create a PortMessage from a SignalEvent.
    pub fn create_message(signal: SignalEvent) -> PortMessage {
        PortMessage::new(
            "signal",
            json!({
                "signal": signal.to_string(),
            }),
        )
    }
}

impl Default for SignalSource1 {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Source1 for SignalSource1 {
    fn id(&self) -> &str {
        &self.id
    }

    async fn handle(&self, msg: PortMessage) -> RunLoopResult<Vec<Task>> {
        let signal_str = msg.payload["signal"].as_str().unwrap_or("unknown");

        let signal = match signal_str {
            "shutdown" => SignalEvent::Shutdown,
            "reload" => SignalEvent::Reload,
            "user1" => SignalEvent::User1,
            "user2" => SignalEvent::User2,
            _ => {
                debug!("Unknown signal: {}", signal_str);
                return Ok(Vec::new());
            }
        };

        info!("Processing signal: {:?}", signal);

        let event = match signal {
            SignalEvent::Shutdown => {
                Task::new("system:shutdown", json!({ "signal": "shutdown" }))
                    .with_source(TaskSource::System)
                    .with_priority(TaskPriority::System)
            }
            SignalEvent::Reload => {
                Task::new("system:reload", json!({ "signal": "reload" }))
                    .with_source(TaskSource::System)
                    .with_priority(TaskPriority::High)
            }
            SignalEvent::User1 => {
                Task::new("system:user1", json!({ "signal": "user1" }))
                    .with_source(TaskSource::System)
                    .with_priority(TaskPriority::Normal)
            }
            SignalEvent::User2 => {
                Task::new("system:user2", json!({ "signal": "user2" }))
                    .with_source(TaskSource::System)
                    .with_priority(TaskPriority::Normal)
            }
        };

        Ok(vec![event])
    }

    fn modes(&self) -> &[RunLoopMode] {
        &self.modes
    }

    fn is_valid(&self) -> bool {
        !self.cancelled.load(Ordering::SeqCst)
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

/// Signal sender for programmatic signal sending.
#[derive(Clone)]
pub struct SignalSender {
    sender: mpsc::Sender<PortMessage>,
}

impl SignalSender {
    /// Send a signal event.
    pub async fn send(&self, signal: SignalEvent) -> Result<(), mpsc::error::SendError<PortMessage>> {
        self.sender.send(SignalSource1::create_message(signal)).await
    }

    /// Send a shutdown signal.
    pub async fn shutdown(&self) -> Result<(), mpsc::error::SendError<PortMessage>> {
        self.send(SignalEvent::Shutdown).await
    }

    /// Send a reload signal.
    pub async fn reload(&self) -> Result<(), mpsc::error::SendError<PortMessage>> {
        self.send(SignalEvent::Reload).await
    }
}

#[cfg(test)]
#[path = "signal_tests.rs"]
mod tests;
