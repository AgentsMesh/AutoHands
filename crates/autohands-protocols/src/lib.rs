//! # AutoHands Protocols
//!
//! Core protocol definitions (traits) for the AutoHands framework.
//! Contains only interface definitions - no implementations.
//!
//! ## Core Traits
//!
//! - [`Extension`] - Base trait for all extensions
//! - [`Tool`] - Trait for tool implementations
//! - [`LLMProvider`] - Trait for LLM provider implementations
//! - [`Channel`] - Trait for message channel implementations
//! - [`MemoryBackend`] - Trait for memory storage implementations
//! - [`Agent`] - Trait for agent implementations
//! - [`SkillLoader`] - Trait for skill loading implementations

pub mod error;
pub mod extension;
pub mod tool;
pub mod provider;
pub mod channel;
pub mod memory;
pub mod agent;
pub mod skill;
pub mod types;

// Re-export core traits
pub use extension::{Extension, ExtensionContext, ExtensionManifest};
pub use tool::{Tool, ToolContext, ToolDefinition, ToolResult};
pub use provider::{CompletionRequest, CompletionResponse, CompletionStream, LLMProvider};
pub use channel::{Channel, IncomingMessage, OutgoingMessage};
pub use memory::{MemoryBackend, MemoryEntry, MemoryQuery};
pub use agent::{Agent, AgentConfig, AgentContext};
pub use skill::{Skill, SkillDefinition, SkillLoader};
pub use error::{
    AgentError, ChannelError, ExtensionError, MemoryError, ProtocolError, ProviderError,
    SkillError, ToolError,
};
pub use types::*;
