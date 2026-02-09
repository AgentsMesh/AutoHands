//! Sub-agent spawning and management tools for AutoHands.
//!
//! Provides tools for dynamically creating, managing, and communicating with
//! child agents at runtime. This enables complex task decomposition and
//! multi-agent collaboration.
//!
//! ## Tools
//!
//! - `agent_spawn` - Create and start a new sub-agent
//! - `agent_status` - Query the status of a spawned agent
//! - `agent_message` - Send a message to a running agent
//! - `agent_terminate` - Terminate a running agent
//!
//! ## Agent Communication
//!
//! Sub-agents communicate through the RunLoop task system. Each agent can:
//! - Receive messages from parent agents
//! - Send results back to parent agents
//! - Submit tasks to the RunLoop

mod extension;
mod manager;
mod tools;

pub use extension::AgentToolsExtension;
pub use manager::{AgentManager, SpawnedAgent, SpawnedAgentStatus};
pub use tools::*;
