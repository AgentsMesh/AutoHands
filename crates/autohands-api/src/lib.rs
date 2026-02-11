//! # AutoHands Interface
//!
//! Unified external interface layer for AutoHands framework.
//!
//! This crate consolidates all external-facing interfaces:
//! - **HTTP**: REST API endpoints for task submission and management
//! - **WebSocket**: Real-time bidirectional communication
//! - **Webhook**: Event-driven trigger system
//! - **Workflow**: Multi-step task orchestration
//! - **Job**: Scheduled task execution via Cron
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────┐
//! │                   autohands-interface (External Interface Layer)       │
//! │  ┌─────────┐  ┌───────────┐  ┌─────────┐  ┌─────────┐  ┌──────────┐  │
//! │  │  HTTP   │  │ WebSocket │  │ Webhook │  │ Workflow│  │   Job    │  │
//! │  │  REST   │  │           │  │         │  │  Engine │  │ Scheduler│  │
//! │  └────┬────┘  └─────┬─────┘  └────┬────┘  └────┬────┘  └────┬─────┘  │
//! │       │             │             │            │             │        │
//! │       └─────────────┴──────┬──────┴────────────┴─────────────┘        │
//! │                            │ RunLoop Bridge                           │
//! └────────────────────────────┼──────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │                         RunLoop (Central Event Hub)                    │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Design Principles
//!
//! 1. **All external requests flow through RunLoop**: HTTP, WebSocket, and Webhook
//!    requests are converted to RunLoop events for unified processing.
//!
//! 2. **Event-driven architecture**: The interface layer acts as an event source
//!    (Source1) for the RunLoop, enabling asynchronous and decoupled processing.
//!
//! 3. **Unified response routing**: Responses flow back through the interface
//!    layer to the appropriate client connection.

pub mod error;
pub mod http;
pub mod job;
pub mod runloop_bridge;
pub mod server;
pub mod state;
pub mod webhook;
pub mod websocket;
pub mod workflow;

// Re-export core types
pub use error::InterfaceError;
pub use http::{
    handlers::{AgentAbortRequest, AgentAbortResponse, AgentRunRequest, AgentRunResponse},
    routes::create_router_with_hybrid_state,
};
pub use runloop_bridge::{
    HybridAppState, RunLoopBridge, RunLoopState, RunLoopTaskRequest, RunLoopTaskResponse,
};
pub use server::{InterfaceConfig, InterfaceServer};
pub use state::AppState;
pub use webhook::{WebhookEvent, WebhookRegistration, WebhookRegistry, WebhookResponse};
pub use websocket::{ApiWsChannel, WsConnectionManager, WsMessage};

// Workflow module exports
pub use workflow::{
    ExecutionContext, ExecutionState, MemoryWorkflowStore, StepResult, StepType, Workflow,
    WorkflowExecution, WorkflowExecutor, WorkflowStep, WorkflowStore,
};

// Job module exports
pub use job::{
    FileJobStore, Job, JobDefinition, JobScheduler, JobStatus, JobStore, MemoryJobStore,
};
