//! General purpose agent for AutoHands.
//!
//! Provides an agentic loop that can use tools to accomplish tasks.

mod agent;
mod executor;
mod extension;

pub use agent::GeneralAgent;
pub use extension::GeneralAgentExtension;
