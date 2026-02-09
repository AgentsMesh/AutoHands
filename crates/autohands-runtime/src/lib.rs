//! # AutoHands Runtime
//!
//! Agent execution runtime implementing the agentic loop.

pub mod agent_loop;
pub mod context_builder;
pub mod history;
pub mod retry;
pub mod runtime;
pub mod session;
pub mod session_store;
pub mod streaming;
pub mod summarizer;
pub mod transcript;

pub use agent_loop::{AgentLoop, AgentLoopConfig};
pub use context_builder::{ContextBuilder, ContextConfig};
pub use history::HistoryManager;
pub use retry::{is_retryable, RetryConfig, RetryProvider};
pub use runtime::{AgentRuntime, AgentRuntimeConfig};
pub use session::{Session, SessionManager};
pub use session_store::{
    FileSessionStore, MemorySessionStore, SessionCleaner, SessionStore, SessionStoreError,
};
pub use streaming::{AgentEventStream, ChunkProcessor, StreamEvent, StreamingAgentLoop};
pub use summarizer::{
    ConversationSummary, HistoryCompressor, LLMSummarizer, Summarizer, SummarizerConfig,
};
pub use transcript::{TranscriptEntry, TranscriptManager, TranscriptWriter};
