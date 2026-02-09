//! Skill tools for AutoHands.
//!
//! Provides tools for dynamic skill discovery, loading, and activation.
//! These tools allow the Agent to:
//!
//! - List available skills and their capabilities
//! - Load a skill's content (expert guidance) on demand
//! - Read files from within a skill's directory
//!
//! ## Usage by Agent
//!
//! When an Agent receives a task, it can:
//! 1. Call `skill_list` to see what skills are available
//! 2. Call `skill_load` to get the expert guidance for a relevant skill
//! 3. Call `skill_read` to access additional resources within the skill
//!
//! This enables truly dynamic skill activation - the Agent decides when
//! and which skills to use based on the task at hand.

mod skill_list;
mod skill_load;
mod skill_read;
mod extension;

pub use skill_list::SkillListTool;
pub use skill_load::SkillLoadTool;
pub use skill_read::SkillReadTool;
pub use extension::SkillToolsExtension;
